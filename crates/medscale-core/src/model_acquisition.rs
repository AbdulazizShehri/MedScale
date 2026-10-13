//! Governed, opt-in model acquisition (Spec 103 ACQUIRE).
//!
//! Downloads exactly the files a catalog row needs for local execution
//! (`config.json`, `tokenizer.json` and one ONNX file) from the Hugging Face
//! Hub, only with an explicit per-model consent that names the repository, the
//! ONNX file and the maximum number of bytes. The egress seam is the existing
//! Governed Browse transport (`medscale_network::BrowseTransport`):
//! - HTTPS on port 443 only;
//! - public addresses only;
//! - redirects are never followed by the transport; every hop is re-validated
//!   here against a fixed Hub host allowlist.
//!
//! Integrity follows the ported OpenMed logic in `medscale_pack::hf_snapshot`:
//! - Hub metadata is bound to the catalog row by the reproducibility hash;
//! - files come only from that pinned commit;
//! - each file is checked for exact size and digest before it is kept.
//!
//! Files are written to a staging directory and moved into place only after
//! every file verified. No patient or research input is ever sent: requests
//! carry only public repository paths.
//!
//! Recovery is file-granular:
//! - the staging directory records the pinned commit it belongs to;
//! - after an interrupted acquisition (transport failure, timeout, HTTP error),
//!   only files that already passed size and digest verification remain in
//!   staging, and a retry for the same commit re-verifies and reuses them
//!   instead of downloading them again;
//! - a staged file that no longer verifies is discarded and downloaded again;
//! - staging for a different commit is discarded;
//! - an integrity failure (digest or size mismatch, a refused host) removes
//!   staging entirely.
//!
//! Inside one file, downloads are streamed to `<name>.part` in staging and
//! resume with a `Range` request from the bytes already received (a server
//! that ignores the range sends the whole file again, which replaces the
//! partial file). A completed file is verified before it replaces nothing:
//! the `.part` file becomes the staged file only after its exact size and
//! digest match the pinned commit.

use std::path::{Path, PathBuf};
use std::time::Duration;

use medscale_network::{BrowseTransport, BrowseTransportError, resolve_redirect, validate_url};
use medscale_pack::{
    CatalogRow, HfRepoMetadata, ModelCatalog, SnapshotError, verify_against_catalog, verify_file,
};
use serde::Serialize;

/// Hub hosts an acquisition may contact (API, file resolution and LFS/Xet CDNs).
pub const HUB_HOSTS: &[&str] = &[
    "huggingface.co",
    "cdn-lfs.huggingface.co",
    "cdn-lfs-us-1.huggingface.co",
    "cdn-lfs-eu-1.huggingface.co",
    "cas-bridge.xethub.hf.co",
];
/// Hugging Face CDN host suffixes. Live acquisition (2026-10-10) showed LFS
/// files redirected to regional hosts such as `us.aws.cdn.hf.co`, which an
/// exact list cannot anticipate. A suffix match requires the leading dot, so
/// `cdn.hf.co.example.com` or `evilcdn.hf.co` never match. Every file is
/// still verified against the pinned-commit size and digest.
pub const HUB_HOST_SUFFIXES: &[&str] = &[".cdn.hf.co", ".xethub.hf.co"];

/// True for an allowed Hub host (exact name or an allowed CDN suffix).
pub fn is_hub_host(host: &str) -> bool {
    HUB_HOSTS.contains(&host)
        || HUB_HOST_SUFFIXES
            .iter()
            .any(|suffix| host.ends_with(suffix))
}

const MAX_HOPS: usize = 4;
const METADATA_MAX_BYTES: usize = 4 * 1024 * 1024;
const JSON_MAX_BYTES: u64 = 64 * 1024 * 1024;
/// Same ceiling as the Pack admission bound for ONNX models.
const ONNX_MAX_BYTES: u64 = 1024 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(600);

/// Waits before retrying a request the Hub answered with `429 Too Many
/// Requests` or `503 Service Unavailable` (temporary refusals; observed as
/// 429 on the unauthenticated metadata API in batch run 38451112364). The
/// retry count is bounded; the last refusal is returned as `Http`.
#[cfg(not(test))]
const RETRY_BACKOFF: [Duration; 3] = [
    Duration::from_secs(5),
    Duration::from_secs(20),
    Duration::from_secs(60),
];
#[cfg(test)]
const RETRY_BACKOFF: [Duration; 3] = [Duration::ZERO; 3];

fn is_temporary_refusal(status: u16) -> bool {
    matches!(status, 429 | 503)
}

/// Runs `request` again after each backoff while it returns a temporary
/// refusal; transport errors and every other status return at once.
fn with_retry<R>(
    status: impl Fn(&R) -> u16,
    mut request: impl FnMut() -> Result<R, AcquisitionError>,
) -> Result<R, AcquisitionError> {
    for wait in RETRY_BACKOFF {
        let response = request()?;
        if !is_temporary_refusal(status(&response)) {
            return Ok(response);
        }
        std::thread::sleep(wait);
    }
    request()
}

