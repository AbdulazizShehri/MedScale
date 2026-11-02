//! Rust-owned Core host for the Tauri desktop.
//!
//! The frontend never holds a session, vault path, capability or Core request.
//! It calls named commands; this module owns the `CliSession`s, the vault
//! location and the subject context, and returns typed DTOs or a typed error.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
        // Core's own reason for an invalid argument tells the person what to
        // fix (for example "bindings are alias=snapshot_id pairs"); the
        // generic status string does not.
        let message = match &err {
            AuthorityError::InvalidArgument { message } if !message.trim().is_empty() => {
                format!("Invalid: {}", message.trim())
            }
            _ => medscale_desktop_vm::project_workspace::status_message(&err).to_owned(),
        };
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
    pub model_download: Arc<Mutex<crate::audio_model_manager::DownloadCoordinator>>,
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

/// A persisted synthetic seed is complete only when every bundled fixture
/// was promoted exactly once. Imported FHIR rows may be present in addition.
fn complete_synthetic_seed(seed: &SeedManifest) -> bool {
    let distinct_subjects: std::collections::HashSet<_> = seed.subjects.iter().collect();
    if distinct_subjects.len() != seed.subjects.len() {
        return false;
    }
    let bundled: std::collections::HashSet<_> = seed_plan()
        .into_iter()
        .flat_map(|(subject, fixtures)| {
            fixtures
                .into_iter()
                .map(move |fixture| (subject, fixture.file))
        })
        .collect();
    if seed.sources.iter().any(|source| {
        !source.fixture.starts_with("imported:")
            && !bundled.contains(&(source.subject_ref.as_str(), source.fixture.as_str()))
    }) {
        return false;
    }
    for (subject, fixtures) in seed_plan() {
        if !seed.subjects.iter().any(|existing| existing == subject) {
            return false;
        }
        for fixture in fixtures {
            let Ok(resource) = serde_json::from_slice::<serde_json::Value>(fixture.bytes) else {
                return false;
            };
            let Some(expected_type) = resource
                .get("resourceType")
                .and_then(|value| value.as_str())
            else {
                return false;
            };
            let matching: Vec<_> = seed
                .sources
                .iter()
                .filter(|source| source.subject_ref == subject && source.fixture == fixture.file)
                .collect();
            if matching.len() != 1
                || matching[0].source_id.trim().is_empty()
                || matching[0].resource_type != expected_type
                || matching[0].claim_kind != fixture.claim_kind
            {
                return false;
            }
        }
    }
    true
}

