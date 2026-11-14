//! Localhost OS IPC for CoreFacade (Spec 024 READY_BASE).
//!
//! Framing: little-endian u32 length prefix + JSON AuthorityRequest/Response.
//! Transport: `interprocess` local sockets (Windows named pipe / Unix domain socket).
//!
//! The full endpoint answers only clients that first send the per-launch token
//! the server writes to `IPC_TOKEN_FILE` in its vault root, so reaching the
//! socket is not enough: a client must also be able to read the vault folder.
//! The Hub's limited endpoint (`serve_connection_limited`) takes no token.

mod framing;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{GenericNamespaced, Listener, ListenerOptions, Stream, ToNsName};
use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, AuthorityResponse, Capability,
};
use medscale_storage::{WriterLock, WriterLockError};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::CoreFacade;

pub use framing::{MAX_FRAME_BYTES, read_frame, write_frame};

/// File in the vault root holding the running server's per-launch token.
pub const IPC_TOKEN_FILE: &str = ".host-ipc-token";

/// Errors from host OS IPC bind/serve/connect.
#[derive(Debug, Error)]
pub enum HostIpcError {
    #[error(transparent)]
    Writer(#[from] WriterLockError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("ipc name: {0}")]
    Name(String),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("frame too large: {0} bytes")]
    FrameTooLarge(usize),
    #[error("server stopped")]
    ServerStopped,
    #[error("ipc token unavailable: start the host for this vault root first")]
    TokenUnavailable,
}

/// Bound localhost IPC host: holds Spec 016 writer lock + Strict CoreFacade.
pub struct HostIpcServer {
    facade: Arc<CoreFacade>,
    _writer: WriterLock,
    listener: Listener,
    endpoint: String,
    vault_root: PathBuf,
    token: Arc<String>,
}

impl HostIpcServer {
    /// Acquire writer lock and listen on a namespaced local-socket endpoint.
    pub fn bind(vault_root: &Path, endpoint: &str) -> Result<Self, HostIpcError> {
        let endpoint = sanitize_endpoint(endpoint)?;
        let writer = WriterLock::try_acquire(vault_root)?;
        let name = endpoint
            .clone()
            .to_ns_name::<GenericNamespaced>()
            .map_err(|e| HostIpcError::Name(e.to_string()))?;
        let listener = ListenerOptions::new()
            .name(name)
            .create_sync()
            .map_err(HostIpcError::Io)?;
        let token: String = medscale_keys::generate_key32()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        write_token(&vault_root.join(IPC_TOKEN_FILE), &token)?;
        Ok(Self {
            facade: Arc::new(CoreFacade::new()),
            _writer: writer,
            listener,
            endpoint,
            vault_root: vault_root.to_path_buf(),
            token: Arc::new(token),
        })
    }

    /// Endpoint name clients must use.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Vault root this host exclusively locks.
    #[must_use]
    pub fn vault_root(&self) -> &Path {
        &self.vault_root
    }

    /// Shared Strict facade (for in-process assertions alongside IPC).
    #[must_use]
    pub fn facade(&self) -> &Arc<CoreFacade> {
        &self.facade
    }

    /// Accept one client and serve request/response frames until EOF.
    pub fn serve_connection(&self) -> Result<(), HostIpcError> {
        let stream = self.listener.accept().map_err(HostIpcError::Io)?;
        serve_stream(&self.facade, stream, &self.token)
    }

    /// Accept one client and serve it, answering only requests whose
    /// capability is in `allowed` (Spec 084: a Hub's device-facing endpoint
    /// serves only Hub bootstrap and sync). Anything else is refused before
    /// it reaches Core, so a sessionless peer cannot use lease-holder reads.
    pub fn serve_connection_limited(&self, allowed: &[Capability]) -> Result<(), HostIpcError> {
        let stream = self.listener.accept().map_err(HostIpcError::Io)?;
        serve_stream_limited(&self.facade, stream, Some(allowed), None)
    }

    /// Move this server into a background accept loop (one connection handler thread each).
    pub fn into_background(self) -> (String, Arc<CoreFacade>, JoinHandle<()>) {
        let endpoint = self.endpoint.clone();
        let facade = Arc::clone(&self.facade);
        let handle = thread::spawn(move || {
            while let Ok(stream) = self.listener.accept() {
                let facade = Arc::clone(&self.facade);
                let token = Arc::clone(&self.token);
                let _ = thread::spawn(move || {
                    let _ = serve_stream(&facade, stream, &token);
                });
            }
        });
        (endpoint, facade, handle)
    }
}

