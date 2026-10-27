//! SQLite metadata for source visibility, authority objects, and migration journal.

use std::path::Path;

use medscale_contracts::objects::{AuthorityScopeId, DigestSha256, OpaqueId, RealmId};
use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

use crate::claim::{ClaimError, assert_claim_path};
use crate::migrate::MigrationJournal;

/// Metadata store errors.
#[derive(Debug, Error)]
pub enum MetaError {
    #[error(transparent)]
    Claim(#[from] ClaimError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("not found")]
    NotFound,
    #[error("revision conflict: {0}")]
    Conflict(String),
    #[error("unsupported durable value: {0}")]
    UnsupportedSchema(String),
    #[error("corrupt object body: {0}")]
    CorruptObjectBody(String),
    #[error("migration incomplete at version {0}")]
    MigrationIncomplete(u32),
}

/// Durable authority object row (Spec 016 schema v2).
#[derive(Debug, Clone)]
pub struct AuthorityObjectRow {
    pub object_id: String,
    pub object_class: String,
    pub realm_id: String,
    pub authority_scope_id: String,
    pub body_json: String,
    pub content_digest_hex: Option<String>,
    pub updated_seq: u64,
}

/// Row describing a durable source blob binding.
#[derive(Debug, Clone)]
pub struct SourceMeta {
    pub source_id: OpaqueId,
    pub realm_id: RealmId,
    pub authority_scope_id: AuthorityScopeId,
    pub digest: DigestSha256,
    pub byte_length: u64,
    pub media_type: String,
    pub visible: bool,
    pub resource_type: String,
}

/// SQLite-backed metadata (one connection per vault / process host).
#[derive(Debug)]
pub struct SqliteMetaStore {
    conn: Connection,
}

impl SqliteMetaStore {
    pub fn open(vault_root: &Path) -> Result<Self, MetaError> {
        let root = assert_claim_path(vault_root)?;
        std::fs::create_dir_all(&root)?;
        let db_path = root.join("meta.sqlite3");
        Self::open_at(&db_path)
    }

