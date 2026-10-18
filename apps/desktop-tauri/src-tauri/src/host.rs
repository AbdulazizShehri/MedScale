//! Rust-owned Core host for the Tauri desktop.
//!
//! The frontend never holds a session, vault path, capability or Core request.
//! It calls named commands; this module owns the `CliSession`s, the vault
//! location and the subject context, and returns typed DTOs or a typed error.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use medscale_contracts::envelopes::AuthorityError;
use medscale_core::CliSession;
use serde::{Deserialize, Serialize};

/// Typed command error. `kind` maps one-to-one onto the error taxonomy the
/// interface renders (board 25); `message` is the verbatim Core status line.
#[derive(Debug, Serialize)]
pub struct CmdError {
    pub kind: &'static str,
    pub message: String,
}

impl CmdError {
    pub fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new("unavailable", message)
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid", message)
    }
}

impl From<AuthorityError> for CmdError {
    fn from(err: AuthorityError) -> Self {
        let kind = match &err {
            AuthorityError::Unauthorized
            | AuthorityError::SessionRequired
            | AuthorityError::SessionExpired
            | AuthorityError::SessionRevoked
            | AuthorityError::SessionDenied
            | AuthorityError::WrongScope
            | AuthorityError::BrokerDenied { .. }
            | AuthorityError::PackDenied { .. } => "denied",
            AuthorityError::ExternalGateRequired { .. } => "blocked",
            AuthorityError::NotFound => "missing",
            AuthorityError::Conflict { .. } => "conflict",
            AuthorityError::StaleReference { .. } => "stale",
            AuthorityError::InvalidArgument { .. }
            | AuthorityError::LexicalReject { .. }
            | AuthorityError::VersionReject { .. }
            | AuthorityError::IllegalTransition => "invalid",
            AuthorityError::Corrupt { .. } | AuthorityError::DigestMismatch => "corrupt",
            AuthorityError::UnsupportedSchema { .. } => "unsupported",
            AuthorityError::Cancelled { .. } => "cancelled",
            AuthorityError::Unavailable { .. }
            | AuthorityError::LeaseRequired
            | AuthorityError::VaultRequired
            | AuthorityError::LeaseHeld { .. }
            | AuthorityError::AlreadyHeld { .. }
            | AuthorityError::NotHolder
            | AuthorityError::NotHeld
            | AuthorityError::MissingKeyMaterial
            | AuthorityError::UnknownRequiresReconcile => "unavailable",
            AuthorityError::PathOutsideClaim | AuthorityError::Internal { .. } => "internal",
        };
        let message = medscale_desktop_vm::project_workspace::status_message(&err);
        Self::new(kind, message)
    }
}

pub type CmdResult<T> = Result<T, CmdError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultKind {
    Synthetic,
    Encrypted,
}

/// What this desktop ingested into the open vault. Core has no roster API, so
/// the subject list is exactly the subjects this app promoted, never inferred.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeedManifest {
    pub subjects: Vec<String>,
    pub sources: Vec<SeededSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeededSource {
    pub source_id: String,
    pub fixture: String,
    pub resource_type: String,
    pub claim_kind: String,
    pub subject_ref: String,
}

pub struct Workspace {
    pub session: CliSession,
    pub kind: VaultKind,
    pub vault_root: PathBuf,
    pub seed: SeedManifest,
}

impl Workspace {
    /// Seals an encrypted vault through Core. Synthetic vaults persist after
    /// every Core request and need no close step.
    pub fn close(&mut self) -> Result<(), AuthorityError> {
        if self.kind == VaultKind::Encrypted && self.session.is_open() {
            self.session.close_encrypted_vault()?;
        }
        Ok(())
    }
}