impl Drop for HostIpcServer {
    fn drop(&mut self) {
        let path = self.vault_root.join(IPC_TOKEN_FILE);
        if std::fs::read_to_string(&path).is_ok_and(|t| t == *self.token) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn write_token(path: &Path, token: &str) -> Result<(), HostIpcError> {
    std::fs::write(path, token)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Compares without stopping at the first difference.
fn token_matches(sent: &[u8], expected: &[u8]) -> bool {
    sent.len() == expected.len()
        && sent
            .iter()
            .zip(expected)
            .fold(0_u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

fn serve_stream(facade: &CoreFacade, stream: Stream, token: &str) -> Result<(), HostIpcError> {
    serve_stream_limited(facade, stream, None, Some(token))
}

fn serve_stream_limited(
    facade: &CoreFacade,
    mut stream: Stream,
    allowed: Option<&[Capability]>,
    token: Option<&str>,
) -> Result<(), HostIpcError> {
    // The first frame must be the token; a client without it is dropped
    // without any reply.
    if let Some(token) = token {
        let sent = match read_frame(&mut stream) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(HostIpcError::Io(e)),
        };
        if !token_matches(&sent, token.as_bytes()) {
            return Ok(());
        }
    }
    loop {
        let req_bytes = match read_frame(&mut stream) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(HostIpcError::Io(e)),
        };
        let req: AuthorityRequest = serde_json::from_slice(&req_bytes)?;
        let resp = match allowed {
            Some(allowed) if !allowed.contains(&req.capability) => AuthorityResponse {
                schema_version: req.schema_version,
                request_id: req.request_id,
                result: Err(AuthorityError::Unauthorized),
            },
            _ => facade.dispatch(req),
        };
        let resp_bytes = serde_json::to_vec(&resp)?;
        write_frame(&mut stream, &resp_bytes)?;
    }
}

/// Client connected to a `HostIpcServer` endpoint.
pub struct HostIpcClient {
    stream: Stream,
    endpoint: String,
}

impl HostIpcClient {
    /// Connect to a namespaced local-socket endpoint without a token. Only
    /// the Hub's limited endpoint answers such a client.
    pub fn connect(endpoint: &str) -> Result<Self, HostIpcError> {
        let endpoint = sanitize_endpoint(endpoint)?;
        let stream = connect_with_retry(&endpoint, 50, Duration::from_millis(20))?;
        Ok(Self { stream, endpoint })
    }

    /// Connect to a full endpoint, presenting the token its server wrote in
    /// `vault_root`.
    pub fn connect_authenticated(endpoint: &str, vault_root: &Path) -> Result<Self, HostIpcError> {
        let token = std::fs::read_to_string(vault_root.join(IPC_TOKEN_FILE))
            .map_err(|_| HostIpcError::TokenUnavailable)?;
        let mut client = Self::connect(endpoint)?;
        write_frame(&mut client.stream, token.trim().as_bytes())?;
        Ok(client)
    }

    /// Endpoint this client is connected to.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Send one authority request and await the response.
    pub fn dispatch(&mut self, req: AuthorityRequest) -> Result<AuthorityResponse, HostIpcError> {
        let req_bytes = serde_json::to_vec(&req)?;
        write_frame(&mut self.stream, &req_bytes)?;
        let resp_bytes = read_frame(&mut self.stream).map_err(HostIpcError::Io)?;
        Ok(serde_json::from_slice(&resp_bytes)?)
    }
}

fn connect_with_retry(
    endpoint: &str,
    attempts: u32,
    delay: Duration,
) -> Result<Stream, HostIpcError> {
    let mut last = None;
    for _ in 0..attempts {
        let name = endpoint
            .to_ns_name::<GenericNamespaced>()
            .map_err(|e| HostIpcError::Name(e.to_string()))?;
        match Stream::connect(name) {
            Ok(s) => return Ok(s),
            Err(e) => {
                last = Some(e);
                thread::sleep(delay);
            }
        }
    }
    Err(HostIpcError::Io(last.unwrap_or_else(|| {
        std::io::Error::other("ipc connect failed")
    })))
}

fn sanitize_endpoint(endpoint: &str) -> Result<String, HostIpcError> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty()
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(HostIpcError::Name(
            "endpoint must be non-empty [A-Za-z0-9_-]+".to_owned(),
        ));
    }
    Ok(trimmed.to_owned())
}

/// Derive a stable test/operator endpoint token from a vault root path.
#[must_use]
pub fn endpoint_for_vault_root(vault_root: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(vault_root.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    format!(
        "medscale-ipc-{}",
        digest
            .iter()
            .take(8)
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}
