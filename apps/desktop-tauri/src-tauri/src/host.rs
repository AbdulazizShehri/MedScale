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
    /// Stable machine reason for states the interface must tell apart (Spec
    /// 108 step 2). Omitted from the DTO when absent, so existing consumers
    /// that read only `kind` and `message` are unaffected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
}

/// Stable reasons for synthetic workspace failures (Spec 108 step 2). A read
/// or lock failure is never reported as corruption.
pub mod seed_reason {
    pub const MANIFEST_MISSING: &str = "seed_manifest_missing";
    pub const MANIFEST_INVALID: &str = "seed_manifest_invalid";
    pub const MANIFEST_INCOMPLETE: &str = "seed_manifest_incomplete";
    pub const CORE_LEDGER_INCOMPLETE: &str = "core_ledger_incomplete";
    pub const IO_FAILURE: &str = "workspace_io_failure";
    pub const BUSY: &str = "workspace_busy";
    pub const WRITER_ACTIVE: &str = "writer_active";
    /// Preflight only: the vault folder is missing, empty or not a folder.
    pub const VAULT_MISSING: &str = "seed_vault_missing";
    /// Preflight only: the ledger file exists but cannot be decoded.
    pub const LEDGER_UNDECODABLE: &str = "seed_ledger_undecodable";
    /// Preflight only: Core was interrupted mid-save (hot rollback journal);
    /// a read-only check cannot roll it back.
    pub const LEDGER_INTERRUPTED: &str = "seed_ledger_interrupted";
    /// Any Core-originated integrity failure. Makes no claim about whether
    /// anything was written before the failure.
    pub const CORE_INTEGRITY: &str = "core_integrity_error";
    /// Automatic first-seed recovery found something it cannot prove is
    /// only the bundled sample; nothing was changed.
    pub const RECOVERY_REFUSED: &str = "seed_recovery_refused";
    /// Pack or ASR records from an earlier workspace exist without a
    /// complete manifest; a new sample is not started over them.
    pub const STALE_SIDECAR: &str = "seed_stale_sidecar";
}

impl CmdError {
    pub fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            reason: None,
        }
    }
    pub fn with_reason(mut self, reason: &'static str) -> Self {
        self.reason = Some(reason);
        self
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
        match &err {
            // Core reports a held vault writer lock as this invalid argument
            // (medscale-core ingest_ops.rs). It is a busy workspace, not bad
            // input and not corruption.
            AuthorityError::InvalidArgument { message } if message.trim() == "WriterHeld" => {
                Self::unavailable("Unavailable: another MedScale process is using this workspace")
                    .with_reason(seed_reason::WRITER_ACTIVE)
            }
            AuthorityError::LeaseHeld { .. } | AuthorityError::AlreadyHeld { .. } => {
                Self::new(kind, message).with_reason(seed_reason::WRITER_ACTIVE)
            }
            AuthorityError::Corrupt { .. } | AuthorityError::DigestMismatch => {
                Self::new(kind, message).with_reason(seed_reason::CORE_INTEGRITY)
            }
            _ => Self::new(kind, message),
        }
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
    /// Spec 108 step 4: written before Core runs on a first seed and
    /// replaced atomically by the complete manifest, so a pending workspace
    /// never reaches the UI. Omitted when false (byte-identical to before).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pending: bool,
    /// Binds the one discard folder name to this pending seed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discard_nonce: Option<String>,
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
    if seed.pending {
        return false;
    }
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

/// Outcome of reading the persisted Core ledger. `Unreadable` and `Corrupt`
/// are kept apart from `Incomplete` so a busy or unreadable workspace is
/// never presented as a damaged one (Spec 108 step 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LedgerCheck {
    Complete,
    Incomplete,
    /// Another connection holds a lock beyond SQLite's default busy wait.
    Busy,
    /// Real I/O or permission failure; the data itself was not judged.
    Unreadable,
    /// Hot rollback journal from an interrupted Core write.
    Interrupted,
    Corrupt,
}

/// Classify a SQLite failure while reading the ledger read-only.
fn classify_ledger_error(error: &rusqlite::Error) -> LedgerCheck {
    use rusqlite::ErrorCode;
    // Missing table or column: Core never finished creating its schema.
    // With modern_sqlite, prepare errors may carry an offset and arrive as
    // SqlInputError rather than SqliteFailure.
    // Display covers both variants without depending on that feature.
    let text = error.to_string();
    if text.contains("no such table") || text.contains("no such column") {
        return LedgerCheck::Incomplete;
    }
    match error {
        // Row shape errors: the data is present but not what Core writes.
        rusqlite::Error::InvalidColumnType(..)
        | rusqlite::Error::FromSqlConversionFailure(..)
        | rusqlite::Error::Utf8Error(..) => return LedgerCheck::Corrupt,
        _ => {}
    }
    match error.sqlite_error_code() {
        Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => LedgerCheck::Busy,
        Some(ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase) => LedgerCheck::Corrupt,
        Some(ErrorCode::ReadOnly) => LedgerCheck::Interrupted,
        _ => LedgerCheck::Unreadable,
    }
}

/// Bool view for tests that only need "fully proven".
#[cfg(test)]
fn complete_synthetic_core(vault_root: &Path, seed: &SeedManifest) -> bool {
    synthetic_core_ledger(vault_root, seed) == LedgerCheck::Complete
}

/// Check the actual persisted Core ledger before trusting a complete sibling
/// seed roster. Read-only: never migrate/rewrite existing patient or Core data.
fn synthetic_core_ledger(vault_root: &Path, seed: &SeedManifest) -> LedgerCheck {
    use medscale_contracts::objects::{ClinicalAssertion, Proposal};
    use rusqlite::{Connection, OpenFlags};

    let file = vault_root.join("meta.sqlite3");
    match std::fs::symlink_metadata(&file) {
        Ok(m) if m.is_file() && !m.file_type().is_symlink() => {}
        // A directory or symlink is not a ledger Core wrote: not proven.
        Ok(_) => return LedgerCheck::Incomplete,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return LedgerCheck::Incomplete,
        Err(_) => return LedgerCheck::Unreadable,
    }
    let db = match Connection::open_with_flags(&file, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(db) => db,
        Err(e) => return classify_ledger_error(&e),
    };
    // rusqlite's default 5 s busy timeout already absorbs short writer locks.
    let mut stmt = match db.prepare("SELECT object_class, body_json FROM authority_objects") {
        Ok(stmt) => stmt,
        Err(e) => return classify_ledger_error(&e),
    };
    let rows = match stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))) {
        Ok(rows) => rows,
        Err(e) => return classify_ledger_error(&e),
    };
    let mut proposals = Vec::<Proposal>::new();
    let mut assertions = Vec::<ClinicalAssertion>::new();
    for row in rows {
        let (kind, body) = match row {
            Ok(row) => row,
            Err(e) => return classify_ledger_error(&e),
        };
        match kind.as_str() {
            "proposal" => {
                let Ok(p) = serde_json::from_str(&body) else {
                    return LedgerCheck::Corrupt;
                };
                proposals.push(p);
            }
            "assertion" => {
                let Ok(a) = serde_json::from_str(&body) else {
                    return LedgerCheck::Corrupt;
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
                return LedgerCheck::Incomplete;
            };
            let Ok(resource) = serde_json::from_slice::<serde_json::Value>(fixture.bytes) else {
                return LedgerCheck::Incomplete;
            };
            let Some(resource_type) = resource
                .get("resourceType")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
            else {
                return LedgerCheck::Incomplete;
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
                return LedgerCheck::Incomplete;
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
                return LedgerCheck::Incomplete;
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
                    return LedgerCheck::Incomplete;
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
            return LedgerCheck::Incomplete;
        }
        let key = (
            row.source_id.clone(),
            row.subject_ref.clone(),
            row.claim_kind.clone(),
            row.resource_type.clone(),
        );
        let Some(next) = import_counts.get(&key).copied().unwrap_or(0).checked_add(1) else {
            return LedgerCheck::Incomplete;
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
            return LedgerCheck::Incomplete;
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
            return LedgerCheck::Incomplete;
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
                return LedgerCheck::Incomplete;
            }
        }
    }
    LedgerCheck::Complete
}

/// Result of the read-only "only bundled fixture data" proof (Spec 108
/// step 4 v3). Anything not recognised as part of a (possibly partial)
/// first seed is user activity or unknown, and is never auto-cleaned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SeedOnlyCheck {
    /// Every byte on disk is accounted for by the bundled seed plan.
    FixtureOnly,
    /// Something other than the bundled seed exists.
    NotFixtureOnly,
    /// The vault could not be read fully (busy, I/O, hot journal): unproven.
    Unproven,
}

/// SHA-256 hex digests of the distinct bundled seed fixtures.
fn seed_fixture_digests() -> std::collections::BTreeSet<String> {
    seed_plan()
        .into_iter()
        .flat_map(|(_, fixtures)| fixtures)
        .map(|f| medscale_contracts::objects::DigestSha256::of(f.bytes).to_hex())
        .collect()
}