impl Drop for Workspace {
    /// Last-resort seal for paths that never reach an explicit lock (window
    /// close, app exit, panic unwinding). Errors cannot be reported here.
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[derive(Default)]
pub struct State {
    pub data_dir: Option<PathBuf>,
    pub workspace: Option<Workspace>,
    pub packs: Option<CliSession>,
}

#[derive(Default)]
pub struct Host {
    pub state: Mutex<State>,
}

/// Synthetic FHIR fixtures shipped inside the binary (no repository needed at
/// runtime). Each is ingested and promoted through governed Core calls.
struct Fixture {
    file: &'static str,
    bytes: &'static [u8],
    claim_kind: &'static str,
}

macro_rules! fixture {
    ($file:literal, $kind:literal) => {
        Fixture {
            file: $file,
            bytes: include_bytes!(concat!(
                "../../../../fixtures/synthetic/fhir/r4/presentation/",
                $file
            )),
            claim_kind: $kind,
        }
    };
}

/// Two synthetic subjects, mirroring the Core presentation qualification
/// fixtures: one complete record and one that exercises absence, conflict
/// and unit-comparability states.
fn seed_plan() -> Vec<(&'static str, Vec<Fixture>)> {
    vec![
        (
            "synthetic-subject-ada",
            vec![
                fixture!("patient-golden.json", "patient"),
                fixture!("observation-hr.json", "observation"),
                fixture!("observation-weight.json", "observation"),
                fixture!("observation-temp-c.json", "observation"),
                fixture!("condition-uri.json", "condition"),
                fixture!("unsupported-medicationrequest.json", "medication"),
            ],
        ),
        (
            "synthetic-subject-coverage",
            vec![
                fixture!("patient-absence.json", "patient"),
                fixture!("condition-uri.json", "condition"),
                fixture!("condition-conflict-b.json", "condition"),
                fixture!("observation-weight.json", "observation"),
                fixture!("observation-weight-conflict-unit.json", "observation"),
                fixture!("observation-unrecognized-unit.json", "observation"),
            ],
        ),
    ]
}

/// Fixture model packs shipped inside the binary and materialized into the app
/// data directory before Core admission (Core verifies digests/signatures).
pub struct PackFile {
    pub name: &'static str,
    pub bytes: &'static [u8],
}

pub const ONNX_FIXTURE_PACK: (&str, &[PackFile]) = (
    "pack-tiny-token-classifier-v0",
    &[
        PackFile {
            name: "pack.manifest.json",
            bytes: include_bytes!(
                "../../../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0/pack.manifest.json"
            ),
        },
        PackFile {
            name: "model.meta.json",
            bytes: include_bytes!(
                "../../../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0/model.meta.json"
            ),
        },
        PackFile {
            name: "model.onnx",
            bytes: include_bytes!(
                "../../../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0/model.onnx"
            ),
        },
        PackFile {
            name: "labels.json",
            bytes: include_bytes!(
                "../../../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0/labels.json"
            ),
        },
        PackFile {
            name: "tokenizer.json",
            bytes: include_bytes!(
                "../../../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0/tokenizer.json"
            ),
        },
    ],
);

impl State {
    pub fn data_dir(&self) -> CmdResult<&Path> {
        self.data_dir
            .as_deref()
            .ok_or_else(|| CmdError::unavailable("Unavailable: app data directory not resolved"))
    }

    pub fn workspace(&mut self) -> CmdResult<&mut Workspace> {
        self.workspace
            .as_mut()
            .ok_or_else(|| CmdError::unavailable("Unavailable: no workspace is open"))
    }

    pub fn session(&mut self) -> CmdResult<&mut CliSession> {
        Ok(&mut self.workspace()?.session)
    }

    /// Least-privilege Model Center session (PacksList + PacksInstallLocal only).
    pub fn packs(&mut self) -> CmdResult<&mut CliSession> {
        if self.packs.is_none() {
            self.packs = Some(CliSession::connect_pack_operator("desktop-model-center")?);
        }
        Ok(self.packs.as_mut().expect("pack session just installed"))
    }

    /// Writes a fixture pack into the app data directory and returns its path.
    pub fn materialize_pack(&self, pack: (&str, &[PackFile])) -> CmdResult<PathBuf> {
        let dir = self.data_dir()?.join("fixture-packs").join(pack.0);
        std::fs::create_dir_all(&dir)
            .map_err(|_| CmdError::unavailable("Unavailable: pack folder"))?;
        for file in pack.1 {
            std::fs::write(dir.join(file.name), file.bytes)
                .map_err(|_| CmdError::unavailable("Unavailable: pack file"))?;
        }
        Ok(dir)
    }
}

fn manifest_path(vault_root: &Path) -> PathBuf {
    let mut name = vault_root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "workspace".to_owned());
    name.push_str(".desktop-seed.json");
    vault_root.with_file_name(name)
}

fn load_manifest(vault_root: &Path) -> SeedManifest {
    std::fs::read(manifest_path(vault_root))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_manifest(vault_root: &Path, manifest: &SeedManifest) -> CmdResult<()> {
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|_| CmdError::new("internal", "Internal: seed manifest"))?;
    std::fs::write(manifest_path(vault_root), bytes)
        .map_err(|_| CmdError::unavailable("Unavailable: seed manifest not writable"))
}