/// Check the actual persisted Core ledger before trusting a complete sibling
/// seed roster. Read-only: never migrate/rewrite existing patient or Core data.
fn complete_synthetic_core(vault_root: &Path, seed: &SeedManifest) -> bool {
    use medscale_contracts::objects::{ClinicalAssertion, Proposal};
    use rusqlite::{Connection, OpenFlags};

    let file = vault_root.join("meta.sqlite3");
    if !std::fs::symlink_metadata(&file).is_ok_and(|m| m.is_file() && !m.file_type().is_symlink()) {
        return false;
    }
    let Ok(db) = Connection::open_with_flags(&file, OpenFlags::SQLITE_OPEN_READ_ONLY) else {
        return false;
    };
    let Ok(mut stmt) = db.prepare("SELECT object_class, body_json FROM authority_objects") else {
        return false;
    };
    let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
    else {
        return false;
    };
    let mut proposals = Vec::<Proposal>::new();
    let mut assertions = Vec::<ClinicalAssertion>::new();
    for row in rows {
        let Ok((kind, body)) = row else {
            return false;
        };
        match kind.as_str() {
            "proposal" => {
                let Ok(p) = serde_json::from_str(&body) else {
                    return false;
                };
                proposals.push(p);
            }
            "assertion" => {
                let Ok(a) = serde_json::from_str(&body) else {
                    return false;
                };
                assertions.push(a);
            }
            _ => {}
        }
    }
    for (subject, fixtures) in seed_plan() {
        for fixture in fixtures {
            let Some(source) = seed
                .sources
                .iter()
                .find(|s| s.subject_ref == subject && s.fixture == fixture.file)
            else {
                return false;
            };
            let Ok(resource) = serde_json::from_slice::<serde_json::Value>(fixture.bytes) else {
                return false;
            };
            let Some(resource_type) = resource
                .get("resourceType")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
            else {
                return false;
            };
            let payload = serde_json::json!({ "resource": resource });
            // Core deliberately deduplicates identical FHIR source bytes.
            // An explicitly recorded user import can promote those same
            // bytes again for this subject without replaying the seed.
            let imported_count = seed
                .sources
                .iter()
                .filter(|s| {
                    s.fixture.starts_with("imported:")
                        && s.subject_ref == subject
                        && s.source_id == source.source_id
                        && s.resource_type == resource_type
                        && s.claim_kind == fixture.claim_kind
                })
                .count();
            let Some(expected_promotions) = imported_count.checked_add(1) else {
                return false;
            };
            let matches: Vec<_> = proposals
                .iter()
                .filter(|p| {
                    p.subject_ref
                        .as_ref()
                        .is_some_and(|s| s.as_str() == subject)
                        && p.claim_kind == fixture.claim_kind
                        && p.payload == payload
                        && p.evidence_refs
                            .iter()
                            .any(|e| e.as_str() == source.source_id.as_str())
                })
                .collect();
            if matches.len() != expected_promotions {
                return false;
            }
            for matched in matches {
                let proposal_id = &matched.header.id;
                if assertions
                    .iter()
                    .filter(|a| {
                        a.promoted_from_proposal_id.as_ref() == Some(proposal_id)
                            && a.subject_ref.as_str() == subject
                            && a.claim_kind == fixture.claim_kind
                            && a.payload == payload
                    })
                    .count()
                    != 1
                {
                    return false;
                }
            }
        }
    }

    // A complete bundled seed is not enough to trust arbitrary user-import
    // rows in the sibling roster. Every explicit imported row must have its
    // own durable, promoted Core proposal, even when content deduplication
    // reused an existing source ID. Count by the four stable Core dimensions;
    // filenames are UI labels, never evidence of a persisted promotion.
    let mut import_counts =
        std::collections::BTreeMap::<(String, String, String, String), usize>::new();
    for row in &seed.sources {
        if !row.fixture.starts_with("imported:") {
            continue;
        }
        if row.source_id.trim().is_empty()
            || row.subject_ref.trim().is_empty()
            || row.claim_kind.trim().is_empty()
            || row.resource_type.trim().is_empty()
            || !seed.subjects.iter().any(|s| s == &row.subject_ref)
        {
            return false;
        }
        let key = (
            row.source_id.clone(),
            row.subject_ref.clone(),
            row.claim_kind.clone(),
            row.resource_type.clone(),
        );
        let Some(next) = import_counts.get(&key).copied().unwrap_or(0).checked_add(1) else {
            return false;
        };
        import_counts.insert(key, next);
    }
    for ((source_id, subject, claim_kind, resource_type), imported_count) in import_counts {
        // Bundled seed sources are independently verified by the fixture loop
        // above. A valid user import may deduplicate to the same source ID.
        let seeded_count = seed
            .sources
            .iter()
            .filter(|s| {
                !s.fixture.starts_with("imported:")
                    && s.source_id == source_id
                    && s.subject_ref == subject
                    && s.claim_kind == claim_kind
                    && s.resource_type == resource_type
            })
            .count();
        let Some(expected) = imported_count.checked_add(seeded_count) else {
            return false;
        };
        let matches: Vec<_> = proposals
            .iter()
            .filter(|p| {
                p.subject_ref
                    .as_ref()
                    .is_some_and(|s| s.as_str() == subject)
                    && p.claim_kind == claim_kind
                    && p.evidence_refs.iter().any(|e| e.as_str() == source_id)
                    && p.payload
                        .get("resource")
                        .and_then(|r| r.get("resourceType"))
                        .and_then(serde_json::Value::as_str)
                        == Some(resource_type.as_str())
            })
            .collect();
        if matches.len() != expected {
            return false;
        }
        for p in matches {
            if assertions
                .iter()
                .filter(|a| {
                    a.promoted_from_proposal_id.as_ref() == Some(&p.header.id)
                        && a.subject_ref.as_str() == subject
                        && a.claim_kind == claim_kind
                        && a.payload == p.payload
                })
                .count()
                != 1
            {
                return false;
            }
        }
    }
    true
}