/// Folder contents a partial first seed can leave: the Core metadata and
/// writer-lock databases and content-addressed fixture blobs (inventory of
/// medscale-storage vault.rs, writer_lock.rs, blob.rs, sqlite_meta.rs).
fn seed_only_files(
    vault_root: &Path,
    digests: &std::collections::BTreeSet<String>,
) -> SeedOnlyCheck {
    let entries = match std::fs::read_dir(vault_root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return SeedOnlyCheck::FixtureOnly,
        Err(_) => return SeedOnlyCheck::Unproven,
    };
    for entry in entries {
        let Ok(entry) = entry else {
            return SeedOnlyCheck::Unproven;
        };
        let Ok(kind) = entry.file_type() else {
            return SeedOnlyCheck::Unproven;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        match name.as_str() {
            // A hot rollback journal means an interrupted write: unproven
            // until a rollback copy is inspected (v3 M5, not yet built).
            "meta.sqlite3-journal" => return SeedOnlyCheck::Unproven,
            "meta.sqlite3" | "writer.lock.sqlite3" | "writer.lock.sqlite3-journal"
                if kind.is_file() => {}
            "blobs" if kind.is_dir() => {
                let Ok(blobs) = std::fs::read_dir(entry.path()) else {
                    return SeedOnlyCheck::Unproven;
                };
                for blob in blobs {
                    let Ok(blob) = blob else {
                        return SeedOnlyCheck::Unproven;
                    };
                    let Ok(blob_kind) = blob.file_type() else {
                        return SeedOnlyCheck::Unproven;
                    };
                    let blob_name = blob.file_name().to_string_lossy().into_owned();
                    if blob_name == "quarantine" && blob_kind.is_dir() {
                        match std::fs::read_dir(blob.path()) {
                            Ok(mut q) => {
                                if q.next().is_some() {
                                    return SeedOnlyCheck::NotFixtureOnly;
                                }
                                continue;
                            }
                            Err(_) => return SeedOnlyCheck::Unproven,
                        }
                    }
                    // `.<hex>.tmp` is an interrupted blob write (blob.rs).
                    let hex = blob_name
                        .strip_prefix('.')
                        .and_then(|n| n.strip_suffix(".tmp"))
                        .unwrap_or(&blob_name);
                    if !(blob_kind.is_file() && digests.contains(hex)) {
                        return SeedOnlyCheck::NotFixtureOnly;
                    }
                }
            }
            _ => return SeedOnlyCheck::NotFixtureOnly,
        }
    }
    SeedOnlyCheck::FixtureOnly
}

/// Read-only proof that a synthetic vault holds nothing but a (possibly
/// partial) first seed of the bundled fixtures. Never writes; any doubt is
/// `Unproven` or `NotFixtureOnly`.
///
/// Scope: only `vault_root`. Sibling sidecars are the caller's check.
/// Precondition before this may authorize deletion (Goldfish review M1,
/// step 4 increment 2): the caller holds the instance lock and Core's
/// `WriterLock` for `vault_root` from before this call until the rename,
/// and the reads run in one transaction. Used by first-seed recovery, which
/// holds that lock (`hold_and_prove_fixture_only`).
pub(crate) fn seed_only_vault(vault_root: &Path) -> SeedOnlyCheck {
    use medscale_contracts::objects::{
        ActionAuditKind, ActionAuditRecord, AuthorityScopeId, ClinicalAssertion, DigestSha256,
        IdentityAssertion, MedicalTime, ObjectHeader, OpaqueId, ProducerKind, Proposal, RealmId,
        SourceRecord, TimePrecision,
    };
    use rusqlite::{Connection, OpenFlags};
    use serde_json::Value;
    use std::collections::{BTreeMap, BTreeSet};
    // CliSession's fixed realm and scope (medscale-core cli_session.rs).
    const SEED_REALM: &str = "cli-realm";
    const SEED_SCOPE: &str = "cli-scope";

    match std::fs::symlink_metadata(vault_root) {
        Ok(m) if m.file_type().is_symlink() || !m.is_dir() => {
            return SeedOnlyCheck::NotFixtureOnly;
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return SeedOnlyCheck::FixtureOnly,
        Err(_) => return SeedOnlyCheck::Unproven,
    }
    let digests = seed_fixture_digests();
    match seed_only_files(vault_root, &digests) {
        SeedOnlyCheck::FixtureOnly => {}
        other => return other,
    }
    let meta = vault_root.join("meta.sqlite3");
    match std::fs::symlink_metadata(&meta) {
        Ok(m) if m.is_file() => {}
        Ok(_) => return SeedOnlyCheck::NotFixtureOnly,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return SeedOnlyCheck::FixtureOnly,
        Err(_) => return SeedOnlyCheck::Unproven,
    }
    let Ok(db) = Connection::open_with_flags(&meta, OpenFlags::SQLITE_OPEN_READ_ONLY) else {
        return SeedOnlyCheck::Unproven;
    };
    // One read transaction: every query below sees the same snapshot
    // (review M1). Dropping the connection ends it.
    if db.execute_batch("BEGIN DEFERRED").is_err() {
        return SeedOnlyCheck::Unproven;
    }
    let query_strings = |sql: &str| -> Option<Vec<Vec<Option<String>>>> {
        let mut stmt = db.prepare(sql).ok()?;
        let width = stmt.column_count();
        let rows = stmt
            .query_map([], |r| {
                (0..width)
                    .map(|i| r.get::<_, Option<String>>(i))
                    .collect::<Result<Vec<_>, _>>()
            })
            .ok()?;
        rows.collect::<Result<Vec<_>, _>>().ok()
    };

    // 1. Only the seed's tables may hold rows.
    let Some(tables) = query_strings("SELECT name FROM sqlite_master WHERE type = 'table'") else {
        return SeedOnlyCheck::Unproven;
    };
    const SEED_TABLES: [&str; 4] = [
        "migration_journal",
        "store_state",
        "sources",
        "authority_objects",
    ];
    for row in &tables {
        let Some(Some(name)) = row.first() else {
            return SeedOnlyCheck::Unproven;
        };
        if SEED_TABLES.contains(&name.as_str()) || name.starts_with("sqlite_") {
            continue;
        }
        let quoted = name.replace('"', "\"\"");
        let Ok(count) = db.query_row(&format!("SELECT COUNT(*) FROM \"{quoted}\""), [], |r| {
            r.get::<_, i64>(0)
        }) else {
            return SeedOnlyCheck::Unproven;
        };
        if count != 0 {
            return SeedOnlyCheck::NotFixtureOnly;
        }
    }
    let has_table = |t: &str| {
        tables
            .iter()
            .any(|r| r.first() == Some(&Some(t.to_owned())))
    };

    // 2. store_state holds only the shared id counter.
    if has_table("store_state") {
        let Some(keys) = query_strings("SELECT key FROM store_state") else {
            return SeedOnlyCheck::Unproven;
        };
        if keys
            .iter()
            .any(|k| k.first() != Some(&Some("next_seq".to_owned())))
        {
            return SeedOnlyCheck::NotFixtureOnly;
        }
    }

    // 3. sources rows reference fixture digests only, in the CLI scope.
    let mut table_sources: BTreeMap<String, String> = BTreeMap::new();
    if has_table("sources") {
        let Some(rows) = query_strings(
            "SELECT source_id, digest_hex, realm_id, authority_scope_id FROM sources",
        ) else {
            return SeedOnlyCheck::Unproven;
        };
        let mut seen = BTreeSet::new();
        for r in rows {
            let (Some(Some(id)), Some(Some(hex)), Some(Some(realm)), Some(Some(scope))) =
                (r.first(), r.get(1), r.get(2), r.get(3))
            else {
                return SeedOnlyCheck::NotFixtureOnly;
            };
            if !digests.contains(hex)
                || !seen.insert(hex.clone())
                || realm != SEED_REALM
                || scope != SEED_SCOPE
            {
                return SeedOnlyCheck::NotFixtureOnly;
            }
            table_sources.insert(id.clone(), hex.clone());
        }
    }

    // 4. authority_objects: only the five seed classes, decoded with the
    // contract types (all `deny_unknown_fields`) and checked against the
    // exact values Core's seed path writes (review M2).
    if !has_table("authority_objects") {
        return SeedOnlyCheck::FixtureOnly;
    }
    let Some(rows) = query_strings(
        "SELECT object_id, object_class, body_json, content_digest_hex, realm_id, authority_scope_id \
         FROM authority_objects",
    ) else {
        return SeedOnlyCheck::Unproven;
    };
    type ObjRow = (String, String, Option<String>);
    let mut by_class: BTreeMap<String, Vec<ObjRow>> = BTreeMap::new();
    for r in rows {
        let (
            Some(Some(id)),
            Some(Some(class)),
            Some(Some(body)),
            Some(Some(realm)),
            Some(Some(scope)),
        ) = (r.first(), r.get(1), r.get(2), r.get(4), r.get(5))
        else {
            return SeedOnlyCheck::NotFixtureOnly;
        };
        if realm != SEED_REALM || scope != SEED_SCOPE {
            return SeedOnlyCheck::NotFixtureOnly;
        }
        let digest = r.get(3).cloned().flatten();
        by_class
            .entry(class.clone())
            .or_default()
            .push((id.clone(), body.clone(), digest));
    }
    if by_class
        .keys()
        .any(|c| !["source", "identity", "audit", "proposal", "assertion"].contains(&c.as_str()))
    {
        return SeedOnlyCheck::NotFixtureOnly;
    }
    let empty = Vec::new();
    let class = |c: &str| by_class.get(c).unwrap_or(&empty);
    let header_ok = |h: &ObjectHeader, id: &str| {
        h.id.as_str() == id
            && h.schema_version == medscale_contracts::AUTHORITY_SCHEMA_VERSION
            && h.realm_id == RealmId::new(SEED_REALM)
            && h.authority_scope_id == AuthorityScopeId::new(SEED_SCOPE)
    };

    // Sources: one per distinct fixture digest, bytes emptied by the
    // durable sync, FHIR media type, no acquisition time.
    let mut source_digest: BTreeMap<String, String> = BTreeMap::new();
    for (id, body, column_digest) in class("source") {
        let Ok(src) = serde_json::from_str::<SourceRecord>(body) else {
            return SeedOnlyCheck::NotFixtureOnly;
        };
        let hex = src.content_digest.to_hex();
        if !header_ok(&src.header, id)
            || column_digest.as_deref() != Some(hex.as_str())
            || !digests.contains(&hex)
            || source_digest.values().any(|d| *d == hex)
            || !src.bytes.is_empty()
            || src.media_type != "application/fhir+json"
            || src.acquired_at.is_some()
            || !src
                .provenance_note
                .as_deref()
                .is_some_and(|n| n.starts_with("fhir "))
            || table_sources.get(id).is_some_and(|t| *t != hex)
        {
            return SeedOnlyCheck::NotFixtureOnly;
        }
        source_digest.insert(id.clone(), hex);
    }
    // Identity: only the one Patient identifier of patient-golden.json.
    let golden_hex = seed_plan()
        .into_iter()
        .flat_map(|(_, fixtures)| fixtures)
        .find(|f| f.file == "patient-golden.json")
        .map(|f| DigestSha256::of(f.bytes).to_hex());
    if class("identity").len() > 1 {
        return SeedOnlyCheck::NotFixtureOnly;
    }
    for (id, body, _) in class("identity") {
        let Ok(ident) = serde_json::from_str::<IdentityAssertion>(body) else {
            return SeedOnlyCheck::NotFixtureOnly;
        };
        let evidence_is_golden = ident.evidence_refs.len() == 1
            && golden_hex
                .as_ref()
                .is_some_and(|g| source_digest.get(ident.evidence_refs[0].as_str()) == Some(g));
        if !header_ok(&ident.header, id)
            || ident.subject_id.as_str() != "subject:http://hospital.example/mrn:P-004"
            || ident.identifier_system != "http://hospital.example/mrn"
            || ident.identifier_value != "P-004"
            || ident.confidence.is_some()
            || !evidence_is_golden
        {
            return SeedOnlyCheck::NotFixtureOnly;
        }
    }
    // Proposals: each equals one bundled plan entry, evidenced by the source
    // holding exactly that fixture's bytes; at most one per plan entry.
    let mut plan_used = BTreeSet::new();
    let mut proposals: BTreeMap<String, Proposal> = BTreeMap::new();
    for (id, body, _) in class("proposal") {
        let Ok(proposal) = serde_json::from_str::<Proposal>(body) else {
            return SeedOnlyCheck::NotFixtureOnly;
        };
        let Some(subject) = proposal.subject_ref.as_ref().map(OpaqueId::as_str) else {
            return SeedOnlyCheck::NotFixtureOnly;
        };
        if !header_ok(&proposal.header, id)
            || proposal.producer != ProducerKind::Rule
            || proposal.confidence.is_some()
            || proposal.evidence_refs.len() != 1
        {
            return SeedOnlyCheck::NotFixtureOnly;
        }
        let Some(source_hex) = source_digest.get(proposal.evidence_refs[0].as_str()) else {
            return SeedOnlyCheck::NotFixtureOnly;
        };
        let matched = seed_plan()
            .into_iter()
            .find_map(|(plan_subject, fixtures)| {
                fixtures.into_iter().find_map(|f| {
                    let resource: Value = serde_json::from_slice(f.bytes).ok()?;
                    (plan_subject == subject
                        && f.claim_kind == proposal.claim_kind
                        && proposal.payload == serde_json::json!({ "resource": resource })
                        && *source_hex == DigestSha256::of(f.bytes).to_hex())
                    .then_some((plan_subject, f.file))
                })
            });
        let Some(entry) = matched else {
            return SeedOnlyCheck::NotFixtureOnly;
        };
        if !plan_used.insert(entry) {
            return SeedOnlyCheck::NotFixtureOnly;
        }
        proposals.insert(id.clone(), proposal);
    }
    // Assertions: at most one per proposal, an exact promotion of it.
    let recorded = MedicalTime::new("1970-01-01T00:00:00Z", TimePrecision::Instant, false);
    let mut assertion_of: BTreeMap<String, String> = BTreeMap::new();
    for (id, body, _) in class("assertion") {
        let Ok(assertion) = serde_json::from_str::<ClinicalAssertion>(body) else {
            return SeedOnlyCheck::NotFixtureOnly;
        };
        let Some(from) = assertion
            .promoted_from_proposal_id
            .as_ref()
            .map(|p| p.as_str().to_owned())
        else {
            return SeedOnlyCheck::NotFixtureOnly;
        };
        let Some(proposal) = proposals.get(&from) else {
            return SeedOnlyCheck::NotFixtureOnly;
        };
        if !header_ok(&assertion.header, id)
            || Some(&assertion.subject_ref) != proposal.subject_ref.as_ref()
            || assertion.claim_kind != proposal.claim_kind
            || assertion.payload != proposal.payload
            || assertion.authorized_by.as_str() != "cli-operator"
            || assertion.effective_time.is_some()
            || assertion.recorded_time != recorded
            || assertion_of.values().any(|p| *p == from)
        {
            return SeedOnlyCheck::NotFixtureOnly;
        }
        assertion_of.insert(id.clone(), from);
    }
    // Audits: one ingest audit per source and one promotion audit per
    // assertion, exactly as Core's seed path writes them.
    let mut ingest_audited = BTreeSet::new();
    let mut promote_audited = BTreeSet::new();
    for (id, body, _) in class("audit") {
        let Ok(audit) = serde_json::from_str::<ActionAuditRecord>(body) else {
            return SeedOnlyCheck::NotFixtureOnly;
        };
        if !header_ok(&audit.header, id)
            || audit.kind != ActionAuditKind::Audit
            || audit.effect_state.is_some()
            || audit.detail.is_some()
        {
            return SeedOnlyCheck::NotFixtureOnly;
        }
        let targets: Vec<&str> = audit.target_refs.iter().map(OpaqueId::as_str).collect();
        let ok = match audit.action.as_str() {
            "ingest_fhir_synthetic" => {
                targets.len() == 1
                    && audit.actor.as_str() == "ingest"
                    && source_digest.get(targets[0]).is_some_and(|hex| {
                        audit
                            .payload_digest
                            .as_ref()
                            .map(DigestSha256::to_hex)
                            .as_ref()
                            == Some(hex)
                    })
                    && ingest_audited.insert(targets[0].to_owned())
            }
            "promote_proposal" => {
                targets.len() == 2
                    && audit.actor.as_str() == "cli-operator"
                    && audit.payload_digest.is_none()
                    && assertion_of.get(targets[1]).map(String::as_str) == Some(targets[0])
                    && promote_audited.insert(targets[1].to_owned())
            }
            _ => false,
        };
        if !ok {
            return SeedOnlyCheck::NotFixtureOnly;
        }
    }
    SeedOnlyCheck::FixtureOnly
}

/// What the read-only preflight found (Spec 108 step 4).
#[derive(Debug)]
enum Preflight {
    /// No manifest and an absent or empty folder: a first seed may start.
    Fresh,
    /// A complete manifest proven against the durable Core ledger.
    Complete(SeedManifest),
    /// An exact pending manifest: an interrupted first seed to recover.
    Pending(SeedManifest),
}

/// The exact pending shape: `{pending: true, subjects: [], sources: [],
/// discard_nonce: <32 lowercase hex>}` and no other keys.
fn exact_pending_manifest(bytes: &[u8]) -> Option<SeedManifest> {
    let raw: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let object = raw.as_object()?;
    let keys: std::collections::BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let expected: std::collections::BTreeSet<&str> =
        ["pending", "subjects", "sources", "discard_nonce"]
            .into_iter()
            .collect();
    if keys != expected {
        return None;
    }
    let seed: SeedManifest = serde_json::from_value(raw).ok()?;
    let nonce_ok = seed
        .discard_nonce
        .as_deref()
        .is_some_and(valid_discard_nonce);
    (seed.pending && seed.subjects.is_empty() && seed.sources.is_empty() && nonce_ok)
        .then_some(seed)
}

fn valid_discard_nonce(nonce: &str) -> bool {
    nonce.len() == 32
        && nonce
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn new_discard_nonce() -> String {
    let seed = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    );
    medscale_contracts::objects::DigestSha256::of(seed.as_bytes()).to_hex()[..32].to_owned()
}

/// Fail closed before Core opens an existing synthetic vault. A missing,
/// truncated or partial manifest after a prior Core write is NOT a license
/// to replay fixture promotions: Core proposal creation is not idempotent.
fn preflight_synthetic_seed(vault_root: &Path) -> CmdResult<Preflight> {
    let inspection_failed = || {
        CmdError::unavailable("Unavailable: synthetic vault inspection failed")
            .with_reason(seed_reason::IO_FAILURE)
    };
    let incomplete = |reason: &'static str| {
        CmdError::new(
            "corrupt",
            "Corrupt: synthetic seed state is incomplete; refusing replay",
        )
        .with_reason(reason)
    };
    let vault_has_entries = match std::fs::read_dir(vault_root) {
        Ok(mut entries) => entries
            .next()
            .transpose()
            .map_err(|_| inspection_failed())?
            .is_some(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        // The vault path exists but is not a folder: never a vault Core made.
        Err(error) if error.kind() == std::io::ErrorKind::NotADirectory => {
            return Err(incomplete(seed_reason::VAULT_MISSING));
        }
        Err(_) => return Err(inspection_failed()),
    };
    match std::fs::read(manifest_path(vault_root)) {
        Ok(bytes) => {
            if serde_json::from_slice::<serde_json::Value>(&bytes)
                .ok()
                .is_some_and(|v| v.get("pending").is_some())
            {
                // Any pending manifest that is not the exact pending shape
                // is not something MedScale wrote: never recover from it.
                return exact_pending_manifest(&bytes).map(Preflight::Pending).ok_or_else(|| {
                    CmdError::new(
                        "corrupt",
                        "Corrupt: the sample workspace setup record is not recognised; nothing was changed",
                    )
                    .with_reason(seed_reason::RECOVERY_REFUSED)
                });
            }
            let seed: SeedManifest = serde_json::from_slice(&bytes).map_err(|_| {
                CmdError::new("corrupt", "Corrupt: synthetic seed manifest is invalid")
                    .with_reason(seed_reason::MANIFEST_INVALID)
            })?;
            if !vault_has_entries {
                return Err(incomplete(seed_reason::VAULT_MISSING));
            }
            if !complete_synthetic_seed(&seed) {
                return Err(incomplete(seed_reason::MANIFEST_INCOMPLETE));
            }
            match synthetic_core_ledger(vault_root, &seed) {
                LedgerCheck::Complete => Ok(Preflight::Complete(seed)),
                LedgerCheck::Incomplete => Err(incomplete(seed_reason::CORE_LEDGER_INCOMPLETE)),
                LedgerCheck::Corrupt => Err(CmdError::new(
                    "corrupt",
                    "Corrupt: synthetic workspace data cannot be decoded; refusing replay",
                )
                .with_reason(seed_reason::LEDGER_UNDECODABLE)),
                LedgerCheck::Interrupted => Err(incomplete(seed_reason::LEDGER_INTERRUPTED)),
                LedgerCheck::Busy => Err(CmdError::unavailable(
                    "Unavailable: the synthetic workspace is busy; wait a moment and try again",
                )
                .with_reason(seed_reason::BUSY)),
                LedgerCheck::Unreadable => Err(CmdError::unavailable(
                    "Unavailable: the synthetic workspace could not be read",
                )
                .with_reason(seed_reason::IO_FAILURE)),
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !vault_has_entries => {
            Ok(Preflight::Fresh)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(CmdError::new(
            "corrupt",
            "Corrupt: synthetic seed manifest missing; refusing replay",
        )
        .with_reason(seed_reason::MANIFEST_MISSING)),
        Err(_) => Err(CmdError::unavailable(
            "Unavailable: synthetic seed manifest cannot be inspected",
        )
        .with_reason(seed_reason::IO_FAILURE)),
    }
}

/// Replace `path` with `bytes` so a crash leaves either the old file or the
/// new one, never a truncated file. Spec 108 (M3): a torn seed manifest is
/// reported as corrupt and blocks reopening a workspace that may hold user
/// imports, so sidecar records must not be written in place.
/// The temporary sibling `write_atomic` writes before renaming; the inverse
/// is `atomic_temp_target`.
fn atomic_temp_path(path: &Path) -> PathBuf {
    let mut tmp_name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    tmp_name.push(".tmp");
    path.with_file_name(tmp_name)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let tmp = atomic_temp_path(path);
    {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
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
    #[cfg(test)]
    if !manifest.pending && crash_requested("during_commit") {
        // The temporary file of the commit exists; the rename never happened.
        let _ = std::fs::write(atomic_temp_path(&manifest_path(vault_root)), &bytes);
        crash_point("during_commit");
    }
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
            )
            .with_reason(seed_reason::MANIFEST_INCOMPLETE));
        }
        return Ok(());
    }
    let staging = data_dir.join("fixture-staging");
    std::fs::create_dir_all(&staging).map_err(|_| CmdError::unavailable("Unavailable: staging"))?;
    let mut manifest = SeedManifest::default();
    let mut promoted = 0_usize;
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
            promoted += 1;
            crash_point(&format!("after_ingest_{promoted}"));
            workspace.session.promote_fixture_as_assertion(
                source_id.clone(),
                subject,
                fixture.claim_kind,
                resource,
            )?;
            crash_point(&format!("after_fixture_{promoted}"));
            e2e_block_point(promoted);
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
    crash_point("before_commit");
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
    if workspace.seed.pending {
        return Err(CmdError::unavailable(
            "Unavailable: the sample workspace is still being prepared",
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

/// Opens (or first-seeds) the synthetic workspace. Returns true when an
/// interrupted first seed was rebuilt (Spec 108 step 4); any folder it
/// deleted was first proven fixture-only under Core's writer lock.
pub fn open_synthetic(state: &mut State) -> CmdResult<bool> {
    let data_dir = state.data_dir()?.to_path_buf();
    let vault_root = data_dir.join("synthetic-workspace");
    let mut recovered = false;
    let seed = match preflight_synthetic_seed(&vault_root)? {
        Preflight::Complete(seed) => seed,
        Preflight::Pending(seed) => {
            recover_interrupted_first_seed(&data_dir, &vault_root, &seed)?;
            recovered = true;
            seed
        }
        Preflight::Fresh => {
            // The pending manifest lives beside the vault; create only the
            // data directory here, never the vault itself.
            std::fs::create_dir_all(&data_dir).map_err(|_| {
                CmdError::unavailable("Unavailable: data folder not writable")
                    .with_reason(seed_reason::IO_FAILURE)
            })?;
            scan_siblings(&data_dir, None)?;
            let pending = SeedManifest {
                pending: true,
                discard_nonce: Some(new_discard_nonce()),
                ..SeedManifest::default()
            };
            save_manifest(&vault_root, &pending)?;
            crash_point("after_pending_write");
            pending
        }
    };
    // Only an absent or empty real folder may receive a new seed.
    if seed.pending {
        match std::fs::symlink_metadata(&vault_root) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {
                let empty = std::fs::read_dir(&vault_root)
                    .map(|mut d| d.next().is_none())
                    .unwrap_or(false);
                if !empty {
                    return Err(recovery_refused());
                }
            }
            _ => return Err(recovery_refused()),
        }
    }
    std::fs::create_dir_all(&vault_root)
        .map_err(|_| CmdError::unavailable("Unavailable: vault folder not writable"))?;
    let mut session = CliSession::connect("desktop-synthetic")?;
    // Synthetic workspace: offline fixtures replace the network, ASR and
    // durable pseudonym keys. Each is labelled as a fixture in the interface.
    session.use_offline_browse_fixture();
    session.use_fixture_asr();
    session.use_in_memory_privacy_keys();
    session.open_synthetic_vault(&vault_root.display().to_string())?;
    crash_point("after_core_open");
    let mut workspace = Workspace {
        session,
        kind: VaultKind::Synthetic,
        vault_root,
        seed,
    };
    seed_synthetic(&mut workspace, &data_dir)?;
    crash_point("after_commit");
    restore_asr(&mut workspace);
    state.workspace = Some(workspace);
    Ok(recovered)
}

fn recovery_refused() -> CmdError {
    CmdError::new(
        "corrupt",
        "Corrupt: the sample workspace could not be rebuilt automatically; nothing was changed",
    )
    .with_reason(seed_reason::RECOVERY_REFUSED)
}

const VAULT_NAME: &str = "synthetic-workspace";
const MANIFEST_NAME: &str = "synthetic-workspace.desktop-seed.json";
const DISCARD_PREFIX: &str = "synthetic-workspace.discarding-";

/// Inverse of `write_atomic`'s temporary name (`<target>.tmp`).
fn atomic_temp_target(file_name: &str) -> Option<&str> {
    file_name.strip_suffix(".tmp").filter(|t| !t.is_empty())
}

/// Siblings of the synthetic workspace that recovery may touch.
#[derive(Debug, Default)]
struct Siblings {
    discard: Option<PathBuf>,
    manifest_temps: Vec<PathBuf>,
}

/// Classify every data-dir entry whose name mentions the synthetic
/// workspace (ASCII case-insensitive). Unknown entries refuse; nothing is
/// changed here (Spec 108 step 4 v3.2, section 2).
fn scan_siblings(data_dir: &Path, nonce: Option<&str>) -> CmdResult<Siblings> {
    let io = || {
        CmdError::unavailable("Unavailable: synthetic vault inspection failed")
            .with_reason(seed_reason::IO_FAILURE)
    };
    let mut found = Siblings::default();
    let entries = match std::fs::read_dir(data_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(found),
        Err(_) => return Err(io()),
    };
    for entry in entries {
        let entry = entry.map_err(|_| io())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.to_ascii_lowercase().contains(VAULT_NAME) {
            continue;
        }
        let kind = entry.file_type().map_err(|_| io())?;
        let real_dir = kind.is_dir() && !kind.is_symlink();
        if (name == VAULT_NAME && real_dir) || (name == MANIFEST_NAME && kind.is_file()) {
            continue;
        }
        if let Some(n) = nonce
            && real_dir
            && name.strip_prefix(DISCARD_PREFIX) == Some(n)
        {
            found.discard = Some(entry.path());
            continue;
        }
        if kind.is_file() && atomic_temp_target(&name) == Some(MANIFEST_NAME) {
            // A temp that parses as a complete manifest means the commit may
            // have happened; a torn or pending temp never replaced anything.
            let complete = std::fs::read(entry.path())
                .ok()
                .and_then(|b| serde_json::from_slice::<SeedManifest>(&b).ok())
                .is_some_and(|m| !m.pending);
            if complete {
                return Err(recovery_refused());
            }
            found.manifest_temps.push(entry.path());
            continue;
        }
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".desktop-packs.json") || lower.ends_with(".desktop-asr.json") {
            return Err(CmdError::new(
                "corrupt",
                "Corrupt: settings from an earlier sample workspace remain; nothing was changed",
            )
            .with_reason(seed_reason::STALE_SIDECAR));
        }
        return Err(recovery_refused());
    }
    Ok(found)
}

/// Retry transient Windows sharing failures (5, 32, 33) for up to 3 s.
fn with_fs_retry(mut op: impl FnMut() -> std::io::Result<()>) -> CmdResult<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        match op() {
            Ok(()) => return Ok(()),
            Err(e)
                if cfg!(windows)
                    && matches!(e.raw_os_error(), Some(5 | 32 | 33))
                    && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(_) => {
                return Err(CmdError::unavailable(
                    "Unavailable: the sample workspace folder is in use; try again in a moment",
                )
                .with_reason(seed_reason::IO_FAILURE));
            }
        }
    }
}

/// Hold Core's writer lock on an existing real folder and prove it holds
/// only bundled fixture data. The guard is returned so the caller decides
/// when exclusion ends.
fn hold_and_prove_fixture_only(dir: &Path) -> CmdResult<medscale_core::VaultWriterGuard> {
    use medscale_core::{VaultWriterGuard, VaultWriterGuardError};
    match std::fs::symlink_metadata(dir) {
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {}
        _ => return Err(recovery_refused()),
    }
    let guard = VaultWriterGuard::try_hold(dir).map_err(|e| match e {
        VaultWriterGuardError::Held => {
            CmdError::unavailable("Unavailable: another MedScale process is using this workspace")
                .with_reason(seed_reason::WRITER_ACTIVE)
        }
        VaultWriterGuardError::Refused => recovery_refused(),
        VaultWriterGuardError::Io => {
            CmdError::unavailable("Unavailable: the synthetic workspace could not be read")
                .with_reason(seed_reason::IO_FAILURE)
        }
    })?;
    match seed_only_vault(dir) {
        SeedOnlyCheck::FixtureOnly => Ok(guard),
        SeedOnlyCheck::NotFixtureOnly => Err(recovery_refused()),
        SeedOnlyCheck::Unproven if dir.join("meta.sqlite3-journal").exists() => Err(CmdError::new(
            "corrupt",
            "Corrupt: synthetic seed state is incomplete; refusing replay",
        )
        .with_reason(seed_reason::LEDGER_INTERRUPTED)),
        SeedOnlyCheck::Unproven => Err(CmdError::unavailable(
            "Unavailable: the synthetic workspace could not be read",
        )
        .with_reason(seed_reason::IO_FAILURE)),
    }
}

/// Delete a folder only after proving it fixture-only under the lock.
fn delete_proven_fixture_only(dir: &Path) -> CmdResult<()> {
    let guard = hold_and_prove_fixture_only(dir)?;
    // Windows: SQLite files open without delete sharing, so release first.
    if cfg!(windows) {
        drop(guard);
        return with_fs_retry(|| std::fs::remove_dir_all(dir));
    }
    let result = with_fs_retry(|| std::fs::remove_dir_all(dir));
    drop(guard);
    result
}

/// Spec 108 step 4: recover an interrupted first seed. Every folder that is
/// deleted is first proven fixture-only under Core's writer lock; nothing
/// is ever deleted by name. The pending manifest stays and is reused.
fn recover_interrupted_first_seed(
    data_dir: &Path,
    vault_root: &Path,
    seed: &SeedManifest,
) -> CmdResult<()> {
    let Some(nonce) = seed
        .discard_nonce
        .as_deref()
        .filter(|n| valid_discard_nonce(n))
    else {
        return Err(recovery_refused());
    };
    let siblings = scan_siblings(data_dir, Some(nonce))?;
    // A discard folder left by an interrupted recovery is resumed first.
    if let Some(discard) = &siblings.discard {
        delete_proven_fixture_only(discard)?;
    }
    let discard = data_dir.join(format!("{DISCARD_PREFIX}{nonce}"));
    match std::fs::symlink_metadata(vault_root) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Ok(_) => {
            let guard = hold_and_prove_fixture_only(vault_root)?;
            if cfg!(windows) {
                // Windows: release, rename (fails on any open handle), then
                // re-prove the renamed folder under a fresh lock.
                drop(guard);
                ensure_absent(&discard)?;
                with_fs_retry(|| std::fs::rename(vault_root, &discard))?;
                crash_point("recovery_after_rename");
                if let Err(e) = hold_and_prove_fixture_only(&discard).map(drop) {
                    let _ = with_fs_retry(|| std::fs::rename(&discard, vault_root));
                    return Err(e);
                }
                with_fs_retry(|| std::fs::remove_dir_all(&discard))?;
            } else {
                // Unix: keep the lock through the rename and re-prove under
                // the same guard (a second lock on the same file would be
                // refused by SQLite's in-process locking).
                ensure_absent(&discard)?;
                with_fs_retry(|| std::fs::rename(vault_root, &discard))?;
                crash_point("recovery_after_rename");
                if seed_only_vault(&discard) != SeedOnlyCheck::FixtureOnly {
                    let _ = std::fs::rename(&discard, vault_root);
                    return Err(recovery_refused());
                }
                with_fs_retry(|| std::fs::remove_dir_all(&discard))?;
                drop(guard);
            }
        }
        Err(_) => {
            return Err(
                CmdError::unavailable("Unavailable: synthetic vault inspection failed")
                    .with_reason(seed_reason::IO_FAILURE),
            );
        }
    }
    for temp in &siblings.manifest_temps {
        let _ = std::fs::remove_file(temp);
    }
    Ok(())
}

/// A rename target must not exist: on Unix renaming onto an empty folder
/// would silently replace it.
fn ensure_absent(path: &Path) -> CmdResult<()> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(recovery_refused()),
    }
}