    /// Open metadata DB at an explicit path (SyntheticVault / unkeyed plaintext-compatible).
    pub fn open_at(db_path: &Path) -> Result<Self, MetaError> {
        let conn = Self::open_connection(db_path)?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Crate-internal connection access for the Spec 074 row module.
    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Open EncryptedVault work DB with SQLCipher key derived from VaultDek (Spec 023).
    ///
    /// Key material is applied via `PRAGMA key` before any schema read/write. Wrong keys
    /// fail closed on the first schema touch. Never logs the key.
    pub fn open_at_sqlcipher(db_path: &Path, key32: &[u8; 32]) -> Result<Self, MetaError> {
        let conn = Self::open_connection(db_path)?;
        apply_sqlcipher_key(&conn, key32)?;
        verify_sqlcipher_readable(&conn)?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Open work DB under VaultDek key, migrating legacy plaintext sealed work via rekey.
    pub fn open_at_sqlcipher_or_rekey_legacy(
        db_path: &Path,
        key32: &[u8; 32],
    ) -> Result<Self, MetaError> {
        match Self::open_at_sqlcipher(db_path, key32) {
            Ok(store) => Ok(store),
            Err(primary) if db_path.exists() => {
                // Pre-023 AES-GCM seals may contain plaintext SQLite pages.
                let conn = Self::open_connection(db_path)?;
                if verify_sqlcipher_readable(&conn).is_err() {
                    return Err(primary);
                }
                apply_sqlcipher_rekey(&conn, key32)?;
                verify_sqlcipher_readable(&conn)?;
                let store = Self { conn };
                store.migrate()?;
                Ok(store)
            }
            Err(e) => Err(e),
        }
    }

    fn open_connection(db_path: &Path) -> Result<Connection, MetaError> {
        if let Some(parent) = db_path.parent() {
            let _ = assert_claim_path(parent)?;
            std::fs::create_dir_all(parent)?;
        }
        Ok(Connection::open(db_path)?)
    }

    /// Embedded SQLCipher library version string (`PRAGMA cipher_version`).
    pub fn sqlcipher_cipher_version(conn: &Connection) -> Result<String, MetaError> {
        let v: String = conn.pragma_query_value(None, "cipher_version", |row| row.get(0))?;
        Ok(v)
    }

    /// Report cipher_version for an already-open SQLCipher store.
    pub fn cipher_version(&self) -> Result<String, MetaError> {
        Self::sqlcipher_cipher_version(&self.conn)
    }

    /// True when workspace builds with the Spec 023 SQLCipher backend.
    #[must_use]
    pub fn sqlcipher_backend_active() -> bool {
        cfg!(feature = "sqlcipher")
    }

    fn migrate(&self) -> Result<(), MetaError> {
        self.conn.execute_batch(
            r"
            CREATE TABLE IF NOT EXISTS migration_journal (
              version INTEGER PRIMARY KEY,
              state TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sources (
              source_id TEXT PRIMARY KEY,
              realm_id TEXT NOT NULL,
              authority_scope_id TEXT NOT NULL,
              digest_hex TEXT NOT NULL,
              byte_length INTEGER NOT NULL,
              media_type TEXT NOT NULL,
              visible INTEGER NOT NULL,
              resource_type TEXT NOT NULL
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_sources_scope_digest
              ON sources(authority_scope_id, digest_hex);
            CREATE TABLE IF NOT EXISTS gc_marks (
              digest_hex TEXT PRIMARY KEY,
              epoch INTEGER NOT NULL
            );
            ",
        )?;
        let journal = self.migration_journal()?;
        if journal.started_version.is_some()
            && journal
                .started_version
                .is_some_and(|v| v > journal.finished_version)
        {
            // Incomplete migration must not silently continue as empty success.
            return Err(MetaError::MigrationIncomplete(
                journal.started_version.unwrap_or(0),
            ));
        }
        if journal.finished_version < 1 {
            self.conn.execute(
                "INSERT OR REPLACE INTO migration_journal(version, state) VALUES (1, 'finished')",
                [],
            )?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 2 {
            self.begin_migration(2)?;
            self.conn.execute_batch(
                r"
                CREATE TABLE IF NOT EXISTS authority_objects (
                  object_id TEXT PRIMARY KEY,
                  object_class TEXT NOT NULL,
                  realm_id TEXT NOT NULL,
                  authority_scope_id TEXT NOT NULL,
                  body_json TEXT NOT NULL,
                  content_digest_hex TEXT,
                  updated_seq INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_authority_objects_scope_class
                  ON authority_objects(authority_scope_id, object_class);
                CREATE TABLE IF NOT EXISTS store_state (
                  key TEXT PRIMARY KEY,
                  value TEXT NOT NULL
                );
                INSERT OR IGNORE INTO store_state(key, value) VALUES ('next_seq', '0');
                ",
            )?;
            self.finish_migration(2)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 3 {
            self.begin_migration(3)?;
            self.conn.execute_batch(crate::project_graph::V3_DDL)?;
            self.finish_migration(3)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 4 {
            self.begin_migration(4)?;
            self.conn.execute_batch(crate::data_sources::V4_DDL)?;
            self.finish_migration(4)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 5 {
            self.begin_migration(5)?;
            self.conn.execute_batch(crate::collaboration::V5_DDL)?;
            self.finish_migration(5)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 6 {
            self.begin_migration(6)?;
            self.conn.execute_batch(crate::medagent::V6_DDL)?;
            self.finish_migration(6)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 7 {
            self.begin_migration(7)?;
            self.conn.execute_batch(crate::model_fleet::V7_DDL)?;
            self.finish_migration(7)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 8 {
            self.begin_migration(8)?;
            self.conn.execute_batch(crate::privacy_gate::V8_DDL)?;
            self.finish_migration(8)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 9 {
            self.begin_migration(9)?;
            self.conn.execute_batch(crate::browse::V9_DDL)?;
            self.finish_migration(9)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 10 {
            self.begin_migration(10)?;
            self.conn.execute_batch(crate::audio::V10_DDL)?;
            self.finish_migration(10)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 11 {
            self.begin_migration(11)?;
            self.conn.execute_batch(crate::analytics::V11_DDL)?;
            self.finish_migration(11)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 12 {
            self.begin_migration(12)?;
            self.conn.execute_batch(crate::knowledge::V12_DDL)?;
            self.finish_migration(12)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 13 {
            self.begin_migration(13)?;
            self.conn.execute_batch(crate::hub::V13_DDL)?;
            self.finish_migration(13)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 14 {
            self.begin_migration(14)?;
            self.conn.execute_batch(crate::compute::V14_DDL)?;
            self.finish_migration(14)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 15 {
            self.begin_migration(15)?;
            self.conn.execute_batch(crate::r_workspace::V15_DDL)?;
            self.finish_migration(15)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 16 {
            self.begin_migration(16)?;
            self.conn.execute_batch(crate::extensions::V16_DDL)?;
            self.finish_migration(16)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 17 {
            self.begin_migration(17)?;
            self.conn.execute_batch(crate::huddles::V17_DDL)?;
            self.finish_migration(17)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 18 {
            self.begin_migration(18)?;
            self.conn.execute_batch(crate::research_packs::V18_DDL)?;
            self.finish_migration(18)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 19 {
            self.begin_migration(19)?;
            self.conn.execute_batch(crate::institutional::V19_DDL)?;
            self.finish_migration(19)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 20 {
            self.begin_migration(20)?;
            self.conn.execute_batch(crate::federation::V20_DDL)?;
            self.finish_migration(20)?;
        }
        let journal = self.migration_journal()?;
        if journal.finished_version < 21 {
            self.begin_migration(21)?;
            self.conn.execute_batch(crate::audio::V21_DDL)?;
            self.finish_migration(21)?;
        }
        Ok(())
    }

    pub fn migration_journal(&self) -> Result<MigrationJournal, MetaError> {
        let mut stmt = self
            .conn
            .prepare("SELECT version, state FROM migration_journal ORDER BY version")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut finished = 0_u32;
        let mut started = None;
        for row in rows {
            let (version, state) = row?;
            if state == "finished" {
                finished = finished.max(version);
            } else if state == "started" {
                started = Some(version);
            }
        }
        Ok(MigrationJournal {
            finished_version: finished,
            started_version: started,
        })
    }

    pub fn begin_migration(&self, to: u32) -> Result<(), MetaError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO migration_journal(version, state) VALUES (?1, 'started')",
            params![to],
        )?;
        Ok(())
    }

    pub fn finish_migration(&self, to: u32) -> Result<(), MetaError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO migration_journal(version, state) VALUES (?1, 'finished')",
            params![to],
        )?;
        Ok(())
    }

    pub fn find_by_digest(
        &self,
        scope: &AuthorityScopeId,
        digest: &DigestSha256,
    ) -> Result<Option<SourceMeta>, MetaError> {
        let hex = hex_digest(digest);
        let mut stmt = self.conn.prepare(
            "SELECT source_id, realm_id, authority_scope_id, digest_hex, byte_length, media_type, visible, resource_type
             FROM sources WHERE authority_scope_id = ?1 AND digest_hex = ?2",
        )?;
        let mut rows = stmt.query(params![scope.as_opaque().as_str(), hex])?;
        match rows.next()? {
            Some(row) => Ok(Some(map_source_row(row)?)),
            None => Ok(None),
        }
    }

    pub fn insert_source(&self, meta: &SourceMeta) -> Result<(), MetaError> {
        self.conn.execute(
            "INSERT INTO sources(source_id, realm_id, authority_scope_id, digest_hex, byte_length, media_type, visible, resource_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                meta.source_id.as_str(),
                meta.realm_id.as_opaque().as_str(),
                meta.authority_scope_id.as_opaque().as_str(),
                hex_digest(&meta.digest),
                meta.byte_length as i64,
                meta.media_type,
                i64::from(meta.visible),
                meta.resource_type,
            ],
        )?;
        Ok(())
    }

    pub fn upsert_source(&self, meta: &SourceMeta) -> Result<(), MetaError> {
        self.conn.execute(
            "INSERT INTO sources(source_id, realm_id, authority_scope_id, digest_hex, byte_length, media_type, visible, resource_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(source_id) DO UPDATE SET
               realm_id=excluded.realm_id,
               authority_scope_id=excluded.authority_scope_id,
               digest_hex=excluded.digest_hex,
               byte_length=excluded.byte_length,
               media_type=excluded.media_type,
               visible=excluded.visible,
               resource_type=excluded.resource_type",
            params![
                meta.source_id.as_str(),
                meta.realm_id.as_opaque().as_str(),
                meta.authority_scope_id.as_opaque().as_str(),
                hex_digest(&meta.digest),
                meta.byte_length as i64,
                meta.media_type,
                i64::from(meta.visible),
                meta.resource_type,
            ],
        )?;
        Ok(())
    }

    pub fn get_source(&self, source_id: &OpaqueId) -> Result<SourceMeta, MetaError> {
        let mut stmt = self.conn.prepare(
            "SELECT source_id, realm_id, authority_scope_id, digest_hex, byte_length, media_type, visible, resource_type
             FROM sources WHERE source_id = ?1",
        )?;
        let mut rows = stmt.query(params![source_id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        Ok(map_source_row(row)?)
    }

    pub fn list_sources(&self) -> Result<Vec<SourceMeta>, MetaError> {
        let mut stmt = self.conn.prepare(
            "SELECT source_id, realm_id, authority_scope_id, digest_hex, byte_length, media_type, visible, resource_type FROM sources",
        )?;
        let mapped = stmt.query_map([], map_source_row)?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn clear_gc_marks(&self) -> Result<(), MetaError> {
        self.conn.execute("DELETE FROM gc_marks", [])?;
        Ok(())
    }

    pub fn mark_digest(&self, digest: &DigestSha256, epoch: u64) -> Result<(), MetaError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO gc_marks(digest_hex, epoch) VALUES (?1, ?2)",
            params![hex_digest(digest), epoch as i64],
        )?;
        Ok(())
    }

    pub fn is_marked(&self, digest: &DigestSha256) -> Result<bool, MetaError> {
        let mut stmt = self
            .conn
            .prepare("SELECT 1 FROM gc_marks WHERE digest_hex = ?1")?;
        let mut rows = stmt.query(params![hex_digest(digest)])?;
        Ok(rows.next()?.is_some())
    }

    /// Every id sequence in `store_state` except the authority store's own
    /// `next_seq` (which travels with the authority rows).
    pub fn list_id_sequences(&self) -> Result<Vec<(String, u64)>, MetaError> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM store_state WHERE key <> 'next_seq' ORDER BY key")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (key, value) = row?;
            let value = value.parse::<u64>().map_err(|_| {
                MetaError::CorruptObjectBody(format!("id sequence {key} is not numeric"))
            })?;
            out.push((key, value));
        }
        Ok(out)
    }

    /// Restores id sequences, never lowering one already present.
    pub fn restore_id_sequences(
        &self,
        sequences: &std::collections::BTreeMap<String, u64>,
    ) -> Result<(), MetaError> {
        let tx = self.conn.unchecked_transaction()?;
        for (key, value) in sequences {
            if key == "next_seq" || key.is_empty() || key.len() > 256 {
                return Err(MetaError::CorruptObjectBody(
                    "tampered backup: invalid id sequence key".to_owned(),
                ));
            }
            let current: Option<String> = tx
                .query_row(
                    "SELECT value FROM store_state WHERE key = ?1",
                    params![key],
                    |row| row.get(0),
                )
                .optional()?;
            let current = current.and_then(|c| c.parse::<u64>().ok()).unwrap_or(0);
            tx.execute(
                "INSERT OR REPLACE INTO store_state(key, value) VALUES (?1, ?2)",
                params![key, current.max(*value).to_string()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn snapshot_bytes(&self) -> Result<Vec<u8>, MetaError> {
        let sources = self.list_sources()?;
        let source_payload: Vec<_> = sources
            .iter()
            .map(|s| {
                serde_json::json!({
                    "source_id": s.source_id.as_str(),
                    "realm_id": s.realm_id.as_opaque().as_str(),
                    "authority_scope_id": s.authority_scope_id.as_opaque().as_str(),
                    "digest_hex": hex_digest(&s.digest),
                    "byte_length": s.byte_length,
                    "media_type": s.media_type,
                    "visible": s.visible,
                    "resource_type": s.resource_type,
                })
            })
            .collect();
        let objects = self.list_authority_objects()?;
        let object_payload: Vec<_> = objects
            .iter()
            .map(|o| {
                serde_json::json!({
                    "object_id": o.object_id,
                    "object_class": o.object_class,
                    "realm_id": o.realm_id,
                    "authority_scope_id": o.authority_scope_id,
                    "body_json": o.body_json,
                    "content_digest_hex": o.content_digest_hex,
                    "updated_seq": o.updated_seq,
                })
            })
            .collect();
        let payload = serde_json::json!({
            "schema_version": crate::CURRENT_META_SCHEMA_VERSION,
            "next_seq": self.get_next_seq()?,
            "sources": source_payload,
            "objects": object_payload,
            // Spec 074: full row scans ride the same snapshot so synthetic
            // backup/restore preserves Projects without a second mechanism.
            // Bounded callers only; scale evidence records observed sizes.
            "projects": self.list_all_projects()?,
            "experiments": self.list_all_experiments()?,
            "refs": self.list_all_refs()?,
            "edges": self.list_all_edges()?,
            // Spec 075: Data Source Fabric rows ride the same snapshot so
            // synthetic backup/restore preserves sources, snapshots, views,
            // transformations and releases without a second mechanism.
            "data_sources": self.list_all_data_sources()?,
            "data_snapshots": self.list_all_snapshots()?,
            "data_snapshot_parts": self.list_all_snapshot_parts()?,
            "data_receipts": self.list_all_receipts()?,
            "data_saved_views": self.list_all_saved_views()?,
            "data_transformations": self.list_all_transformations()?,
            "dataset_releases": self.list_all_dataset_releases()?,
            // Spec 076: Collaboration Substrate rows ride the same snapshot
            // so synthetic backup/restore preserves rooms, participants,
            // threads, messages, tasks, notes, approvals and the activity
            // hash chain without a second mechanism.
            "collab_participants": self.list_all_participants()?,
            "collab_participant_agent_refs": self.list_all_participant_agent_refs()?,
            "collab_rooms": self.list_all_rooms()?,
            "collab_room_memberships": self.list_all_memberships()?,
            "collab_threads": self.list_all_threads()?,
            "collab_messages": self.list_all_messages()?,
            "collab_message_edits": self.list_all_message_edits()?,
            "collab_tasks": self.list_all_tasks()?,
            "collab_notes": self.list_all_notes()?,
            "collab_note_revisions": self.list_all_note_revisions()?,
            "collab_approval_requests": self.list_all_approval_requests()?,
            "collab_approval_decisions": self.list_all_approval_decisions()?,
            "collab_activity_records": self.list_all_activity_records()?,
            // Spec 077: MedAgent Workbench rows ride the same snapshot so
            // synthetic backup/restore preserves agent identities, context
            // manifests, runs, turns, tool invocations/receipts, run
            // receipts and proposal links without a second mechanism.
            "medagent_identities": self.list_all_agent_identities()?,
            "medagent_capability_manifests": self.list_all_capability_manifests()?,
            "medagent_context_manifests": self.list_all_context_manifests()?,
            "medagent_runs": self.list_all_agent_runs()?,
            "medagent_turns": self.list_all_agent_turns()?,
            "medagent_tool_invocations": self.list_all_tool_invocations()?,
            "medagent_tool_receipts": self.list_all_tool_receipts()?,
            "medagent_run_receipts": self.list_all_run_receipts()?,
            "medagent_proposals": self.list_all_agent_proposals()?,
            // Spec 078: Model Fleet + Compare rows ride the same snapshot so
            // synthetic backup/restore preserves agent lanes, fleet runs,
            // lane run refs and comparison reports without a second
            // mechanism.
            "model_fleet_lanes": self.list_all_agent_lanes()?,
            "model_fleet_runs": self.list_all_fleet_runs()?,
            "model_fleet_lane_run_refs": self.list_all_lane_run_refs()?,
            "model_fleet_comparison_reports": self.list_all_comparison_reports()?,
        });
        // Spec 079: Privacy Gate rows (added outside `json!` to stay under the
        // macro recursion limit). Pseudonym entries are sealed; map keys live
        // in the KeyStore and are never part of a backup.
        let privacy = [
            (
                "privacy_classifications",
                serde_json::to_value(self.list_all_classifications()?),
            ),
            (
                "privacy_profiles",
                serde_json::to_value(self.list_all_privacy_profiles()?),
            ),
            (
                "privacy_deid_receipts",
                serde_json::to_value(self.list_all_deid_receipts()?),
            ),
            (
                "privacy_pseudonym_maps",
                serde_json::to_value(self.list_all_pseudonym_maps()?),
            ),
            (
                "privacy_pseudonym_entries",
                serde_json::to_value(self.list_all_pseudonym_entries()?),
            ),
            (
                "privacy_reid_audit",
                serde_json::to_value(self.list_all_reid_audit()?),
            ),
            (
                "privacy_egress_decisions",
                serde_json::to_value(self.list_all_egress_decisions()?),
            ),
            // Spec 080: Governed Browse rows (evidence/download bytes as hex).
            (
                "browse_allowlist",
                serde_json::to_value(self.list_all_browse_allowlist()?),
            ),
            (
                "browse_sessions",
                serde_json::to_value(self.list_all_browse_sessions()?),
            ),
            (
                "browse_evidence",
                serde_json::to_value(self.list_all_browse_evidence()?),
            ),
            (
                "browse_downloads",
                serde_json::to_value(self.list_all_browse_downloads()?),
            ),
            (
                "browse_receipts",
                serde_json::to_value(self.list_all_browse_receipts()?),
            ),
            // Spec 081: AudioFlow rows (source audio and open-capture chunks
            // as hex).
            (
                "audio_sources",
                serde_json::to_value(self.list_all_audio_sources()?),
            ),
            (
                "audio_capture_sessions",
                serde_json::to_value(self.list_all_capture_sessions()?),
            ),
            (
                "audio_capture_chunks",
                serde_json::to_value(self.list_all_capture_chunks()?),
            ),
            (
                "audio_transcripts",
                serde_json::to_value(self.list_all_transcript_revisions()?),
            ),
            (
                "audio_transcript_receipts",
                serde_json::to_value(self.list_all_transcript_receipts()?),
            ),
            // Active-vault removals have independent, immutable receipts.
            (
                "audio_retention_removal_receipts",
                serde_json::to_value(self.list_audio_retention_removal_receipts()?),
            ),
            // Spec 082: Analytics Gate rows (derived table bytes as hex).
            (
                "analytics_receipts",
                serde_json::to_value(self.list_all_query_receipts()?),
            ),
            (
                "analytics_results",
                serde_json::to_value(self.list_all_derived_tables()?),
            ),
            (
                "analytics_cohorts",
                serde_json::to_value(self.list_all_cohorts()?),
            ),
            // Spec 083: Knowledge + Research Canvas rows.
            (
                "knowledge_index_versions",
                serde_json::to_value(self.list_all_index_versions()?),
            ),
            (
                "knowledge_receipts",
                serde_json::to_value(self.list_all_retrieval_receipts()?),
            ),
            (
                "knowledge_canvases",
                serde_json::to_value(self.list_all_canvas_revisions()?),
            ),
        ];
        let mut payload = payload;
        for (key, value) in privacy {
            payload[key] = value.map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        }
        // Spec 084: Hub rows (device secrets are never exported).
        for (key, value) in self.hub_backup_families()? {
            payload[key] = value;
        }
        // Spec 085: Compute rows (output bytes as hex).
        for (key, value) in self.compute_backup_families()? {
            payload[key] = value;
        }
        // Spec 086: R Workspace rows (published table bytes as hex).
        for (key, value) in self.r_workspace_backup_families()? {
            payload[key] = value;
        }
        // Spec 087: Community Extensions rows.
        for (key, value) in self.extension_backup_families()? {
            payload[key] = value;
        }
        // Spec 088: AudioFlow Advanced huddle rows.
        for (key, value) in self.huddle_backup_families()? {
            payload[key] = value;
        }
        // Spec 089: Research Pack rows.
        for (key, value) in self.research_pack_backup_families()? {
            payload[key] = value;
        }
        // Spec 090: institutional adapter rows (credential handles only).
        for (key, value) in self.institutional_backup_families()? {
            payload[key] = value;
        }
        // Spec 091: federation rows (identity secret never exported).
        for (key, value) in self.federation_backup_families()? {
            payload[key] = value;
        }
        // Id sequences: without them a restored vault would hand out ids
        // that restored rows already use.
        let sequences: std::collections::BTreeMap<String, u64> =
            self.list_id_sequences()?.into_iter().collect();
        payload["id_sequences"] = serde_json::json!(sequences);
        Ok(serde_json::to_vec(&payload).unwrap_or_default())
    }

    pub fn get_next_seq(&self) -> Result<u64, MetaError> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM store_state WHERE key = 'next_seq'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value.unwrap_or_else(|| "0".to_owned()).parse().unwrap_or(0))
    }

    pub fn set_next_seq(&self, next_seq: u64) -> Result<(), MetaError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO store_state(key, value) VALUES ('next_seq', ?1)",
            params![next_seq.to_string()],
        )?;
        Ok(())
    }

    pub fn list_authority_objects(&self) -> Result<Vec<AuthorityObjectRow>, MetaError> {
        let mut stmt = self.conn.prepare(
            "SELECT object_id, object_class, realm_id, authority_scope_id, body_json, content_digest_hex, updated_seq
             FROM authority_objects",
        )?;
        let mapped = stmt.query_map([], |row| {
            Ok(AuthorityObjectRow {
                object_id: row.get(0)?,
                object_class: row.get(1)?,
                realm_id: row.get(2)?,
                authority_scope_id: row.get(3)?,
                body_json: row.get(4)?,
                content_digest_hex: row.get(5)?,
                updated_seq: row.get::<_, i64>(6)? as u64,
            })
        })?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(row?);
        }
        Ok(out)
    }

    /// Replace all authority objects and next_seq in one transaction (Spec 016 sync).
    pub fn replace_authority_snapshot(
        &self,
        objects: &[AuthorityObjectRow],
        next_seq: u64,
    ) -> Result<(), MetaError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM authority_objects", [])?;
        for obj in objects {
            tx.execute(
                "INSERT INTO authority_objects(object_id, object_class, realm_id, authority_scope_id, body_json, content_digest_hex, updated_seq)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    obj.object_id,
                    obj.object_class,
                    obj.realm_id,
                    obj.authority_scope_id,
                    obj.body_json,
                    obj.content_digest_hex,
                    obj.updated_seq as i64,
                ],
            )?;
        }
        tx.execute(
            "INSERT OR REPLACE INTO store_state(key, value) VALUES ('next_seq', ?1)",
            params![next_seq.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }
}

fn map_source_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SourceMeta> {
    let digest_hex: String = row.get(3)?;
    Ok(SourceMeta {
        source_id: OpaqueId::new(row.get::<_, String>(0)?),
        realm_id: RealmId::new(row.get::<_, String>(1)?),
        authority_scope_id: AuthorityScopeId::new(row.get::<_, String>(2)?),
        digest: parse_hex(&digest_hex),
        byte_length: row.get::<_, i64>(4)? as u64,
        media_type: row.get(5)?,
        visible: row.get::<_, i64>(6)? != 0,
        resource_type: row.get(7)?,
    })
}

fn hex_digest(digest: &DigestSha256) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn parse_hex(name: &str) -> DigestSha256 {
    let mut bytes = [0_u8; 32];
    for (i, chunk) in name.as_bytes().chunks(2).enumerate() {
        if i >= 32 {
            break;
        }
        let s = std::str::from_utf8(chunk).unwrap_or("00");
        bytes[i] = u8::from_str_radix(s, 16).unwrap_or(0);
    }
    DigestSha256::from_bytes(bytes)
}

fn key_material_hex(key32: &[u8; 32]) -> String {
    key32.iter().map(|b| format!("{b:02x}")).collect()
}

fn apply_sqlcipher_key(conn: &Connection, key32: &[u8; 32]) -> Result<(), MetaError> {
    // Passphrase-style key material (hex of VaultDek). Never logged.
    conn.pragma_update(None, "key", key_material_hex(key32))?;
    Ok(())
}

fn apply_sqlcipher_rekey(conn: &Connection, key32: &[u8; 32]) -> Result<(), MetaError> {
    conn.pragma_update(None, "rekey", key_material_hex(key32))?;
    Ok(())
}

fn verify_sqlcipher_readable(conn: &Connection) -> Result<(), MetaError> {
    // Wrong SQLCipher keys typically fail here with "file is not a database".
    let _: i64 = conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))?;
    Ok(())
}