/// Fail closed before Core opens an existing synthetic vault. A missing,
/// truncated or partial manifest after a prior Core write is NOT a license
/// to replay fixture promotions: Core proposal creation is not idempotent.
fn preflight_synthetic_seed(vault_root: &Path) -> CmdResult<SeedManifest> {
    let vault_has_entries = match std::fs::read_dir(vault_root) {
        Ok(mut entries) => entries
            .next()
            .transpose()
            .map_err(|_| CmdError::unavailable("Unavailable: synthetic vault inspection failed"))?
            .is_some(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => {
            return Err(CmdError::unavailable(
                "Unavailable: synthetic vault inspection failed",
            ));
        }
    };
    match std::fs::read(manifest_path(vault_root)) {
        Ok(bytes) => {
            let seed: SeedManifest = serde_json::from_slice(&bytes).map_err(|_| {
                CmdError::new("corrupt", "Corrupt: synthetic seed manifest is invalid")
            })?;
            if !vault_has_entries
                || !complete_synthetic_seed(&seed)
                || !complete_synthetic_core(vault_root, &seed)
            {
                return Err(CmdError::new(
                    "corrupt",
                    "Corrupt: synthetic seed state is incomplete; refusing replay",
                ));
            }
            Ok(seed)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !vault_has_entries => {
            Ok(SeedManifest::default())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(CmdError::new(
            "corrupt",
            "Corrupt: synthetic seed manifest missing; refusing replay",
        )),
        Err(_) => Err(CmdError::unavailable(
            "Unavailable: synthetic seed manifest cannot be inspected",
        )),
    }
}

/// Replace `path` with `bytes` without exposing a truncated destination.
/// Each attempt uses an exclusive, per-writer temporary file. A fixed
/// `<file>.tmp` name can be truncated by a second writer, invalidating the
/// first writer's rename; create_new prevents that collision. This does not
/// replace the workspace's separate single-writer/locking requirement.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind, Write};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);
    let name = path
        .file_name()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "atomic write needs a filename"))?;
    let mut opened = None;
    for _ in 0..16 {
        let mut temp_name = name.to_os_string();
        temp_name.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let candidate = path.with_file_name(temp_name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                opened = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    let (tmp, mut file) = opened.ok_or_else(|| {
        Error::new(ErrorKind::AlreadyExists, "no unused atomic temporary filename")
    })?;
    let write_result = file.write_all(bytes).and_then(|_| file.sync_all());
    drop(file);
    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&tmp);
        return Err(error);
    }
    if let Err(error) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(error);
    }
    sync_parent_dir(path)
}

/// Persist the rename itself on Unix. Windows has no directory handle sync
/// in std; NTFS journals the rename metadata.
#[cfg(unix)]
fn sync_parent_dir(path: &Path) -> std::io::Result<()> {
    match path.parent() {
        Some(dir) => std::fs::File::open(dir)?.sync_all(),
        None => Ok(()),
    }
}

#[cfg(not(unix))]
fn sync_parent_dir(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

fn save_manifest(vault_root: &Path, manifest: &SeedManifest) -> CmdResult<()> {
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|_| CmdError::new("internal", "Internal: seed manifest"))?;
    write_atomic(&manifest_path(vault_root), &bytes)
        .map_err(|_| CmdError::unavailable("Unavailable: seed manifest not writable"))
}

fn pack_dirs_path(vault_root: &Path) -> PathBuf {
    let mut name = vault_root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "workspace".to_owned());
    name.push_str(".desktop-packs.json");
    vault_root.with_file_name(name)
}

/// Local folders of Packs this desktop admitted into a vault, by Pack id.
/// Core re-verifies the folder against the admitted manifest on every run.
pub fn load_pack_dirs(vault_root: &Path) -> std::collections::BTreeMap<String, String> {
    std::fs::read(pack_dirs_path(vault_root))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save_pack_dirs(
    vault_root: &Path,
    dirs: &std::collections::BTreeMap<String, String>,
) -> CmdResult<()> {
    let bytes = serde_json::to_vec_pretty(dirs)
        .map_err(|_| CmdError::new("internal", "Internal: pack folders"))?;
    write_atomic(&pack_dirs_path(vault_root), &bytes)
        .map_err(|_| CmdError::unavailable("Unavailable: pack folders not writable"))
}

fn asr_pack_path(vault_root: &Path) -> PathBuf {
    let mut name = vault_root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "workspace".to_owned());
    name.push_str(".desktop-asr.json");
    vault_root.with_file_name(name)
}

/// The Audio Pack folder this desktop installed for a vault (Spec 106).
pub fn load_asr_pack(vault_root: &Path) -> Option<String> {
    std::fs::read(asr_pack_path(vault_root))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|v| v["dir"].as_str().map(str::to_owned))
}

pub fn save_asr_pack(vault_root: &Path, dir: &str) -> CmdResult<()> {
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({ "dir": dir }))
        .map_err(|_| CmdError::new("internal", "Internal: audio pack record"))?;
    write_atomic(&asr_pack_path(vault_root), &bytes)
        .map_err(|_| CmdError::unavailable("Unavailable: audio pack record not writable"))
}