/// Ingests and promotes the synthetic fixtures through Core, once per vault.
pub fn seed_synthetic(workspace: &mut Workspace, data_dir: &Path) -> CmdResult<()> {
    if !workspace.seed.subjects.is_empty() {
        return Ok(());
    }
    let staging = data_dir.join("fixture-staging");
    std::fs::create_dir_all(&staging).map_err(|_| CmdError::unavailable("Unavailable: staging"))?;
    let mut manifest = SeedManifest::default();
    for (subject, fixtures) in seed_plan() {
        for fixture in fixtures {
            let path = staging.join(fixture.file);
            std::fs::write(&path, fixture.bytes)
                .map_err(|_| CmdError::unavailable("Unavailable: staging write"))?;
            let resource: serde_json::Value = serde_json::from_slice(fixture.bytes)
                .map_err(|_| CmdError::new("corrupt", "Corrupt: fixture is not JSON"))?;
            let resource_type = resource
                .get("resourceType")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Unknown")
                .to_owned();
            let source_id = workspace
                .session
                .ingest_fhir_file(&path.display().to_string())?;
            workspace.session.promote_fixture_as_assertion(
                source_id.clone(),
                subject,
                fixture.claim_kind,
                resource,
            )?;
            manifest.sources.push(SeededSource {
                source_id: source_id.as_str().to_owned(),
                fixture: fixture.file.to_owned(),
                resource_type,
                claim_kind: fixture.claim_kind.to_owned(),
                subject_ref: subject.to_owned(),
            });
        }
        manifest.subjects.push(subject.to_owned());
    }
    let _ = std::fs::remove_dir_all(&staging);
    save_manifest(&workspace.vault_root, &manifest)?;
    workspace.seed = manifest;
    Ok(())
}

pub fn open_synthetic(state: &mut State) -> CmdResult<()> {
    let data_dir = state.data_dir()?.to_path_buf();
    let vault_root = data_dir.join("synthetic-workspace");
    std::fs::create_dir_all(&vault_root)
        .map_err(|_| CmdError::unavailable("Unavailable: vault folder not writable"))?;
    let mut session = CliSession::connect("desktop-synthetic")?;
    // Synthetic workspace: offline fixtures replace the network, ASR and
    // durable pseudonym keys. Each is labelled as a fixture in the interface.
    session.use_offline_browse_fixture();
    session.use_fixture_asr();
    session.use_in_memory_privacy_keys();
    session.open_synthetic_vault(&vault_root.display().to_string())?;
    let seed = load_manifest(&vault_root);
    let mut workspace = Workspace {
        session,
        kind: VaultKind::Synthetic,
        vault_root,
        seed,
    };
    seed_synthetic(&mut workspace, &data_dir)?;
    state.workspace = Some(workspace);
    Ok(())
}

pub fn encrypted_root(state: &State) -> CmdResult<PathBuf> {
    Ok(state.data_dir()?.join("encrypted-workspace"))
}

pub fn create_encrypted(state: &mut State, passphrase: &str) -> CmdResult<Vec<String>> {
    validate_passphrase(passphrase)?;
    let vault_root = encrypted_root(state)?;
    let occupied = std::fs::read_dir(&vault_root)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false);
    if occupied || manifest_path(&vault_root).exists() {
        return Err(CmdError::new(
            "conflict",
            "Conflict: an encrypted vault already exists here, nothing written",
        ));
    }
    std::fs::create_dir_all(&vault_root)
        .map_err(|_| CmdError::unavailable("Unavailable: vault folder not writable"))?;
    let mut session = CliSession::connect("desktop-encrypted")?;
    let codes = session.create_encrypted_vault(&vault_root.display().to_string(), passphrase)?;
    save_manifest(&vault_root, &SeedManifest::default())?;
    state.workspace = Some(Workspace {
        session,
        kind: VaultKind::Encrypted,
        vault_root,
        seed: SeedManifest::default(),
    });
    Ok(codes)
}

pub fn unlock_encrypted(state: &mut State, passphrase: &str) -> CmdResult<()> {
    validate_passphrase(passphrase)?;
    let vault_root = encrypted_root(state)?;
    if !manifest_path(&vault_root).exists() {
        return Err(CmdError::new(
            "missing",
            "Missing: no encrypted vault at the default location",
        ));
    }
    let mut session = CliSession::connect("desktop-encrypted")?;
    session.open_encrypted_vault(&vault_root.display().to_string(), passphrase)?;
    let seed = load_manifest(&vault_root);
    state.workspace = Some(Workspace {
        session,
        kind: VaultKind::Encrypted,
        vault_root,
        seed,
    });
    Ok(())
}

fn validate_passphrase(passphrase: &str) -> CmdResult<()> {
    let len = passphrase.chars().count();
    if !(8..=256).contains(&len) {
        return Err(CmdError::invalid(
            "Invalid: passphrase must be 8–256 characters",
        ));
    }
    Ok(())
}

/// Bounded identifier check for ids the frontend passes back (project, run,
/// source…). Core still validates existence and scope.
pub fn bounded_id(raw: &str) -> CmdResult<&str> {
    let ok = !raw.is_empty()
        && raw.len() <= 128
        && raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.' | '@'));
    if ok {
        Ok(raw)
    } else {
        Err(CmdError::invalid("Invalid: malformed identifier"))
    }
}

pub fn bounded_text(raw: &str, max: usize) -> CmdResult<String> {
    let text = raw.trim();
    if text.is_empty() || text.chars().count() > max {
        return Err(CmdError::invalid(format!(
            "Invalid: text must be 1–{max} characters"
        )));
    }
    Ok(text.to_owned())
}