/// Native E2E only (`e2e-webdriver` builds): block the first seed after
/// fixture `n` and write a marker, so the journey can kill the app at a known
/// boundary without Windows Error Reporting holding the instance lock.
#[cfg(feature = "e2e-webdriver")]
fn e2e_block_point(promoted: usize) {
    let wanted = std::env::var("MEDSCALE_E2E_BLOCK_SEED_AFTER")
        .ok()
        .and_then(|v| v.parse::<usize>().ok());
    if wanted == Some(promoted) {
        if let Ok(marker) = std::env::var("MEDSCALE_E2E_SEED_MARKER") {
            let _ = std::fs::write(marker, promoted.to_string());
        }
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }
}

#[cfg(not(feature = "e2e-webdriver"))]
fn e2e_block_point(_promoted: usize) {}

/// Test-only crash hook: exit with a sentinel code at a named boundary.
#[cfg(test)]
fn crash_requested(boundary: &str) -> bool {
    std::env::var("MEDSCALE_TEST_SEED_CRASH").as_deref() == Ok(boundary)
}

#[cfg(test)]
fn crash_point(boundary: &str) {
    if crash_requested(boundary) {
        if let Ok(marker) = std::env::var("MEDSCALE_TEST_SEED_CRASH_MARKER") {
            let _ = std::fs::write(marker, boundary);
        }
        std::process::exit(86);
    }
}