/// Reinstalls the remembered Audio Pack engine after the vault opens, so
/// local speech recognition resumes after a restart. Core's Pack registry is
/// per session, so the folder is admitted again first (Core re-verifies the
/// manifest, digests and signature); a failure leaves the route unavailable.
fn restore_asr(workspace: &mut Workspace) {
    if let Some(dir) = load_asr_pack(&workspace.vault_root)
        && workspace
            .session
            .packs_install_local(&dir)
            .is_ok_and(|r| r.admitted)
    {
        let _ = workspace.session.use_local_asr_pack(&dir);
    }
}

/// Ingests and promotes the synthetic fixtures through Core, once per vault.
pub fn seed_synthetic(workspace: &mut Workspace, data_dir: &Path) -> CmdResult<()> {
    if !workspace.seed.subjects.is_empty() || !workspace.seed.sources.is_empty() {
        if !complete_synthetic_seed(&workspace.seed) {
            return Err(CmdError::new(
                "corrupt",
                "Corrupt: synthetic seed state is incomplete; refusing replay",
            ));
        }
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

/// Claim kind for an imported FHIR resource type (as in the bundled fixtures).
fn claim_kind_for(resource_type: &str) -> &'static str {
    match resource_type {
        "Patient" => "patient",
        "Observation" => "observation",
        "Condition" => "condition",
        "MedicationRequest" | "MedicationStatement" => "medication",
        _ => "other",
    }
}

/// Largest FHIR file the desktop imports (1 MiB).
pub const FHIR_IMPORT_MAX_BYTES: u64 = 1 << 20;

/// Reads a regular file of 1..=`max` bytes. The bound applies to the bytes
/// actually read, not to earlier metadata, so a file that grows after a
/// check cannot exceed it.
pub fn read_bounded(path: &Path, max: u64) -> CmdResult<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path)
        .map_err(|_| CmdError::new("missing", "Missing: file not found"))?;
    if !file.metadata().is_ok_and(|m| m.is_file()) {
        return Err(CmdError::invalid("Invalid: not a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| CmdError::unavailable("Unavailable: file not readable"))?;
    if bytes.is_empty() || bytes.len() as u64 > max {
        return Err(CmdError::invalid(format!(
            "Invalid: file must be 1 to {max} bytes"
        )));
    }
    Ok(bytes)
}

/// Ingests and promotes one synthetic FHIR R4 JSON file through Core for a
/// subject, and records it in the vault's manifest. Synthetic vaults only:
/// Core admits no clinical ingest into encrypted vaults yet.
pub fn import_fhir(
    workspace: &mut Workspace,
    path: &Path,
    subject: &str,
) -> CmdResult<SeededSource> {
    if workspace.kind != VaultKind::Synthetic {
        return Err(CmdError::new(
            "denied",
            "Denied: clinical ingest is admitted into the synthetic workspace only",
        ));
    }
    let bytes = read_bounded(path, FHIR_IMPORT_MAX_BYTES)?;
    let resource: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| CmdError::new("corrupt", "Corrupt: file is not JSON"))?;
    let resource_type = resource
        .get("resourceType")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| CmdError::invalid("Invalid: not a FHIR resource (no resourceType)"))?
        .to_owned();
    let claim_kind = claim_kind_for(&resource_type);
    let source_id = workspace.session.ingest_fhir_bytes(bytes)?;
    workspace.session.promote_fixture_as_assertion(
        source_id.clone(),
        subject,
        claim_kind,
        resource,
    )?;
    let file = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "resource.json".to_owned());
    let source = SeededSource {
        source_id: source_id.as_str().to_owned(),
        fixture: format!("imported:{file}"),
        resource_type,
        claim_kind: claim_kind.to_owned(),
        subject_ref: subject.to_owned(),
    };
    workspace.seed.sources.push(source.clone());
    if !workspace.seed.subjects.iter().any(|s| s == subject) {
        workspace.seed.subjects.push(subject.to_owned());
    }
    save_manifest(&workspace.vault_root, &workspace.seed)?;
    Ok(source)
}

