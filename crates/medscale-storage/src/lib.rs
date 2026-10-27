//! Durable storage: synthetic vaults (003) + encrypted vaults (005).

mod analytics;
pub mod analytics_engine;
mod audio;
mod backup;
mod blob;
mod browse;
mod claim;
mod collaboration;
mod compute;
mod data_sources;
mod encrypted_vault;
mod extensions;
mod federation;
mod gc;
mod hub;
mod huddles;
mod institutional;
mod knowledge;
mod medagent;
mod migrate;
mod model_fleet;
mod privacy_gate;
mod privacy_probes;
mod project_graph;
mod r_workspace;
mod research_packs;
mod sealed_blob;
mod sqlite_meta;
mod vault;
mod writer_lock;

pub use analytics::DerivedTableRow;
pub use audio::{AudioChunkRow, AudioRetentionRemovalReceipt, AudioSourceRow};
pub use backup::{backup_vault, restore_vault};
pub use blob::FsBlobStore;
pub use browse::{BrowseDownloadRow, BrowseEvidenceRow, BrowseSessionCommit};
pub use claim::{ClaimError, assert_claim_path};
pub use compute::{COMPUTE_TABLES, ComputeOutputRow};
pub use data_sources::{
    EXTERNAL_TABLES_MAX, ExternalCell, ExternalTable, RECEIPT_KIND_IMPORT, RECEIPT_KIND_REFRESH,
    RECEIPT_KIND_TRANSFORMATION, SnapshotRecord, TransformationRecord, list_external_sqlite_tables,
    read_external_sqlite_table, validate_external_identifier,
};
pub use encrypted_vault::{EncryptedVault, EncryptedVaultError, default_vault_root};
pub use extensions::{EXTENSION_TABLES, InstallChange};
pub use federation::{FEDERATION_TABLES, FederationChange, ImportedItemRow};
pub use gc::{GcStats, run_gc};
pub use hub::{MirrorRow, SeqClaim, event_claims_seq};
pub use huddles::{HUDDLE_TABLES, HuddleChange};
pub use institutional::{AdapterChange, INSTITUTIONAL_TABLES, effect_state_name};
pub use knowledge::IndexVersionRow;
pub use migrate::MigrationJournal;
pub use privacy_gate::{DeidTransformCommit, PseudonymEntryRow};
pub use privacy_probes::{
    ClassifiedResidualSurface, OsFileProbeResult, PrivacyProbeReport, ProbeHonestyClass,
    crash_sidecar_leftovers_present, probe_os_privacy_surfaces, residual_risk_classes_open,
    scan_vault_work_leftovers,
};
pub use r_workspace::{R_WORKSPACE_TABLES, RPublishedTableRow};
pub use research_packs::{PackChange, RESEARCH_PACK_TABLES};
pub use sealed_blob::SealedBlobStore;
pub use sqlite_meta::{AuthorityObjectRow, MetaError, SourceMeta, SqliteMetaStore};
pub use vault::{SyntheticVault, VaultError};
pub use writer_lock::{WriterLock, WriterLockError};

/// Top metadata schema version written by this build (Audio retention: v21). Tests of
/// earlier specs assert against this constant so a later additive migration
/// does not require editing them.
pub const CURRENT_META_SCHEMA_VERSION: u32 = 21;