/// The live Governed Browse transport (public addresses only, no redirects
/// followed by the transport). Used only after explicit consent.
pub fn live_transport() -> medscale_network::UreqBrowseTransport {
    medscale_network::UreqBrowseTransport
}

/// Explicit, per-model consent captured from the person before any request.
#[derive(Debug, Clone)]
pub struct AcquisitionConsent {
    pub repo_id: String,
    pub onnx_file: String,
    /// Upper bound the person approved for the total download.
    pub approved_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AcquiredFile {
    pub name: String,
    pub bytes: u64,
    pub sha256: String,
    /// True when the file was reused from verified staging of an earlier,
    /// interrupted acquisition of the same commit.
    pub resumed: bool,
    /// Bytes of a partial download reused through a `Range` request.
    pub resumed_from: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AcquisitionReport {
    pub repo_id: String,
    pub revision: String,
    pub snapshot_dir: PathBuf,
    pub files: Vec<AcquiredFile>,
    pub total_bytes: u64,
    /// Bytes reused from verified staging rather than downloaded.
    pub resumed_bytes: u64,
}

/// Name of the marker that binds a staging directory to one pinned commit.
const STAGING_MARKER: &str = ".medscale-staging";

impl AcquisitionError {
    /// Integrity failures discard staging; interruptions keep verified files.
    fn discards_staging(&self) -> bool {
        matches!(
            self,
            AcquisitionError::Snapshot(_)
                | AcquisitionError::HostNotAllowed(_)
                | AcquisitionError::UrlRefused(_)
                | AcquisitionError::TooLarge { .. }
        )
    }
}

/// Opens (or creates) staging for `repo_id@revision`, discarding staging that
/// belongs to any other commit.
fn open_staging(staging: &Path, repo_id: &str, revision: &str) -> Result<(), AcquisitionError> {
    let marker = format!("{repo_id}@{revision}\n");
    let current = std::fs::read_to_string(staging.join(STAGING_MARKER)).ok();
    if current.as_deref() != Some(marker.as_str()) {
        let _ = std::fs::remove_dir_all(staging);
    }
    std::fs::create_dir_all(staging).map_err(|e| io(&e))?;
    std::fs::write(staging.join(STAGING_MARKER), marker).map_err(|e| io(&e))
}

/// A staged file is reused only when it still has the exact size and digest.
fn staged_file(meta: &HfRepoMetadata, staging: &Path, name: &str, size: u64) -> Option<String> {
    let path = staging.join(name);
    let len = std::fs::metadata(&path).ok()?.len();
    let verified = if len == size {
        std::fs::read(&path)
            .ok()
            .and_then(|body| verify_file(meta, name, &body).ok())
    } else {
        None
    };
    if verified.is_none() {
        let _ = std::fs::remove_file(&path);
    }
    verified
}

#[derive(Debug, thiserror::Error)]
pub enum AcquisitionError {
    #[error("catalog: {0}")]
    Catalog(String),
    #[error("{0} is not in the catalog")]
    NotInCatalog(String),
    #[error("consent does not cover {0}")]
    ConsentMismatch(String),
    #[error("download needs {needed} bytes; consent approved {approved}")]
    ExceedsConsent { needed: u64, approved: u64 },
    #[error("{0}: exact size unknown in Hub metadata; refusing")]
    UnknownSize(String),
    #[error("{name}: {bytes} bytes exceeds the {limit}-byte bound")]
    TooLarge {
        name: String,
        bytes: u64,
        limit: u64,
    },
    #[error("host {0} is not an allowed Hub host")]
    HostNotAllowed(String),
    #[error("URL refused: {0}")]
    UrlRefused(String),
    #[error("too many redirects")]
    TooManyRedirects,
    #[error("HTTP {status} for {url}")]
    Http { status: u16, url: String },
    #[error("transport: {0:?}")]
    Transport(BrowseTransportError),
    #[error(transparent)]
    Snapshot(#[from] SnapshotError),
    #[error("metadata: {0}")]
    Metadata(String),
    #[error("io: {0}")]
    Io(String),
}

fn io(e: &std::io::Error) -> AcquisitionError {
    AcquisitionError::Io(e.to_string())
}

/// One governed GET, following redirects only to allowed Hub hosts.
fn governed_get(
    transport: &dyn BrowseTransport,
    url: &str,
    max: usize,
) -> Result<Vec<u8>, AcquisitionError> {
    let mut next = url.to_string();
    for _ in 0..=MAX_HOPS {
        let validated =
            validate_url(&next).map_err(|r| AcquisitionError::UrlRefused(format!("{r:?}")))?;
        if !is_hub_host(&validated.host) {
            return Err(AcquisitionError::HostNotAllowed(validated.host));
        }
        let response = with_retry(
            |r: &medscale_network::BrowseHttpResponse| r.status,
            || {
                transport
                    .get(&validated, max, TIMEOUT)
                    .map_err(AcquisitionError::Transport)
            },
        )?;
        match response.status {
            200 => {
                if response.body.len() > max {
                    return Err(AcquisitionError::TooLarge {
                        name: validated.path,
                        bytes: response.body.len() as u64,
                        limit: max as u64,
                    });
                }
                return Ok(response.body);
            }
            301 | 302 | 303 | 307 | 308 => {
                let location = response.location.ok_or(AcquisitionError::Http {
                    status: response.status,
                    url: validated.url.clone(),
                })?;
                next = resolve_redirect(&validated, &location);
            }
            status => {
                return Err(AcquisitionError::Http {
                    status,
                    url: validated.url,
                });
            }
        }
    }
    Err(AcquisitionError::TooManyRedirects)
}

fn expected_size(meta: &HfRepoMetadata, name: &str) -> Result<u64, AcquisitionError> {
    let sibling = meta
        .siblings
        .iter()
        .find(|s| s.rfilename == name)
        .ok_or_else(|| {
            AcquisitionError::Snapshot(SnapshotError::NotInSnapshot(name.to_string()))
        })?;
    sibling
        .lfs
        .as_ref()
        .and_then(|l| l.size)
        .or(sibling.size)
        .ok_or_else(|| AcquisitionError::UnknownSize(name.to_string()))
}

/// One governed streamed download into `part`, resuming from its current
/// length, following redirects only to allowed Hub hosts.
fn governed_download(
    transport: &dyn BrowseTransport,
    url: &str,
    part: &Path,
    size: u64,
) -> Result<u64, AcquisitionError> {
    let mut next = url.to_string();
    for _ in 0..=MAX_HOPS {
        let validated =
            validate_url(&next).map_err(|r| AcquisitionError::UrlRefused(format!("{r:?}")))?;
        if !is_hub_host(&validated.host) {
            return Err(AcquisitionError::HostNotAllowed(validated.host));
        }
        let existing = std::fs::metadata(part).map(|m| m.len()).unwrap_or(0);
        let resume = if existing < size {
            existing
        } else {
            let _ = std::fs::remove_file(part);
            0
        };
        let response = with_retry(
            |r: &medscale_network::BrowseFileResponse| r.status,
            || {
                transport
                    .get_to_file(&validated, resume, size, part, TIMEOUT)
                    .map_err(AcquisitionError::Transport)
            },
        )?;
        match response.status {
            200 | 206 => {
                if response.file_len > size {
                    let _ = std::fs::remove_file(part);
                    return Err(AcquisitionError::TooLarge {
                        name: validated.path,
                        bytes: response.file_len,
                        limit: size,
                    });
                }
                return Ok(if response.status == 206 { resume } else { 0 });
            }
            301 | 302 | 303 | 307 | 308 => {
                let location = response.location.ok_or(AcquisitionError::Http {
                    status: response.status,
                    url: validated.url.clone(),
                })?;
                next = resolve_redirect(&validated, &location);
            }
            status => {
                return Err(AcquisitionError::Http {
                    status,
                    url: validated.url,
                });
            }
        }
    }
    Err(AcquisitionError::TooManyRedirects)
}

/// Upper bound on commits searched for the catalog-pinned revision.
const MAX_REVISION_SEARCH: usize = 32;
const COMMITS_MAX_BYTES: usize = 1024 * 1024;

#[derive(serde::Deserialize)]
struct HubCommit {
    id: String,
}

fn parse_metadata(bytes: &[u8]) -> Result<HfRepoMetadata, AcquisitionError> {
    serde_json::from_slice(bytes).map_err(|e| AcquisitionError::Metadata(e.to_string()))
}

/// Hub metadata whose reproducibility hash matches the catalog row.
///
/// The catalog binds a row to one commit, but a repository can receive later
/// commits (for example model-card edits) after the catalog snapshot. Then the
/// current metadata no longer matches. Instead of failing, the commit history
/// is searched, newest first and bounded, for the revision whose metadata
/// reproduces the catalog hash. Files are then fetched from that commit only.
/// Every request goes through the same governed transport and host allowlist.
fn pinned_metadata(
    transport: &dyn BrowseTransport,
    row: &CatalogRow,
) -> Result<(HfRepoMetadata, Vec<u8>), AcquisitionError> {
    let base = format!("https://huggingface.co/api/models/{}", row.repo_id);
    let bytes = governed_get(transport, &format!("{base}?blobs=true"), METADATA_MAX_BYTES)?;
    let meta = parse_metadata(&bytes)?;
    let mismatch = match verify_against_catalog(&meta, row) {
        Ok(()) => return Ok((meta, bytes)),
        Err(e @ SnapshotError::ReproducibilityMismatch { .. }) => e,
        Err(e) => return Err(e.into()),
    };
    let commits_bytes = governed_get(
        transport,
        &format!("{base}/commits/main"),
        COMMITS_MAX_BYTES,
    )?;
    let commits: Vec<HubCommit> = serde_json::from_slice(&commits_bytes)
        .map_err(|e| AcquisitionError::Metadata(e.to_string()))?;
    for commit in commits.iter().take(MAX_REVISION_SEARCH) {
        let hex40 = commit.id.len() == 40 && commit.id.bytes().all(|b| b.is_ascii_hexdigit());
        if !hex40 || commit.id == meta.sha {
            continue;
        }
        let bytes = governed_get(
            transport,
            &format!("{base}/revision/{}?blobs=true", commit.id),
            METADATA_MAX_BYTES,
        )?;
        let candidate = parse_metadata(&bytes)?;
        if candidate.sha == commit.id && verify_against_catalog(&candidate, row).is_ok() {
            return Ok((candidate, bytes));
        }
    }
    Err(mismatch.into())
}

/// Acquires the snapshot files for one catalog row into `dest_dir`
/// (`metadata.json`, `config.json`, `tokenizer.json`, the ONNX file).
pub fn acquire_snapshot(
    transport: &dyn BrowseTransport,
    catalog: &ModelCatalog,
    consent: &AcquisitionConsent,
    dest_dir: &Path,
) -> Result<AcquisitionReport, AcquisitionError> {
    let row: &CatalogRow = catalog
        .get(&consent.repo_id)
        .ok_or_else(|| AcquisitionError::NotInCatalog(consent.repo_id.clone()))?;
    let weight_file = consent.onnx_file.to_ascii_lowercase();
    if !(weight_file.ends_with(".onnx") || weight_file.ends_with(".safetensors")) {
        return Err(AcquisitionError::ConsentMismatch(consent.onnx_file.clone()));
    }

    // 1. Metadata, bound to the catalog row (immutable commit and file list),
    //    at the catalog-pinned revision even if the repository moved on.
    let (meta, meta_bytes) = pinned_metadata(transport, row)?;

    // 2. Exact sizes and the consent bound, before any file download.
    let files = ["config.json", "tokenizer.json", consent.onnx_file.as_str()];
    let mut total = 0u64;
    for name in files {
        let bytes = expected_size(&meta, name)?;
        let limit = if name == consent.onnx_file {
            ONNX_MAX_BYTES
        } else {
            JSON_MAX_BYTES
        };
        if bytes > limit {
            return Err(AcquisitionError::TooLarge {
                name: name.to_string(),
                bytes,
                limit,
            });
        }
        total += bytes;
    }
    if total > consent.approved_bytes {
        return Err(AcquisitionError::ExceedsConsent {
            needed: total,
            approved: consent.approved_bytes,
        });
    }

    // 3. Download into staging (reusing verified files of an interrupted
    //    acquisition of the same commit); verify each file; publish atomically.
    let staging = dest_dir.with_extension("partial");
    open_staging(&staging, &meta.id, &meta.sha)?;
    let result: Result<Vec<AcquiredFile>, AcquisitionError> = (|| {
        let mut acquired = Vec::new();
        for name in files {
            let size = expected_size(&meta, name)?;
            if let Some(sha256) = staged_file(&meta, &staging, name, size) {
                acquired.push(AcquiredFile {
                    name: name.to_string(),
                    bytes: size,
                    sha256,
                    resumed: true,
                    resumed_from: 0,
                });
                continue;
            }
            let url = format!(
                "https://huggingface.co/{}/resolve/{}/{name}",
                meta.id, meta.sha
            );
            // Streamed into `<name>.part`, resuming from any bytes left by an
            // interrupted attempt; it becomes the staged file only after the
            // exact size and digest verify.
            let part = staging.join(format!("{name}.part"));
            let resumed_from = governed_download(transport, &url, &part, size)?;
            let body = std::fs::read(&part).map_err(|e| io(&e))?;
            if body.len() as u64 != size {
                let _ = std::fs::remove_file(&part);
                return Err(AcquisitionError::Snapshot(SnapshotError::SizeMismatch {
                    path: name.to_string(),
                }));
            }
            let sha256 = match verify_file(&meta, name, &body) {
                Ok(sha256) => sha256,
                Err(e) => {
                    let _ = std::fs::remove_file(&part);
                    return Err(e.into());
                }
            };
            drop(body);
            std::fs::rename(&part, staging.join(name)).map_err(|e| io(&e))?;
            acquired.push(AcquiredFile {
                name: name.to_string(),
                bytes: size,
                sha256,
                resumed: false,
                resumed_from,
            });
        }
        std::fs::write(staging.join("metadata.json"), &meta_bytes).map_err(|e| io(&e))?;
        Ok(acquired)
    })();
    let acquired = match result {
        Ok(a) => a,
        Err(e) => {
            if e.discards_staging() {
                let _ = std::fs::remove_dir_all(&staging);
            }
            return Err(e);
        }
    };
    let _ = std::fs::remove_file(staging.join(STAGING_MARKER));
    let _ = std::fs::remove_dir_all(dest_dir);
    std::fs::rename(&staging, dest_dir).map_err(|e| io(&e))?;
    let resumed_bytes = acquired
        .iter()
        .map(|f| if f.resumed { f.bytes } else { f.resumed_from })
        .sum();
    Ok(AcquisitionReport {
        repo_id: meta.id.clone(),
        revision: meta.sha.clone(),
        snapshot_dir: dest_dir.to_path_buf(),
        files: acquired,
        total_bytes: total,
        resumed_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_network::ScriptedBrowseTransport;

    /// Answers every GET with `status` for the first `refusals` calls, then
    /// with `200` and a fixed body.
    struct RefusingTransport {
        status: u16,
        refusals: usize,
        calls: std::sync::atomic::AtomicUsize,
    }

    impl BrowseTransport for RefusingTransport {
        fn get(
            &self,
            _url: &medscale_network::ValidatedUrl,
            _max: usize,
            _timeout: Duration,
        ) -> Result<medscale_network::BrowseHttpResponse, medscale_network::BrowseTransportError>
        {
            let n = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let status = if n < self.refusals { self.status } else { 200 };
            Ok(medscale_network::BrowseHttpResponse {
                status,
                location: None,
                content_type: "application/json".into(),
                www_authenticate: false,
                body: b"{}".to_vec(),
            })
        }
    }

    #[test]
    fn temporary_refusals_are_retried_a_bounded_number_of_times() {
        let url = "https://huggingface.co/api/models/OpenMed/x";
        for status in [429, 503] {
            let t = RefusingTransport {
                status,
                refusals: RETRY_BACKOFF.len(),
                calls: Default::default(),
            };
            assert_eq!(governed_get(&t, url, 1024).unwrap(), b"{}");
            assert_eq!(t.calls.load(std::sync::atomic::Ordering::SeqCst), 4);
        }
        // Still refused after every retry: the refusal is reported.
        let t = RefusingTransport {
            status: 429,
            refusals: usize::MAX,
            calls: Default::default(),
        };
        assert!(matches!(
            governed_get(&t, url, 1024),
            Err(AcquisitionError::Http { status: 429, .. })
        ));
        assert_eq!(t.calls.load(std::sync::atomic::Ordering::SeqCst), 4);
        // Other errors are not retried.
        let t = RefusingTransport {
            status: 404,
            refusals: usize::MAX,
            calls: Default::default(),
        };
        assert!(matches!(
            governed_get(&t, url, 1024),
            Err(AcquisitionError::Http { status: 404, .. })
        ));
        assert_eq!(t.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
    use sha2::{Digest, Sha256};

    const COMMIT: &str = "ea920f36fadd7b45935247d639f0ffa1ef493b23";
    const SHA: &str = "54c9cc119d325ddad23300a1e9108bafa9643a06";
    const REPO: &str = "OpenMed/Synthetic-NER-onnx";

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }
    fn git_blob(b: &[u8]) -> String {
        let mut h = sha1::Sha1::new();
        h.update(format!("blob {}\0", b.len()).as_bytes());
        h.update(b);
        hex(&h.finalize())
    }

    struct Fixture {
        config: Vec<u8>,
        tokenizer: Vec<u8>,
        onnx: Vec<u8>,
        meta: String,
        catalog: ModelCatalog,
    }

    fn fixture(gated: bool) -> Fixture {
        let config = br#"{"id2label":{"0":"O","1":"B-X"}}"#.to_vec();
        let tokenizer = br#"{"version":"1.0"}"#.to_vec();
        let onnx = vec![42u8; 4096];
        let meta = format!(
            r#"{{"id":"{REPO}","sha":"{SHA}","lastModified":"2026-07-13T07:43:16.000Z","gated":{},"private":false,"siblings":[{{"rfilename":"config.json","size":{},"blobId":"{}"}},{{"rfilename":"model.onnx","size":4096,"lfs":{{"sha256":"{}","size":4096}}}},{{"rfilename":"tokenizer.json","size":{},"blobId":"{}"}}]}}"#,
            if gated { "\"auto\"" } else { "false" },
            config.len(),
            git_blob(&config),
            hex(&Sha256::digest(&onnx)),
            tokenizer.len(),
            git_blob(&tokenizer)
        );
        let parsed: HfRepoMetadata =
            serde_json::from_str(&meta.replace("\"auto\"", "false")).unwrap();
        let hash = medscale_pack::reproducibility_hash(&parsed);
        let row = format!(
            r#"{{"repo_id":"{REPO}","family":"NER","task":"token-classification","formats":["onnx"],"license":"apache-2.0","reproducibility_hash":"{hash}"}}"#
        );
        let catalog = ModelCatalog::import("s/f", COMMIT, row.as_bytes()).unwrap();
        Fixture {
            config,
            tokenizer,
            onnx,
            meta,
            catalog,
        }
    }

    fn transport(f: &Fixture, onnx_body: &[u8], cdn: &str) -> ScriptedBrowseTransport {
        let base = format!("https://huggingface.co/{REPO}/resolve/{SHA}");
        ScriptedBrowseTransport::new()
            .route(
                &format!("https://huggingface.co/api/models/{REPO}?blobs=true"),
                ScriptedBrowseTransport::ok("application/json", f.meta.as_bytes()),
            )
            .route(
                &format!("{base}/config.json"),
                ScriptedBrowseTransport::ok("application/json", &f.config),
            )
            .route(
                &format!("{base}/tokenizer.json"),
                ScriptedBrowseTransport::ok("application/json", &f.tokenizer),
            )
            .route(
                &format!("{base}/model.onnx"),
                ScriptedBrowseTransport::redirect(&format!("https://{cdn}/blobs/abc")),
            )
            .route(
                &format!("https://{cdn}/blobs/abc"),
                ScriptedBrowseTransport::ok("application/octet-stream", onnx_body),
            )
    }

    fn consent(bytes: u64) -> AcquisitionConsent {
        AcquisitionConsent {
            repo_id: REPO.into(),
            onnx_file: "model.onnx".into(),
            approved_bytes: bytes,
        }
    }

    fn dest(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("medscale-acq-{tag}-{}", std::process::id()))
    }

    #[test]
    fn acquires_verified_files_through_an_allowed_cdn_redirect() {
        let f = fixture(false);
        let t = transport(&f, &f.onnx, "cdn-lfs.huggingface.co");
        let d = dest("ok");
        let report = acquire_snapshot(&t, &f.catalog, &consent(1_000_000), &d).unwrap();
        assert_eq!(report.revision, SHA);
        assert_eq!(
            report.total_bytes,
            (f.config.len() + f.tokenizer.len() + 4096) as u64
        );
        for name in [
            "metadata.json",
            "config.json",
            "tokenizer.json",
            "model.onnx",
        ] {
            assert!(d.join(name).is_file(), "{name}");
        }
        assert!(!d.with_extension("partial").exists());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn tampered_artifact_leaves_nothing_behind() {
        let f = fixture(false);
        let t = transport(&f, &vec![7u8; 4096], "cdn-lfs.huggingface.co");
        let d = dest("tamper");
        let err = acquire_snapshot(&t, &f.catalog, &consent(1_000_000), &d).unwrap_err();
        assert!(
            matches!(
                err,
                AcquisitionError::Snapshot(SnapshotError::DigestMismatch { .. })
            ),
            "{err}"
        );
        assert!(!d.exists() && !d.with_extension("partial").exists());
    }

    /// Metadata and both JSON files answer; the ONNX download times out.
    fn interrupted_transport(f: &Fixture) -> ScriptedBrowseTransport {
        let base = format!("https://huggingface.co/{REPO}/resolve/{SHA}");
        ScriptedBrowseTransport::new()
            .route(
                &format!("https://huggingface.co/api/models/{REPO}?blobs=true"),
                ScriptedBrowseTransport::ok("application/json", f.meta.as_bytes()),
            )
            .route(
                &format!("{base}/config.json"),
                ScriptedBrowseTransport::ok("application/json", &f.config),
            )
            .route(
                &format!("{base}/tokenizer.json"),
                ScriptedBrowseTransport::ok("application/json", &f.tokenizer),
            )
            .route(
                &format!("{base}/model.onnx"),
                Err(BrowseTransportError::Timeout),
            )
    }

    /// Only metadata and the ONNX file answer: any JSON re-download fails.
    fn onnx_only_transport(f: &Fixture) -> ScriptedBrowseTransport {
        ScriptedBrowseTransport::new()
            .route(
                &format!("https://huggingface.co/api/models/{REPO}?blobs=true"),
                ScriptedBrowseTransport::ok("application/json", f.meta.as_bytes()),
            )
            .route(
                &format!("https://huggingface.co/{REPO}/resolve/{SHA}/model.onnx"),
                ScriptedBrowseTransport::ok("application/octet-stream", &f.onnx),
            )
    }

    #[test]
    fn interrupted_acquisition_resumes_from_verified_staging() {
        let f = fixture(false);
        let d = dest("resume");
        let staging = d.with_extension("partial");
        let err = acquire_snapshot(
            &interrupted_transport(&f),
            &f.catalog,
            &consent(1_000_000),
            &d,
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                AcquisitionError::Transport(BrowseTransportError::Timeout)
            ),
            "{err}"
        );
        assert!(!d.exists());
        assert!(staging.join("config.json").is_file());
        assert!(staging.join("tokenizer.json").is_file());
        assert!(!staging.join("model.onnx").exists());

        let report = acquire_snapshot(
            &onnx_only_transport(&f),
            &f.catalog,
            &consent(1_000_000),
            &d,
        )
        .unwrap();
        let resumed: Vec<_> = report
            .files
            .iter()
            .filter(|x| x.resumed)
            .map(|x| x.name.as_str())
            .collect();
        assert_eq!(resumed, ["config.json", "tokenizer.json"]);
        assert_eq!(
            report.resumed_bytes,
            (f.config.len() + f.tokenizer.len()) as u64
        );
        assert!(d.join("model.onnx").is_file());
        assert!(!d.join(STAGING_MARKER).exists());
        assert!(!staging.exists());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn an_interrupted_file_resumes_with_a_range_request() {
        let f = fixture(false);
        let d = dest("range");
        let staging = d.with_extension("partial");
        let cdn = "https://cdn-lfs.huggingface.co/blobs/abc";
        let interrupted =
            transport(&f, &f.onnx, "cdn-lfs.huggingface.co").interrupt_after(cdn, 1000);
        let err = acquire_snapshot(&interrupted, &f.catalog, &consent(1_000_000), &d).unwrap_err();
        assert!(
            matches!(
                err,
                AcquisitionError::Transport(BrowseTransportError::Timeout)
            ),
            "{err}"
        );
        assert_eq!(
            std::fs::metadata(staging.join("model.onnx.part"))
                .unwrap()
                .len(),
            1000
        );
        let report = acquire_snapshot(
            &transport(&f, &f.onnx, "cdn-lfs.huggingface.co"),
            &f.catalog,
            &consent(1_000_000),
            &d,
        )
        .unwrap();
        let onnx = report
            .files
            .iter()
            .find(|x| x.name == "model.onnx")
            .unwrap();
        assert_eq!(onnx.resumed_from, 1000);
        assert_eq!(std::fs::read(d.join("model.onnx")).unwrap(), f.onnx);
        assert!(report.resumed_bytes >= 1000);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn a_server_that_ignores_range_replaces_the_partial_file() {
        let f = fixture(false);
        let d = dest("norange");
        let cdn = "https://cdn-lfs.huggingface.co/blobs/abc";
        let _ = acquire_snapshot(
            &transport(&f, &f.onnx, "cdn-lfs.huggingface.co").interrupt_after(cdn, 1000),
            &f.catalog,
            &consent(1_000_000),
            &d,
        );
        let report = acquire_snapshot(
            &transport(&f, &f.onnx, "cdn-lfs.huggingface.co").ignoring_range(),
            &f.catalog,
            &consent(1_000_000),
            &d,
        )
        .unwrap();
        let onnx = report
            .files
            .iter()
            .find(|x| x.name == "model.onnx")
            .unwrap();
        assert_eq!(onnx.resumed_from, 0);
        assert_eq!(std::fs::read(d.join("model.onnx")).unwrap(), f.onnx);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn a_corrupted_partial_file_is_caught_by_the_digest() {
        let f = fixture(false);
        let d = dest("badpart");
        let staging = d.with_extension("partial");
        let cdn = "https://cdn-lfs.huggingface.co/blobs/abc";
        let _ = acquire_snapshot(
            &transport(&f, &f.onnx, "cdn-lfs.huggingface.co").interrupt_after(cdn, 1000),
            &f.catalog,
            &consent(1_000_000),
            &d,
        );
        // Tamper with the bytes kept for resumption.
        std::fs::write(staging.join("model.onnx.part"), vec![9u8; 1000]).unwrap();
        let err = acquire_snapshot(
            &transport(&f, &f.onnx, "cdn-lfs.huggingface.co"),
            &f.catalog,
            &consent(1_000_000),
            &d,
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                AcquisitionError::Snapshot(SnapshotError::DigestMismatch { .. })
            ),
            "{err}"
        );
        assert!(!d.exists() && !staging.exists());
    }

    #[test]
    fn corrupted_staged_file_is_downloaded_again() {
        let f = fixture(false);
        let d = dest("corrupt");
        let staging = d.with_extension("partial");
        let _ = acquire_snapshot(
            &interrupted_transport(&f),
            &f.catalog,
            &consent(1_000_000),
            &d,
        );
        // Same size, different bytes: must not be reused.
        std::fs::write(staging.join("config.json"), vec![b' '; f.config.len()]).unwrap();
        // The JSON files are not scripted here, so the corrupted file cannot be
        // replaced and the run stops; nothing unverified is published.
        let err = acquire_snapshot(
            &onnx_only_transport(&f),
            &f.catalog,
            &consent(1_000_000),
            &d,
        )
        .unwrap_err();
        assert!(matches!(err, AcquisitionError::Transport(_)), "{err}");
        assert!(!d.exists());
        assert!(!staging.join("config.json").exists());
        let report = acquire_snapshot(
            &transport(&f, &f.onnx, "cdn-lfs.huggingface.co"),
            &f.catalog,
            &consent(1_000_000),
            &d,
        )
        .unwrap();
        let config = report
            .files
            .iter()
            .find(|x| x.name == "config.json")
            .unwrap();
        assert!(!config.resumed);
        assert_eq!(std::fs::read(d.join("config.json")).unwrap(), f.config);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn staging_for_another_commit_is_discarded() {
        let f = fixture(false);
        let d = dest("stale");
        let staging = d.with_extension("partial");
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(
            staging.join(STAGING_MARKER),
            format!("{REPO}@{}\n", "0".repeat(40)),
        )
        .unwrap();
        std::fs::write(staging.join("config.json"), &f.config).unwrap();
        std::fs::write(staging.join("stray.bin"), b"x").unwrap();
        let report = acquire_snapshot(
            &transport(&f, &f.onnx, "cdn-lfs.huggingface.co"),
            &f.catalog,
            &consent(1_000_000),
            &d,
        )
        .unwrap();
        assert_eq!(report.resumed_bytes, 0);
        assert!(!d.join("stray.bin").exists());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn regional_hub_cdn_hosts_are_allowed_by_suffix_only() {
        for host in [
            "huggingface.co",
            "cdn-lfs.huggingface.co",
            "us.aws.cdn.hf.co",
            "eu.aws.cdn.hf.co",
            "cas-bridge.xethub.hf.co",
        ] {
            assert!(is_hub_host(host), "{host}");
        }
        for host in [
            "cdn.hf.co.example.com",
            "evilcdn.hf.co",
            "hf.co",
            "huggingface.co.example.com",
            "example.com",
        ] {
            assert!(!is_hub_host(host), "{host}");
        }
        let f = fixture(false);
        let t = transport(&f, &f.onnx, "us.aws.cdn.hf.co");
        let d = dest("regional");
        let report = acquire_snapshot(&t, &f.catalog, &consent(1_000_000), &d).unwrap();
        assert_eq!(report.revision, SHA);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn redirect_to_a_non_hub_host_is_refused() {
        let f = fixture(false);
        let t = transport(&f, &f.onnx, "evil.example.com");
        let err = acquire_snapshot(&t, &f.catalog, &consent(1_000_000), &dest("host")).unwrap_err();
        assert!(matches!(err, AcquisitionError::HostNotAllowed(h) if h == "evil.example.com"));
    }

    #[test]
    fn insufficient_consent_refuses_before_any_file_download() {
        let f = fixture(false);
        // Only metadata is scripted: a file request would fail as a transport error.
        let t = ScriptedBrowseTransport::new().route(
            &format!("https://huggingface.co/api/models/{REPO}?blobs=true"),
            ScriptedBrowseTransport::ok("application/json", f.meta.as_bytes()),
        );
        let err = acquire_snapshot(&t, &f.catalog, &consent(100), &dest("consent")).unwrap_err();
        assert!(matches!(
            err,
            AcquisitionError::ExceedsConsent { approved: 100, .. }
        ));
    }

    /// The repository received a later commit (new sha, extra file) after
    /// the catalog snapshot; the pinned revision is found in its history.
    fn moved_repository(f: &Fixture) -> ScriptedBrowseTransport {
        const NEW: &str = "1111111111111111111111111111111111111111";
        let newer = f.meta.replace(SHA, NEW).replace(
            "\"siblings\":[",
            "\"siblings\":[{\"rfilename\":\"README.md\"},",
        );
        let api = format!("https://huggingface.co/api/models/{REPO}");
        let files = format!("https://huggingface.co/{REPO}/resolve/{SHA}");
        // Routes match first-come, so the moved main metadata is listed first.
        ScriptedBrowseTransport::new()
            .route(
                &format!("{api}?blobs=true"),
                ScriptedBrowseTransport::ok("application/json", newer.as_bytes()),
            )
            .route(
                &format!("{api}/commits/main"),
                ScriptedBrowseTransport::ok(
                    "application/json",
                    format!(r#"[{{"id":"{NEW}"}},{{"id":"not-a-sha"}},{{"id":"{SHA}"}}]"#)
                        .as_bytes(),
                ),
            )
            .route(
                &format!("{api}/revision/{SHA}?blobs=true"),
                ScriptedBrowseTransport::ok("application/json", f.meta.as_bytes()),
            )
            .route(
                &format!("{files}/config.json"),
                ScriptedBrowseTransport::ok("application/json", &f.config),
            )
            .route(
                &format!("{files}/tokenizer.json"),
                ScriptedBrowseTransport::ok("application/json", &f.tokenizer),
            )
            .route(
                &format!("{files}/model.onnx"),
                ScriptedBrowseTransport::ok("application/octet-stream", &f.onnx),
            )
    }

    #[test]
    fn a_moved_repository_is_acquired_at_the_catalog_pinned_revision() {
        let f = fixture(false);
        let d = dest("moved");
        let report =
            acquire_snapshot(&moved_repository(&f), &f.catalog, &consent(1_000_000), &d).unwrap();
        assert_eq!(report.revision, SHA);
        let meta: HfRepoMetadata =
            serde_json::from_slice(&std::fs::read(d.join("metadata.json")).unwrap()).unwrap();
        assert_eq!(meta.sha, SHA);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn no_matching_revision_keeps_the_reproducibility_refusal() {
        let f = fixture(false);
        let base = format!("https://huggingface.co/api/models/{REPO}");
        let newer = f
            .meta
            .replace(SHA, "2222222222222222222222222222222222222222");
        let t = ScriptedBrowseTransport::new()
            .route(
                &format!("{base}?blobs=true"),
                ScriptedBrowseTransport::ok("application/json", newer.as_bytes()),
            )
            .route(
                &format!("{base}/commits/main"),
                ScriptedBrowseTransport::ok("application/json", b"[]"),
            );
        let err =
            acquire_snapshot(&t, &f.catalog, &consent(1_000_000), &dest("nomatch")).unwrap_err();
        assert!(
            matches!(
                err,
                AcquisitionError::Snapshot(SnapshotError::ReproducibilityMismatch { .. })
            ),
            "{err}"
        );
    }

    #[test]
    fn gated_repositories_and_unlisted_models_are_refused() {
        let f = fixture(true);
        let t = transport(&f, &f.onnx, "cdn-lfs.huggingface.co");
        let err =
            acquire_snapshot(&t, &f.catalog, &consent(1_000_000), &dest("gated")).unwrap_err();
        assert!(
            matches!(err, AcquisitionError::Snapshot(SnapshotError::Gated)),
            "{err}"
        );
        let mut other = consent(1_000_000);
        other.repo_id = "OpenMed/not-listed".into();
        assert!(matches!(
            acquire_snapshot(&t, &f.catalog, &other, &dest("unlisted")),
            Err(AcquisitionError::NotInCatalog(_))
        ));
    }
}