pub fn open_synthetic(state: &mut State) -> CmdResult<()> {
    let data_dir = state.data_dir()?.to_path_buf();
    let vault_root = data_dir.join("synthetic-workspace");
    let seed = preflight_synthetic_seed(&vault_root)?;
    std::fs::create_dir_all(&vault_root)
        .map_err(|_| CmdError::unavailable("Unavailable: vault folder not writable"))?;
    let mut session = CliSession::connect("desktop-synthetic")?;
    // Synthetic workspace: offline fixtures replace the network, ASR and
    // durable pseudonym keys. Each is labelled as a fixture in the interface.
    session.use_offline_browse_fixture();
    session.use_fixture_asr();
    session.use_in_memory_privacy_keys();
    session.open_synthetic_vault(&vault_root.display().to_string())?;
    let mut workspace = Workspace {
        session,
        kind: VaultKind::Synthetic,
        vault_root,
        seed,
    };
    seed_synthetic(&mut workspace, &data_dir)?;
    restore_asr(&mut workspace);
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
    // The sibling .desktop-seed.json is a UI roster, not encrypted Core
    // material. Stale files must never trigger an attempted vault unlock.
    if !vault_root.join("vault_header.json").is_file() {
        return Err(CmdError::new(
            "missing",
            "Missing: no encrypted vault at the default location",
        ));
    }
    if !vault_root.join("meta.sealed").is_file() {
        return Err(CmdError::new(
            "corrupt",
            "Corrupt: encrypted vault metadata is missing; original files were not changed",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_does_not_clobber_an_existing_legacy_temp_file() {
        let dir = std::env::temp_dir().join(format!(
            "medscale-atomic-legacy-temp-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("synthetic test dir");
        let destination = dir.join("fictional-sidecar.json");
        let legacy_temp = dir.join("fictional-sidecar.json.tmp");
        std::fs::write(&legacy_temp, b"untouched legacy sentinel").expect("legacy temp");
        write_atomic(&destination, b"{\"synthetic\":true}").expect("atomic replace");
        assert_eq!(
            std::fs::read(&destination).expect("read destination"),
            b"{\"synthetic\":true}"
        );
        assert_eq!(
            std::fs::read(&legacy_temp).expect("read legacy temp"),
            b"untouched legacy sentinel",
            "a pre-existing temporary file must not be truncated or renamed"
        );
        std::fs::remove_dir_all(&dir).expect("synthetic-only cleanup");
    }

    #[test]
    fn failed_atomic_replace_preserves_destination_and_removes_its_temp() {
        let dir = std::env::temp_dir().join(format!(
            "medscale-atomic-failed-rename-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let destination = dir.join("fictional-sidecar.json");
        std::fs::create_dir_all(&destination).expect("directory target");
        let sentinel = destination.join("synthetic-sentinel.txt");
        std::fs::write(&sentinel, b"unchanged").expect("sentinel");

        let error = write_atomic(&destination, b"{\"synthetic\":true}")
            .expect_err("a regular file must not replace an existing directory");
        assert_ne!(error.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(std::fs::read(&sentinel).expect("read sentinel"), b"unchanged");
        let leaked = std::fs::read_dir(&dir)
            .expect("list temporary parent")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leaked, 0, "failed rename must remove its own temp");
        std::fs::remove_dir_all(&dir).expect("synthetic-only cleanup");
    }

    #[test]
    fn sidecar_records_are_replaced_atomically_without_leftover_temp() {
        let dir = std::env::temp_dir().join(format!(
            "medscale-atomic-sidecar-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let vault_root = dir.join("synthetic-workspace");
        std::fs::create_dir_all(&vault_root).expect("synthetic test dir");
        let mut manifest = full_seed_manifest_for_test();
        save_manifest(&vault_root, &manifest).expect("first manifest");
        manifest.sources.truncate(1);
        save_manifest(&vault_root, &manifest).expect("replacing manifest");
        let reread: SeedManifest =
            serde_json::from_slice(&std::fs::read(manifest_path(&vault_root)).expect("read"))
                .expect("manifest stays valid JSON after replacement");
        assert_eq!(reread.sources.len(), 1);
        save_asr_pack(&vault_root, "synthetic-pack-dir").expect("asr record");
        assert_eq!(
            load_asr_pack(&vault_root).as_deref(),
            Some("synthetic-pack-dir")
        );
        let leftovers = std::fs::read_dir(&dir)
            .expect("list")
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0, "no temporary sidecar files remain");
        std::fs::remove_dir_all(&dir).expect("synthetic-only cleanup");
    }

    fn full_seed_manifest_for_test() -> SeedManifest {
        let mut manifest = SeedManifest::default();
        for (subject, fixtures) in seed_plan() {
            manifest.subjects.push(subject.to_owned());
            for fixture in fixtures {
                let resource: serde_json::Value = serde_json::from_slice(fixture.bytes).unwrap();
                manifest.sources.push(SeededSource {
                    source_id: "synthetic-test-source".to_owned(),
                    fixture: fixture.file.to_owned(),
                    resource_type: resource["resourceType"].as_str().unwrap().to_owned(),
                    claim_kind: fixture.claim_kind.to_owned(),
                    subject_ref: subject.to_owned(),
                });
            }
        }
        manifest
    }

    #[test]
    fn synthetic_seed_preflight_rejects_partial_or_missing_manifest_without_replay() {
        let root = std::env::temp_dir().join(format!(
            "medscale-seed-preflight-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(preflight_synthetic_seed(&root).unwrap().subjects.is_empty());
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("synthetic-core-placeholder"), b"synthetic-only").unwrap();
        assert!(preflight_synthetic_seed(&root).is_err());

        let complete = full_seed_manifest_for_test();
        let mut partial = complete.clone();
        partial.sources.pop();
        std::fs::write(manifest_path(&root), serde_json::to_vec(&partial).unwrap()).unwrap();
        assert!(preflight_synthetic_seed(&root).is_err());

        std::fs::write(manifest_path(&root), b"{invalid-json").unwrap();
        assert!(preflight_synthetic_seed(&root).is_err());

        std::fs::write(manifest_path(&root), serde_json::to_vec(&complete).unwrap()).unwrap();
        assert!(complete_synthetic_seed(&complete));
        // A forged complete roster cannot substitute for actual persisted Core rows.
        assert!(preflight_synthetic_seed(&root).is_err());

        let mut duplicate = complete.clone();
        duplicate.sources.push(complete.sources[0].clone());
        assert!(!complete_synthetic_seed(&duplicate));
        let mut nonbundled = complete.clone();
        nonbundled.sources.push(SeededSource {
            source_id: "fictional-unbundled-source".to_owned(),
            fixture: "unknown-extra-fixture.json".to_owned(),
            resource_type: "Patient".to_owned(),
            claim_kind: "patient".to_owned(),
            subject_ref: complete.subjects[0].clone(),
        });
        assert!(
            !complete_synthetic_seed(&nonbundled),
            "a non-imported extra manifest row is never a bundled fixture"
        );
        let mut imported = complete;
        imported
            .subjects
            .push("synthetic-subject-imported".to_owned());
        imported.sources.push(SeededSource {
            source_id: "synthetic-import".to_owned(),
            fixture: "imported:synthetic-fhir.json".to_owned(),
            resource_type: "Patient".to_owned(),
            claim_kind: "patient".to_owned(),
            subject_ref: "synthetic-subject-imported".to_owned(),
        });
        assert!(complete_synthetic_seed(&imported));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn interrupted_core_promotion_must_not_replay_or_duplicate_on_synthetic_restart() {
        use medscale_storage::SqliteMetaStore;

        fn counts(root: &Path) -> (usize, usize) {
            let store = SqliteMetaStore::open(root).expect("synthetic-only metadata");
            let rows = store
                .list_authority_objects()
                .expect("durable authority objects");
            (
                rows.iter().filter(|r| r.object_class == "proposal").count(),
                rows.iter()
                    .filter(|r| r.object_class == "assertion")
                    .count(),
            )
        }

        let data_dir = std::env::temp_dir().join(format!(
            "medscale-synthetic-interruption-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&data_dir).expect("synthetic-only data root");
        let vault_root = data_dir.join("synthetic-workspace");

        // Exercise real Core ingest/promotion, not a placeholder in the vault.
        // A crash can occur after a durable proposal/assertion but before the
        // sibling desktop-seed.json is written by seed_synthetic.
        let (subject, fixtures) = seed_plan().into_iter().next().expect("seed plan");
        let fixture = fixtures
            .into_iter()
            .next()
            .expect("first synthetic fixture");
        let resource: serde_json::Value =
            serde_json::from_slice(fixture.bytes).expect("bundled FHIR fixture");
        let resource_type = resource["resourceType"]
            .as_str()
            .expect("FHIR resourceType")
            .to_owned();
        let mut session = CliSession::connect("desktop-synthetic").expect("Core session");
        session.use_offline_browse_fixture();
        session.use_fixture_asr();
        session.use_in_memory_privacy_keys();
        session
            .open_synthetic_vault(&vault_root.display().to_string())
            .expect("Core opens synthetic vault");
        let source_id = session
            .ingest_fhir_bytes(fixture.bytes.to_vec())
            .expect("Core ingest");
        session
            .promote_fixture_as_assertion(source_id.clone(), subject, fixture.claim_kind, resource)
            .expect("Core persisted one synthetic proposal/assertion");
        drop(session);

        let before = counts(&vault_root);
        assert_eq!(before, (1, 1), "actual durable proposal/assertion proof");

        // No sibling manifest: reopening must refuse before starting Core.
        let mut restarted = State {
            data_dir: Some(data_dir.clone()),
            ..State::default()
        };
        let missing = open_synthetic(&mut restarted).expect_err("missing seed must fail closed");
        assert_eq!(missing.kind, "corrupt");
        assert!(restarted.workspace.is_none());
        assert_eq!(
            counts(&vault_root),
            before,
            "no replay after missing manifest"
        );

        // A parseable one-fixture manifest cannot authorize replay either.
        let partial = SeedManifest {
            subjects: vec![subject.to_owned()],
            sources: vec![SeededSource {
                source_id: source_id.as_str().to_owned(),
                fixture: fixture.file.to_owned(),
                resource_type,
                claim_kind: fixture.claim_kind.to_owned(),
                subject_ref: subject.to_owned(),
            }],
        };
        save_manifest(&vault_root, &partial).expect("write deliberately partial fixture roster");
        let invalid = open_synthetic(&mut restarted).expect_err("partial seed must fail closed");
        assert_eq!(invalid.kind, "corrupt");
        assert!(restarted.workspace.is_none());
        assert_eq!(
            counts(&vault_root),
            before,
            "no duplicate proposals or assertions"
        );
        save_manifest(&vault_root, &full_seed_manifest_for_test())
            .expect("write forged but structurally complete roster");
        let forged = open_synthetic(&mut restarted)
            .expect_err("complete JSON without complete Core must be refused");
        assert_eq!(forged.kind, "corrupt");
        assert_eq!(
            counts(&vault_root),
            before,
            "forged roster must not replay or change durable Core rows"
        );
        std::fs::remove_dir_all(&data_dir).expect("cleanup synthetic test files");
    }

    #[test]
    fn complete_core_seed_reopens_with_unchanged_durable_counts() {
        use medscale_storage::SqliteMetaStore;
        let dir = std::env::temp_dir().join(format!(
            "medscale-full-core-seed-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join("synthetic-workspace");
        let mut state = State {
            data_dir: Some(dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut state).expect("real Core synthetic seeding");
        let manifest = state.workspace.as_ref().unwrap().seed.clone();
        assert!(complete_synthetic_seed(&manifest));
        drop(state);
        let read_counts = || {
            let db = SqliteMetaStore::open(&root).expect("Core synthetic metadata");
            let rows = db.list_authority_objects().unwrap();
            (
                rows.iter().filter(|r| r.object_class == "proposal").count(),
                rows.iter()
                    .filter(|r| r.object_class == "assertion")
                    .count(),
            )
        };
        let before = read_counts();
        assert_eq!(before, (12, 12));
        assert!(complete_synthetic_core(&root, &manifest));
        let mut second = State {
            data_dir: Some(dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut second).expect("complete real Core must reopen");
        assert_eq!(
            read_counts(),
            before,
            "no extra fixture promotions on reopen"
        );
        drop(second);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn same_fixture_reimport_must_not_make_complete_synthetic_core_unopenable() {
        use medscale_storage::SqliteMetaStore;

        let dir = std::env::temp_dir().join(format!(
            "medscale-reimport-synthetic-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let vault_root = dir.join("synthetic-workspace");
        let mut state = State {
            data_dir: Some(dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut state).expect("real Core seed");
        let (subject, fixture) = seed_plan()
            .into_iter()
            .next()
            .and_then(|(subject, fixtures)| {
                fixtures
                    .into_iter()
                    .next()
                    .map(|fixture| (subject, fixture))
            })
            .expect("bundled fictional FHIR");
        let seeded_source_id = state
            .workspace
            .as_ref()
            .expect("synthetic Core")
            .seed
            .sources
            .iter()
            .find(|src| src.subject_ref == subject && src.fixture == fixture.file)
            .expect("seeded row")
            .source_id
            .clone();
        let file = dir.join("same-content-user-import.json");
        std::fs::write(&file, fixture.bytes).expect("write synthetic duplicate");
        let imported = import_fhir(
            state.workspace.as_mut().expect("synthetic workspace"),
            &file,
            subject,
        )
        .expect("same-content import must remain allowed");
        assert_eq!(
            imported.source_id, seeded_source_id,
            "Core content digest deduplicates the original source id"
        );
        assert!(
            state
                .workspace
                .as_ref()
                .expect("workspace")
                .seed
                .sources
                .iter()
                .any(|row| row.fixture == "imported:same-content-user-import.json")
        );
        drop(state);

        let counts = || {
            let db = SqliteMetaStore::open(&vault_root).expect("synthetic Core metadata");
            let rows = db.list_authority_objects().expect("durable rows");
            (
                rows.iter().filter(|r| r.object_class == "proposal").count(),
                rows.iter()
                    .filter(|r| r.object_class == "assertion")
                    .count(),
            )
        };
        let before = counts();
        assert_eq!(before, (13, 13), "12 seeds + one legitimate reimport");
        let mut reopened = State {
            data_dir: Some(dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut reopened)
            .expect("the actual seed plus an imported duplicate must not be mistaken for replay");
        assert_eq!(counts(), before, "reopen must not duplicate anything");
        drop(reopened);
        std::fs::remove_dir_all(&dir).expect("clean synthetic test data");
    }

    #[test]
    fn forged_imported_manifest_row_without_core_promotion_is_rejected() {
        let data_dir = std::env::temp_dir().join(format!(
            "medscale-forged-import-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let vault_root = data_dir.join("synthetic-workspace");
        let mut state = State {
            data_dir: Some(data_dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut state).expect("real 12/12 synthetic Core");
        let mut manifest = state.workspace.as_ref().expect("workspace").seed.clone();
        assert!(complete_synthetic_core(&vault_root, &manifest));
        manifest.sources.push(SeededSource {
            source_id: "forged-import-with-no-core-source".to_owned(),
            fixture: "imported:fake-patient.json".to_owned(),
            resource_type: "Patient".to_owned(),
            claim_kind: "patient".to_owned(),
            subject_ref: manifest.subjects[0].clone(),
        });
        assert!(
            complete_synthetic_seed(&manifest),
            "the 12 seed fixtures remain valid"
        );
        assert!(
            !complete_synthetic_core(&vault_root, &manifest),
            "a fabricated imported FHIR roster row has no matching Core promotion"
        );
        drop(state);
        std::fs::remove_dir_all(&data_dir).expect("synthetic-only cleanup");
    }

    #[test]
    fn novel_imported_fhir_has_attested_core_promotion_after_restart() {
        let data_dir = std::env::temp_dir().join(format!(
            "medscale-novel-import-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let vault_root = data_dir.join("synthetic-workspace");
        let mut state = State {
            data_dir: Some(data_dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut state).expect("real Core synthetic seed");
        let (subject, fixtures) = seed_plan().into_iter().next().expect("fixtures");
        let original = fixtures.into_iter().next().expect("bundled patient");
        let mut resource: serde_json::Value =
            serde_json::from_slice(original.bytes).expect("fictional FHIR JSON");
        resource["id"] = serde_json::json!("synthetic-novel-import");
        let file = data_dir.join("unique-fictional-patient.json");
        std::fs::write(&file, serde_json::to_vec(&resource).expect("JSON"))
            .expect("write disposable fictional import");
        let imported = import_fhir(state.workspace.as_mut().expect("workspace"), &file, subject)
            .expect("valid novel FHIR import through real Core");
        assert_eq!(imported.fixture, "imported:unique-fictional-patient.json");
        let manifest = state.workspace.as_ref().expect("workspace").seed.clone();
        assert!(complete_synthetic_core(&vault_root, &manifest));
        drop(state);
        let mut reopened = State {
            data_dir: Some(data_dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut reopened).expect("novel fictional import survives reopen");
        drop(reopened);
        std::fs::remove_dir_all(&data_dir).expect("synthetic-only cleanup");
    }

    #[test]
    fn invalid_arguments_carry_cores_reason() {
        let err = CmdError::from(AuthorityError::InvalidArgument {
            message: "bindings are alias=snapshot_id pairs".to_owned(),
        });
        assert_eq!(err.kind, "invalid");
        assert_eq!(err.message, "Invalid: bindings are alias=snapshot_id pairs");
        // Without a reason the generic status message stays.
        let empty = CmdError::from(AuthorityError::InvalidArgument {
            message: " ".to_owned(),
        });
        assert_eq!(
            empty.message,
            medscale_desktop_vm::project_workspace::status_message(
                &AuthorityError::InvalidArgument {
                    message: " ".to_owned()
                }
            )
        );
    }
}