#[cfg(not(test))]
fn crash_point(_boundary: &str) {}

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

    fn sqlite_failure(code: rusqlite::ErrorCode, message: Option<&str>) -> rusqlite::Error {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code,
                extended_code: 0,
            },
            message.map(str::to_owned),
        )
    }

    #[test]
    fn ledger_read_failures_are_not_reported_as_corruption() {
        use rusqlite::ErrorCode;
        let cases = [
            (ErrorCode::DatabaseBusy, None, LedgerCheck::Busy),
            (ErrorCode::DatabaseLocked, None, LedgerCheck::Busy),
            (ErrorCode::CannotOpen, None, LedgerCheck::Unreadable),
            (ErrorCode::PermissionDenied, None, LedgerCheck::Unreadable),
            (ErrorCode::SystemIoFailure, None, LedgerCheck::Unreadable),
            (ErrorCode::ReadOnly, None, LedgerCheck::Interrupted),
            (ErrorCode::DatabaseCorrupt, None, LedgerCheck::Corrupt),
            (ErrorCode::NotADatabase, None, LedgerCheck::Corrupt),
            (
                ErrorCode::Unknown,
                Some("no such table: authority_objects"),
                LedgerCheck::Incomplete,
            ),
        ];
        for (code, message, expected) in cases {
            assert_eq!(
                classify_ledger_error(&sqlite_failure(code, message)),
                expected,
                "{code:?}"
            );
        }
        assert_eq!(
            classify_ledger_error(&rusqlite::Error::InvalidColumnType(
                1,
                "body_json".to_owned(),
                rusqlite::types::Type::Blob
            )),
            LedgerCheck::Corrupt,
            "a non-text row is undecodable data, not an I/O failure"
        );
    }

    #[test]
    fn writer_held_is_reported_as_busy_not_invalid_input() {
        let err = CmdError::from(AuthorityError::InvalidArgument {
            message: "WriterHeld".to_owned(),
        });
        assert_eq!(err.kind, "unavailable");
        assert_eq!(err.reason, Some(seed_reason::WRITER_ACTIVE));
        let other = CmdError::from(AuthorityError::InvalidArgument {
            message: "bindings are alias=snapshot_id pairs".to_owned(),
        });
        assert_eq!(other.kind, "invalid");
        assert_eq!(other.reason, None);
    }

    #[test]
    fn reason_is_omitted_from_the_dto_when_absent() {
        let plain = serde_json::to_value(CmdError::invalid("Invalid: x")).expect("json");
        assert!(plain.get("reason").is_none());
        let tagged = serde_json::to_value(
            CmdError::unavailable("Unavailable: y").with_reason(seed_reason::BUSY),
        )
        .expect("json");
        assert_eq!(tagged["reason"], "workspace_busy");
    }

    #[test]
    fn preflight_reasons_distinguish_invalid_manifest_and_undecodable_ledger() {
        let data_dir = std::env::temp_dir().join(format!(
            "medscale-seed-reasons-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let vault_root = data_dir.join("synthetic-workspace");
        std::fs::create_dir_all(&vault_root).expect("synthetic test dir");
        std::fs::write(
            vault_root.join("synthetic-core-placeholder"),
            b"synthetic-only",
        )
        .expect("entry");

        std::fs::write(manifest_path(&vault_root), b"{invalid-json").expect("manifest");
        let invalid = preflight_synthetic_seed(&vault_root).expect_err("invalid JSON");
        assert_eq!(invalid.kind, "corrupt");
        assert_eq!(invalid.reason, Some(seed_reason::MANIFEST_INVALID));

        save_manifest(&vault_root, &full_seed_manifest_for_test()).expect("manifest");
        let no_ledger = preflight_synthetic_seed(&vault_root).expect_err("no Core ledger");
        assert_eq!(no_ledger.reason, Some(seed_reason::CORE_LEDGER_INCOMPLETE));

        // A file that is not a SQLite database is undecodable data, not a
        // missing ledger and not a read failure.
        std::fs::write(
            vault_root.join("meta.sqlite3"),
            b"synthetic non-database bytes",
        )
        .expect("bad ledger");
        let corrupt = preflight_synthetic_seed(&vault_root).expect_err("undecodable ledger");
        assert_eq!(corrupt.kind, "corrupt");
        assert_eq!(corrupt.reason, Some(seed_reason::LEDGER_UNDECODABLE));
        assert!(
            !corrupt.message.contains(&data_dir.display().to_string()),
            "user-facing errors never include filesystem paths"
        );
        std::fs::remove_dir_all(&data_dir).expect("synthetic-only cleanup");
    }

    fn scratch_data_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "medscale-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    #[test]
    fn empty_real_ledger_file_is_incomplete_not_an_io_failure() {
        let data_dir = scratch_data_dir("empty-ledger");
        let vault_root = data_dir.join("synthetic-workspace");
        std::fs::create_dir_all(&vault_root).expect("dir");
        // A 0-byte file is a valid empty SQLite database with no schema.
        std::fs::write(vault_root.join("meta.sqlite3"), b"").expect("empty ledger");
        save_manifest(&vault_root, &full_seed_manifest_for_test()).expect("manifest");
        let err = preflight_synthetic_seed(&vault_root).expect_err("no schema");
        assert_eq!(err.kind, "corrupt");
        assert_eq!(err.reason, Some(seed_reason::CORE_LEDGER_INCOMPLETE));
        std::fs::remove_dir_all(&data_dir).expect("synthetic-only cleanup");
    }

    #[test]
    fn manifest_without_vault_and_file_in_place_of_vault_are_vault_missing() {
        let data_dir = scratch_data_dir("vault-missing");
        std::fs::create_dir_all(&data_dir).expect("dir");
        let vault_root = data_dir.join("synthetic-workspace");
        save_manifest(&vault_root, &full_seed_manifest_for_test()).expect("manifest");
        let no_vault = preflight_synthetic_seed(&vault_root).expect_err("no vault");
        assert_eq!(no_vault.reason, Some(seed_reason::VAULT_MISSING));
        std::fs::write(
            &vault_root,
            b"synthetic file where the vault folder belongs",
        )
        .expect("file");
        let not_dir = preflight_synthetic_seed(&vault_root).expect_err("not a folder");
        assert_eq!(not_dir.kind, "corrupt");
        assert_eq!(not_dir.reason, Some(seed_reason::VAULT_MISSING));
        std::fs::remove_dir_all(&data_dir).expect("synthetic-only cleanup");
    }

    #[test]
    fn second_session_on_open_synthetic_workspace_is_writer_active() {
        let data_dir = scratch_data_dir("writer-active");
        let mut first = State {
            data_dir: Some(data_dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut first).expect("first synthetic workspace");
        let mut second = State {
            data_dir: Some(data_dir.clone()),
            ..State::default()
        };
        let err = open_synthetic(&mut second).expect_err("Core writer lock is held");
        // Core's writer lock (writer_active) or, if Core also holds the
        // ledger, the read-only preflight (workspace_busy): both mean in use.
        assert_eq!(err.kind, "unavailable");
        assert!(
            matches!(
                err.reason,
                Some(seed_reason::WRITER_ACTIVE) | Some(seed_reason::BUSY)
            ),
            "{:?}",
            err.reason
        );
        assert!(second.workspace.is_none());
        drop(first);
        drop(second);
        std::fs::remove_dir_all(&data_dir).expect("synthetic-only cleanup");
    }

    fn seed_only_scratch(tag: &str) -> (PathBuf, PathBuf) {
        let data_dir = std::env::temp_dir().join(format!(
            "medscale-seedonly-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let vault_root = data_dir.join("synthetic-workspace");
        (data_dir, vault_root)
    }

    fn copy_object_row(
        vault_root: &Path,
        class: &str,
        new_id: &str,
        edit: impl Fn(&mut serde_json::Value),
    ) {
        let db = rusqlite::Connection::open(vault_root.join("meta.sqlite3")).expect("db");
        let (id, body): (String, String) = db
            .query_row(
                "SELECT object_id, body_json FROM authority_objects WHERE object_class = ?1 LIMIT 1",
                [class],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("existing row");
        let mut value: serde_json::Value = serde_json::from_str(&body).expect("json");
        value["header"]["id"] = serde_json::json!(new_id);
        edit(&mut value);
        db.execute(
            "INSERT INTO authority_objects(object_id, object_class, realm_id, authority_scope_id, body_json, content_digest_hex, updated_seq) \
             SELECT ?1, object_class, realm_id, authority_scope_id, ?2, content_digest_hex, updated_seq \
             FROM authority_objects WHERE object_id = ?3",
            rusqlite::params![new_id, value.to_string(), id],
        )
        .expect("crafted row");
    }

    fn seeded_scratch(tag: &str) -> (PathBuf, PathBuf) {
        let (data_dir, vault_root) = seed_only_scratch(tag);
        let mut state = State {
            data_dir: Some(data_dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut state).expect("genuine seed");
        drop(state);
        assert_eq!(seed_only_vault(&vault_root), SeedOnlyCheck::FixtureOnly);
        (data_dir, vault_root)
    }

    #[test]
    fn seed_only_rejects_real_core_user_activity_with_fixture_bytes() {
        // Re-importing the exact bytes of a bundled fixture deduplicates the
        // source (no new blob) but adds a second proposal for the same entry.
        let (data_dir, vault_root) = seed_only_scratch("dup-import");
        let mut state = State {
            data_dir: Some(data_dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut state).expect("seed");
        let (subject, fixtures) = seed_plan().into_iter().next().expect("plan");
        let file = data_dir.join("same-bytes.json");
        std::fs::write(&file, fixtures[0].bytes).expect("write");
        import_fhir(state.workspace.as_mut().expect("workspace"), &file, subject)
            .expect("same-content import");
        drop(state);
        assert_eq!(seed_only_vault(&vault_root), SeedOnlyCheck::NotFixtureOnly);
        std::fs::remove_dir_all(&data_dir).expect("cleanup");

        // The same fixture bytes under the other subject.
        let (data_dir, vault_root) = seed_only_scratch("other-subject");
        let mut state = State {
            data_dir: Some(data_dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut state).expect("seed");
        let plan = seed_plan();
        let file = data_dir.join("cross-subject.json");
        std::fs::write(&file, plan[0].1[0].bytes).expect("write");
        import_fhir(
            state.workspace.as_mut().expect("workspace"),
            &file,
            plan[1].0,
        )
        .expect("cross-subject import");
        drop(state);
        assert_eq!(seed_only_vault(&vault_root), SeedOnlyCheck::NotFixtureOnly);
        std::fs::remove_dir_all(&data_dir).expect("cleanup");

        // A project created through Core.
        let (data_dir, vault_root) = seed_only_scratch("project");
        let mut state = State {
            data_dir: Some(data_dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut state).expect("seed");
        state
            .session()
            .expect("session")
            .project_create("Seed-only probe".to_owned(), None)
            .expect("project");
        drop(state);
        assert_eq!(seed_only_vault(&vault_root), SeedOnlyCheck::NotFixtureOnly);
        std::fs::remove_dir_all(&data_dir).expect("cleanup");
    }

    /// A named vault mutation for the seed-only rejection tables.
    type Craft = (&'static str, fn(&Path));

    /// Edit the first matching authority row in place (no duplicate row), so
    /// each exact-shape check is proven on its own.
    fn edit_object_row(
        vault_root: &Path,
        class: &str,
        like: &str,
        edit: impl Fn(&mut serde_json::Value),
    ) {
        let db = rusqlite::Connection::open(vault_root.join("meta.sqlite3")).expect("db");
        let (id, body): (String, String) = db
            .query_row(
                "SELECT object_id, body_json FROM authority_objects \
                 WHERE object_class = ?1 AND body_json LIKE ?2 LIMIT 1",
                [class, like],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("existing row");
        let mut value: serde_json::Value = serde_json::from_str(&body).expect("json");
        edit(&mut value);
        db.execute(
            "UPDATE authority_objects SET body_json = ?1 WHERE object_id = ?2",
            rusqlite::params![value.to_string(), id],
        )
        .expect("edited row");
    }

    fn edit_sql(vault_root: &Path, sql: &str) {
        let db = rusqlite::Connection::open(vault_root.join("meta.sqlite3")).expect("db");
        db.execute(sql, []).expect("edited column");
    }

    #[test]
    fn seed_only_each_exact_shape_check_is_load_bearing() {
        let cases: [Craft; 9] = [
            ("proposal-extra-field", |v| {
                edit_object_row(v, "proposal", "%", |b| {
                    b["unexpected"] = serde_json::json!(1)
                })
            }),
            ("source-bytes", |v| {
                edit_object_row(v, "source", "%", |b| b["bytes"] = serde_json::json!([1]))
            }),
            ("ingest-audit-digest", |v| {
                edit_object_row(v, "audit", "%ingest_fhir_synthetic%", |b| {
                    b["payload_digest"] = serde_json::Value::Null
                })
            }),
            ("assertion-authorized-by", |v| {
                edit_object_row(v, "assertion", "%", |b| {
                    b["authorized_by"] = serde_json::json!("someone-else")
                })
            }),
            ("identity-subject", |v| {
                edit_object_row(v, "identity", "%", |b| {
                    b["subject_id"] = serde_json::json!("subject:other")
                })
            }),
            ("header-realm", |v| {
                edit_object_row(v, "proposal", "%", |b| {
                    b["header"]["realm_id"] = serde_json::json!("other-realm")
                })
            }),
            ("header-schema-version", |v| {
                edit_object_row(v, "assertion", "%", |b| {
                    b["header"]["schema_version"] = serde_json::json!(9999)
                })
            }),
            ("object-realm-column", |v| {
                edit_sql(
                    v,
                    "UPDATE authority_objects SET realm_id = 'other-realm' WHERE object_id = \
                     (SELECT object_id FROM authority_objects LIMIT 1)",
                )
            }),
            ("sources-realm-column", |v| {
                edit_sql(
                    v,
                    "UPDATE sources SET realm_id = 'other-realm' WHERE source_id = \
                     (SELECT source_id FROM sources LIMIT 1)",
                )
            }),
        ];
        for (tag, craft) in cases {
            let (data_dir, vault_root) = seeded_scratch(tag);
            craft(&vault_root);
            assert_eq!(
                seed_only_vault(&vault_root),
                SeedOnlyCheck::NotFixtureOnly,
                "{tag}"
            );
            std::fs::remove_dir_all(&data_dir).expect("cleanup");
        }
    }

    #[test]
    fn seed_only_rejects_crafted_rows_and_files() {
        let cases: [Craft; 7] = [
            ("unknown-class", |v| {
                let db = rusqlite::Connection::open(v.join("meta.sqlite3")).expect("db");
                db.execute(
                    "UPDATE authority_objects SET object_class = 'amendment' WHERE object_id = \
                     (SELECT object_id FROM authority_objects WHERE object_class = 'audit' LIMIT 1)",
                    [],
                )
                .expect("crafted class");
            }),
            ("second-identity", |v| {
                copy_object_row(v, "identity", "ident-crafted", |_| {})
            }),
            ("forged-audit", |v| {
                copy_object_row(v, "audit", "audit-crafted", |b| {
                    b["action"] = serde_json::json!("packs.evaluate_local");
                })
            }),
            ("extra-field", |v| {
                copy_object_row(v, "proposal", "proposal-crafted", |b| {
                    b["unexpected"] = serde_json::json!(true);
                })
            }),
            ("quarantine", |v| {
                std::fs::write(v.join("blobs").join("quarantine").join("x"), b"q").expect("q")
            }),
            ("tombstone", |v| {
                std::fs::create_dir_all(v.join("blobs").join("tombstone")).expect("t")
            }),
            ("tmp-blob", |v| {
                std::fs::write(
                    v.join("blobs").join(format!(".{}.tmp", "0".repeat(64))),
                    b"x",
                )
                .expect("tmp")
            }),
        ];
        for (tag, craft) in cases {
            let (data_dir, vault_root) = seeded_scratch(tag);
            craft(&vault_root);
            assert_eq!(
                seed_only_vault(&vault_root),
                SeedOnlyCheck::NotFixtureOnly,
                "{tag}"
            );
            std::fs::remove_dir_all(&data_dir).expect("cleanup");
        }
    }

    fn crash_scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "medscale-crash-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("scratch");
        dir
    }

    /// Re-run this test binary as a child that opens the synthetic workspace
    /// and exits with code 86 at `boundary` (no Drop, OS releases handles).
    fn run_crash_child(data_dir: &Path, boundary: &str) {
        let marker = std::env::temp_dir().join(format!(
            "medscale-crash-marker-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let out = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "host::tests::seed_crash_child",
                "--exact",
                "--ignored",
                "--test-threads=1",
                "--nocapture",
            ])
            .env("MEDSCALE_TEST_SEED_CRASH", boundary)
            .env("MEDSCALE_TEST_SEED_CRASH_MARKER", &marker)
            .env("MEDSCALE_TEST_SEED_CRASH_DATA_DIR", data_dir)
            .output()
            .expect("spawn crash child");
        assert_eq!(
            out.status.code(),
            Some(86),
            "{boundary}: child did not stop at the boundary\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            std::fs::read_to_string(&marker).expect("boundary marker"),
            boundary
        );
        let _ = std::fs::remove_file(&marker);
    }

    /// Child half of the crash tests; a no-op unless the parent set it up.
    #[test]
    #[ignore = "re-executed by crash tests only"]
    fn seed_crash_child() {
        let Some(dir) = std::env::var_os("MEDSCALE_TEST_SEED_CRASH_DATA_DIR") else {
            return;
        };
        let mut state = State {
            data_dir: Some(PathBuf::from(dir)),
            ..State::default()
        };
        let _ = open_synthetic(&mut state);
    }

    fn ledger_counts(vault_root: &Path) -> (i64, i64) {
        let db = rusqlite::Connection::open(vault_root.join("meta.sqlite3")).expect("db");
        let count = |class: &str| {
            db.query_row(
                "SELECT COUNT(*) FROM authority_objects WHERE object_class = ?1",
                [class],
                |r| r.get::<_, i64>(0),
            )
            .expect("count")
        };
        (count("proposal"), count("assertion"))
    }

    fn reopen(data_dir: &Path) -> (CmdResult<bool>, State) {
        let mut state = State {
            data_dir: Some(data_dir.to_path_buf()),
            ..State::default()
        };
        let result = open_synthetic(&mut state);
        (result, state)
    }

    fn assert_complete_sample(state: &State, data_dir: &Path) {
        let seed = &state.workspace.as_ref().expect("workspace").seed;
        assert!(!seed.pending, "committed manifest is not pending");
        assert_eq!(seed.subjects.len(), 2);
        assert_eq!(
            ledger_counts(&data_dir.join("synthetic-workspace")),
            (12, 12),
            "exactly the bundled 12 proposals and 12 assertions, no duplicates"
        );
    }

    #[test]
    fn interrupted_first_seed_is_recovered_at_every_boundary() {
        for boundary in [
            "after_pending_write",
            "after_core_open",
            "after_ingest_1",
            "after_fixture_1",
            "after_fixture_11",
            "before_commit",
        ] {
            let data_dir = crash_scratch(boundary);
            run_crash_child(&data_dir, boundary);
            let (result, state) = reopen(&data_dir);
            assert!(result.expect(boundary), "{boundary}: recovered");
            assert_complete_sample(&state, &data_dir);
            drop(state);
            let (again, state) = reopen(&data_dir);
            assert!(!again.expect(boundary), "{boundary}: no second recovery");
            assert_complete_sample(&state, &data_dir);
            drop(state);
            std::fs::remove_dir_all(&data_dir).expect("cleanup");
        }
    }

    #[test]
    fn commit_boundaries_never_discard_a_committed_sample() {
        // The commit's temporary file exists but was never renamed: refuse.
        let data_dir = crash_scratch("during-commit");
        run_crash_child(&data_dir, "during_commit");
        let (result, _state) = reopen(&data_dir);
        let err = result.expect_err("a complete manifest temp may mean the commit happened");
        assert_eq!(err.reason, Some(seed_reason::RECOVERY_REFUSED));
        assert!(
            data_dir.join("synthetic-workspace").exists(),
            "nothing deleted"
        );
        std::fs::remove_dir_all(&data_dir).expect("cleanup");

        // Committed but the UI never received it: opens normally.
        let data_dir = crash_scratch("after-commit");
        run_crash_child(&data_dir, "after_commit");
        let (result, state) = reopen(&data_dir);
        assert!(!result.expect("committed sample opens"));
        assert_complete_sample(&state, &data_dir);
        drop(state);
        std::fs::remove_dir_all(&data_dir).expect("cleanup");
    }

    #[test]
    fn interrupted_recovery_is_resumed_after_the_rename() {
        let data_dir = crash_scratch("resume");
        run_crash_child(&data_dir, "after_fixture_1");
        run_crash_child(&data_dir, "recovery_after_rename");
        assert!(!data_dir.join("synthetic-workspace").exists());
        let (result, state) = reopen(&data_dir);
        assert!(result.expect("resumed recovery"));
        assert_complete_sample(&state, &data_dir);
        drop(state);
        let leftovers = std::fs::read_dir(&data_dir)
            .expect("list")
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains("discarding"))
            .count();
        assert_eq!(leftovers, 0, "the proven discard folder was deleted");
        std::fs::remove_dir_all(&data_dir).expect("cleanup");
    }

    #[test]
    fn recovery_refuses_anything_it_cannot_prove() {
        // Legacy missing manifest after a committed sample: never recovered.
        let data_dir = crash_scratch("legacy");
        let (result, state) = reopen(&data_dir);
        result.expect("first seed");
        drop(state);
        std::fs::remove_file(manifest_path(&data_dir.join("synthetic-workspace")))
            .expect("drop manifest");
        let (result, _state) = reopen(&data_dir);
        assert_eq!(
            result.expect_err("legacy").reason,
            Some(seed_reason::MANIFEST_MISSING)
        );
        assert!(
            data_dir
                .join("synthetic-workspace")
                .join("meta.sqlite3")
                .exists()
        );
        std::fs::remove_dir_all(&data_dir).expect("cleanup");

        // A pending seed whose vault also holds a non-fixture proposal.
        let data_dir = crash_scratch("user-row");
        run_crash_child(&data_dir, "after_fixture_1");
        copy_object_row(
            &data_dir.join("synthetic-workspace"),
            "proposal",
            "proposal-user",
            |b| b["claim_kind"] = serde_json::json!("observation"),
        );
        let (result, _state) = reopen(&data_dir);
        assert_eq!(
            result.expect_err("user row").reason,
            Some(seed_reason::RECOVERY_REFUSED)
        );
        assert!(
            data_dir
                .join("synthetic-workspace")
                .join("meta.sqlite3")
                .exists()
        );
        std::fs::remove_dir_all(&data_dir).expect("cleanup");

        // A pack sidecar beside a pending seed: the UI may have been reached.
        let data_dir = crash_scratch("sidecar");
        run_crash_child(&data_dir, "after_fixture_1");
        std::fs::write(
            data_dir.join("synthetic-workspace.desktop-packs.json"),
            b"{}",
        )
        .expect("sidecar");
        let (result, _state) = reopen(&data_dir);
        assert_eq!(
            result.expect_err("sidecar").reason,
            Some(seed_reason::STALE_SIDECAR)
        );
        std::fs::remove_dir_all(&data_dir).expect("cleanup");

        // A complete-manifest temp beside a pending manifest.
        let data_dir = crash_scratch("complete-temp");
        run_crash_child(&data_dir, "after_fixture_1");
        std::fs::write(
            data_dir.join("synthetic-workspace.desktop-seed.json.tmp"),
            serde_json::to_vec(&full_seed_manifest_for_test()).expect("json"),
        )
        .expect("temp");
        let (result, _state) = reopen(&data_dir);
        assert_eq!(
            result.expect_err("complete temp").reason,
            Some(seed_reason::RECOVERY_REFUSED)
        );
        std::fs::remove_dir_all(&data_dir).expect("cleanup");

        // Another holder of Core's writer lock: refused, then recovered.
        let data_dir = crash_scratch("writer");
        run_crash_child(&data_dir, "after_fixture_1");
        let guard =
            medscale_core::VaultWriterGuard::try_hold(&data_dir.join("synthetic-workspace"))
                .expect("hold");
        let (result, _state) = reopen(&data_dir);
        assert_eq!(
            result.expect_err("writer held").reason,
            Some(seed_reason::WRITER_ACTIVE)
        );
        drop(guard);
        let (result, state) = reopen(&data_dir);
        assert!(result.expect("recovered after release"));
        assert_complete_sample(&state, &data_dir);
        drop(state);
        std::fs::remove_dir_all(&data_dir).expect("cleanup");

        // An orphan discard folder beside a fresh vault is never touched.
        let data_dir = crash_scratch("orphan");
        let orphan = data_dir.join(format!("{DISCARD_PREFIX}{}", "0".repeat(32)));
        std::fs::create_dir_all(&orphan).expect("orphan");
        let (result, _state) = reopen(&data_dir);
        assert_eq!(
            result.expect_err("orphan").reason,
            Some(seed_reason::RECOVERY_REFUSED)
        );
        assert!(orphan.exists());
        std::fs::remove_dir_all(&data_dir).expect("cleanup");
    }

    #[test]
    fn pending_shape_and_temp_names_are_exact() {
        let generated =
            atomic_temp_path(&manifest_path(Path::new("data").join(VAULT_NAME).as_path()));
        assert_eq!(
            atomic_temp_target(&generated.file_name().expect("name").to_string_lossy()),
            Some(MANIFEST_NAME),
            "atomic_temp_target is the exact inverse of write_atomic's temp name"
        );
        assert_eq!(atomic_temp_target(".tmp"), None);
        assert_eq!(atomic_temp_target("synthetic-workspace"), None);
        let nonce = new_discard_nonce();
        assert!(valid_discard_nonce(&nonce));
        assert!(!valid_discard_nonce("../../../../outside-the-data-dir"));
        assert!(!valid_discard_nonce(&"A".repeat(32)));
        let pending = SeedManifest {
            pending: true,
            discard_nonce: Some(nonce),
            ..SeedManifest::default()
        };
        let bytes = serde_json::to_vec(&pending).expect("json");
        assert!(exact_pending_manifest(&bytes).is_some());
        let mut extra: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        extra["sources"] = serde_json::json!([{"source_id": "x"}]);
        assert!(exact_pending_manifest(extra.to_string().as_bytes()).is_none());
        let mut unknown: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        unknown["unexpected"] = serde_json::json!(1);
        assert!(exact_pending_manifest(unknown.to_string().as_bytes()).is_none());
        // A complete manifest is byte-identical to the pre-step-4 format.
        let complete = serde_json::to_value(full_seed_manifest_for_test()).expect("json");
        assert!(complete.get("pending").is_none() && complete.get("discard_nonce").is_none());
    }

    #[test]
    fn seed_only_accepts_absent_and_complete_genuine_seed() {
        let (data_dir, vault_root) = seed_only_scratch("complete");
        assert_eq!(seed_only_vault(&vault_root), SeedOnlyCheck::FixtureOnly);
        let mut state = State {
            data_dir: Some(data_dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut state).expect("genuine 12/12 seed");
        drop(state);
        assert_eq!(
            seed_only_vault(&vault_root),
            SeedOnlyCheck::FixtureOnly,
            "a genuine bundled seed is fixture-only"
        );
        std::fs::remove_dir_all(&data_dir).expect("synthetic-only cleanup");
    }

    #[test]
    fn seed_only_accepts_a_genuine_partial_seed() {
        let (data_dir, vault_root) = seed_only_scratch("partial");
        std::fs::create_dir_all(&data_dir).expect("dir");
        let mut session = CliSession::connect("desktop-synthetic").expect("Core");
        session
            .open_synthetic_vault(&vault_root.display().to_string())
            .expect("vault");
        let (subject, fixtures) = seed_plan().into_iter().next().expect("plan");
        let fixture = fixtures.into_iter().next().expect("fixture");
        let path = data_dir.join(fixture.file);
        std::fs::write(&path, fixture.bytes).expect("staging");
        let source = session
            .ingest_fhir_file(&path.display().to_string())
            .expect("ingest");
        let resource: serde_json::Value = serde_json::from_slice(fixture.bytes).expect("json");
        session
            .promote_fixture_as_assertion(source, subject, fixture.claim_kind, resource)
            .expect("promote");
        drop(session);
        std::fs::remove_file(&path).expect("staging cleanup");
        assert_eq!(seed_only_vault(&vault_root), SeedOnlyCheck::FixtureOnly);
        std::fs::remove_dir_all(&data_dir).expect("synthetic-only cleanup");
    }

    #[test]
    fn seed_only_rejects_user_import_unknown_files_and_feature_rows() {
        let (data_dir, vault_root) = seed_only_scratch("user");
        let mut state = State {
            data_dir: Some(data_dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut state).expect("seed");
        let (subject, fixtures) = seed_plan().into_iter().next().expect("plan");
        let mut resource: serde_json::Value =
            serde_json::from_slice(fixtures[0].bytes).expect("json");
        resource["id"] = serde_json::json!("seed-only-user-import");
        let file = data_dir.join("user-import.json");
        std::fs::write(&file, serde_json::to_vec(&resource).expect("json")).expect("write");
        import_fhir(state.workspace.as_mut().expect("workspace"), &file, subject)
            .expect("user import");
        drop(state);
        assert_eq!(
            seed_only_vault(&vault_root),
            SeedOnlyCheck::NotFixtureOnly,
            "a user FHIR import is user data"
        );
        std::fs::remove_dir_all(&data_dir).expect("synthetic-only cleanup");

        let (data_dir, vault_root) = seed_only_scratch("extra");
        let mut state = State {
            data_dir: Some(data_dir.clone()),
            ..State::default()
        };
        open_synthetic(&mut state).expect("seed");
        drop(state);
        std::fs::write(vault_root.join("synthetic-fixture-table.csv"), b"a,b").expect("extra");
        assert_eq!(seed_only_vault(&vault_root), SeedOnlyCheck::NotFixtureOnly);
        std::fs::remove_file(vault_root.join("synthetic-fixture-table.csv")).expect("rm");
        assert_eq!(seed_only_vault(&vault_root), SeedOnlyCheck::FixtureOnly);
        {
            let db = rusqlite::Connection::open(vault_root.join("meta.sqlite3")).expect("db");
            db.execute(
                "INSERT INTO store_state(key, value) VALUES ('project_id_seq', '1')",
                [],
            )
            .expect("feature counter");
        }
        assert_eq!(
            seed_only_vault(&vault_root),
            SeedOnlyCheck::NotFixtureOnly,
            "any feature id counter means user activity"
        );
        std::fs::write(vault_root.join("meta.sqlite3-journal"), b"hot").expect("journal");
        assert_eq!(
            seed_only_vault(&vault_root),
            SeedOnlyCheck::Unproven,
            "a hot journal is never proof"
        );
        std::fs::remove_dir_all(&data_dir).expect("synthetic-only cleanup");
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
        assert!(matches!(
            preflight_synthetic_seed(&root),
            Ok(Preflight::Fresh)
        ));
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
        assert_eq!(missing.reason, Some(seed_reason::MANIFEST_MISSING));
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
            ..SeedManifest::default()
        };
        save_manifest(&vault_root, &partial).expect("write deliberately partial fixture roster");
        let invalid = open_synthetic(&mut restarted).expect_err("partial seed must fail closed");
        assert_eq!(invalid.kind, "corrupt");
        assert_eq!(invalid.reason, Some(seed_reason::MANIFEST_INCOMPLETE));
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
        assert_eq!(forged.reason, Some(seed_reason::CORE_LEDGER_INCOMPLETE));
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
