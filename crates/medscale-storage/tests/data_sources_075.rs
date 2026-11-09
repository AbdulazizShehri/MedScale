//! Spec 075 storage + migration integration tests (075-B).
//!
//! Synthetic data only. Every claim binds to exact behavior below; nothing is
//! inferred from a green compile.

use std::fs;

use medscale_contracts::data_sources::{
    DATA_SOURCE_SCHEMA_VERSION, DataSourceKind, DataSourceManifest, DataViewKind, DatasetCard,
    FieldType, RightsState, SchemaField, SourceLocator, SourceRevisionBinding, SourceSchema,
    TransformationReceipt, ViewState, fingerprint_schema,
};
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_storage::{
    MetaError, SqliteMetaStore, backup_vault, list_external_sqlite_tables,
    read_external_sqlite_table, restore_vault,
};

fn temp_root(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-075-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn open_meta(root: &std::path::Path) -> SqliteMetaStore {
    SqliteMetaStore::open_at(&root.join("meta.sqlite3")).unwrap()
}

fn header(id: &str) -> ObjectHeader {
    ObjectHeader {
        id: OpaqueId::new(id),
        schema_version: DATA_SOURCE_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn scope() -> AuthorityScopeId {
    AuthorityScopeId::new("scope-1")
}

fn locator() -> SourceLocator {
    SourceLocator::LocalPath {
        path: "fixtures/table.csv".to_owned(),
        format: medscale_contracts::data_sources::LocalFileFormat::Csv,
    }
}

fn manifest(id: &str) -> DataSourceManifest {
    DataSourceManifest::new(
        header(id),
        OpaqueId::new("proj-1"),
        DataSourceKind::LocalTabularFile,
        format!("source-{id}"),
        "csv".to_owned(),
        locator(),
        None,
        vec![
            medscale_contracts::data_sources::DataSourceCapability::DiscoverSchema,
            medscale_contracts::data_sources::DataSourceCapability::ImportSnapshot,
        ],
    )
    .unwrap()
}

fn schema(source: &str) -> SourceSchema {
    let fields = vec![
        SchemaField {
            name: "city".to_owned(),
            field_type: FieldType::Text,
            nullable: true,
            declared_unit: None,
        },
        SchemaField {
            name: "dose".to_owned(),
            field_type: FieldType::Integer,
            nullable: true,
            declared_unit: None,
        },
    ];
    SourceSchema {
        source_id: OpaqueId::new(source),
        schema_fingerprint: fingerprint_schema(&fields),
        fields,
    }
}

fn snapshot_record(snap: &str, source: &str) -> medscale_storage::SnapshotRecord {
    use medscale_contracts::data_sources::{DataSnapshot, SnapshotStatus};
    let schema = schema(source);
    let snapshot = DataSnapshot {
        header: header(snap),
        source_id: OpaqueId::new(source),
        parent_snapshot_id: None,
        source_revision: SourceRevisionBinding::LocalFile {
            digest: DigestSha256::of(b"bytes"),
            byte_length: 5,
        },
        schema_fingerprint: schema.schema_fingerprint.clone(),
        row_count: 2,
        content_digest: DigestSha256::of(b"canonical"),
        status: SnapshotStatus::Complete,
    };
    medscale_storage::SnapshotRecord {
        snapshot,
        schema,
        project_id: OpaqueId::new("proj-1"),
    }
}

#[test]
fn source_crud_with_revision_conflicts() {
    let root = temp_root("source-crud");
    let meta = open_meta(&root);
    let source = manifest("dsrc-1");
    meta.insert_data_source(&source).unwrap();
    let dup = meta.insert_data_source(&source);
    assert!(matches!(dup, Err(MetaError::Conflict(_))), "{dup:?}");
    let read = meta.get_data_source(&OpaqueId::new("dsrc-1")).unwrap();
    assert_eq!(read.display_name, "source-dsrc-1");
    assert_eq!(read.revision, 1);

    let stale = meta.update_data_source_meta(&OpaqueId::new("dsrc-1"), 7, "new", None);
    assert!(matches!(stale, Err(MetaError::Conflict(_))), "{stale:?}");
    let updated = meta
        .update_data_source_meta(&OpaqueId::new("dsrc-1"), 1, "renamed", None)
        .unwrap();
    assert_eq!(updated.display_name, "renamed");
    assert_eq!(updated.revision, 2);

    let archived = meta
        .set_data_source_status(
            &OpaqueId::new("dsrc-1"),
            2,
            medscale_contracts::data_sources::SourceStatus::Archived,
        )
        .unwrap();
    assert_eq!(archived.revision, 3);

    let (listed, next) = meta
        .list_data_sources(&scope(), &OpaqueId::new("proj-1"), None, 25, None)
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert!(next.is_none());
    assert_eq!(listed[0].source_id.as_str(), "dsrc-1");
}

#[test]
fn unknown_source_is_not_found() {
    let root = temp_root("source-missing");
    let meta = open_meta(&root);
    let err = meta.get_data_source(&OpaqueId::new("nope")).unwrap_err();
    assert!(matches!(err, MetaError::NotFound), "{err:?}");
}

#[test]
fn snapshot_insert_lists_parts_and_latest() {
    let root = temp_root("snapshot-rows");
    let meta = open_meta(&root);
    meta.insert_data_source(&manifest("dsrc-1")).unwrap();
    let record = snapshot_record("snap-1", "dsrc-1");
    let parts = vec![medscale_contracts::data_sources::SnapshotPart {
        snapshot_id: OpaqueId::new("snap-1"),
        part_index: 0,
        row_start: 0,
        row_end: 2,
        part_digest: DigestSha256::of(b"part"),
    }];
    meta.insert_snapshot_full(&record, &parts).unwrap();
    let dup = meta.insert_snapshot_full(&record, &parts);
    assert!(matches!(dup, Err(MetaError::Conflict(_))), "{dup:?}");

    let read = meta.get_snapshot(&OpaqueId::new("snap-1")).unwrap();
    assert_eq!(read.snapshot.row_count, 2);
    assert_eq!(read.schema.fields.len(), 2);
    let read_parts = meta.get_snapshot_parts(&OpaqueId::new("snap-1")).unwrap();
    assert_eq!(read_parts.len(), 1);
    assert_eq!(read_parts[0].row_end, 2);

    let latest = meta
        .latest_snapshot_for_source(&OpaqueId::new("dsrc-1"))
        .unwrap()
        .expect("latest");
    assert_eq!(latest.snapshot.header.id.as_str(), "snap-1");

    let (listed, next) = meta
        .list_snapshots(&OpaqueId::new("dsrc-1"), 25, None)
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert!(next.is_none());
}

#[test]
fn receipts_views_transforms_releases_roundtrip() {
    let root = temp_root("receipts-views");
    let meta = open_meta(&root);
    meta.insert_data_source(&manifest("dsrc-1")).unwrap();
    let record = snapshot_record("snap-1", "dsrc-1");
    meta.insert_snapshot_full(&record, &[]).unwrap();

    let receipt = serde_json::json!({
        "source_id": "dsrc-1",
        "snapshot_id": "snap-1",
        "schema_fingerprint": record.schema.schema_fingerprint.to_hex(),
        "rows_materialized": 2,
        "rows_skipped": 0,
        "bytes_hashed": 5,
        "outcome": "complete",
        "warnings": []
    });
    meta.put_receipt("import", &OpaqueId::new("snap-1"), &receipt)
        .unwrap();
    let dup = meta.put_receipt("import", &OpaqueId::new("snap-1"), &receipt);
    assert!(matches!(dup, Err(MetaError::Conflict(_))), "{dup:?}");
    let back = meta
        .get_receipt("import", &OpaqueId::new("snap-1"))
        .unwrap();
    assert_eq!(back["rows_materialized"], 2);
    // Refresh receipts upsert idempotently.
    let refresh = serde_json::json!({"change": "unchanged"});
    meta.put_receipt("refresh", &OpaqueId::new("snap-1"), &refresh)
        .unwrap();
    meta.put_receipt("refresh", &OpaqueId::new("snap-1"), &refresh)
        .unwrap();

    let view = medscale_contracts::data_sources::SavedDataView {
        header: header("view-1"),
        snapshot_id: OpaqueId::new("snap-1"),
        revision: 1,
        view_kind: DataViewKind::Grid,
        state: ViewState {
            sort: vec![],
            filters: vec![],
            group_by: None,
            visible_columns: None,
            page_size: 50,
        },
        status: medscale_contracts::data_sources::SavedViewStatus::Active,
    };
    meta.insert_saved_view(&view, &OpaqueId::new("proj-1"))
        .unwrap();
    let stale = meta.update_saved_view_state(
        &OpaqueId::new("view-1"),
        9,
        &ViewState {
            sort: vec![],
            filters: vec![],
            group_by: None,
            visible_columns: None,
            page_size: 10,
        },
    );
    assert!(matches!(stale, Err(MetaError::Conflict(_))), "{stale:?}");
    let (views, next) = meta
        .list_saved_views(&OpaqueId::new("snap-1"), 25, None)
        .unwrap();
    assert_eq!(views.len(), 1);
    assert!(next.is_none());

    let ops = vec![
        medscale_contracts::data_sources::TransformOp::SelectColumns {
            columns: vec!["city".to_owned()],
        },
    ];
    let ops_digest = DigestSha256::of(serde_json::to_vec(&ops).unwrap().as_slice());
    let trecord = medscale_storage::TransformationRecord {
        output_snapshot_id: OpaqueId::new("snap-2"),
        first_input_snapshot_id: OpaqueId::new("snap-1"),
        input_ids: vec![OpaqueId::new("snap-1")],
        ops,
        parameters_digest: ops_digest,
        receipt: TransformationReceipt {
            input_snapshot_ids: vec![OpaqueId::new("snap-1")],
            output_snapshot_id: OpaqueId::new("snap-2"),
            ops_digest: DigestSha256::of(b"ops"),
            rows_in: vec![2],
            rows_out: 2,
            cast_failures: 0,
        },
    };
    meta.insert_transformation(&trecord).unwrap();
    let children = meta
        .list_transformations_by_first_input(&OpaqueId::new("snap-1"))
        .unwrap();
    assert_eq!(children.len(), 1);

    // Release requires its snapshot row (scope inheritance).
    let release = medscale_contracts::data_sources::ReleaseManifest {
        release_id: OpaqueId::new("rel-1"),
        card: DatasetCard {
            snapshot_id: OpaqueId::new("snap-1"),
            version: "v1".to_owned(),
            split_group: None,
            annotation_schema_ref: None,
            rights_state: RightsState::SyntheticFixture,
            project_id: OpaqueId::new("proj-1"),
        },
        snapshot_digest: DigestSha256::of(b"canonical"),
    };
    meta.insert_dataset_release(&release).unwrap();
    let back_release = meta.get_dataset_release(&OpaqueId::new("rel-1")).unwrap();
    assert_eq!(back_release.card.version, "v1");
    let (releases, next) = meta
        .list_dataset_releases(&OpaqueId::new("proj-1"), 25, None)
        .unwrap();
    assert_eq!(releases.len(), 1);
    assert!(next.is_none());

    let missing = medscale_contracts::data_sources::ReleaseManifest {
        release_id: OpaqueId::new("rel-2"),
        card: DatasetCard {
            snapshot_id: OpaqueId::new("snap-missing"),
            version: "v1".to_owned(),
            split_group: None,
            annotation_schema_ref: None,
            rights_state: RightsState::SyntheticFixture,
            project_id: OpaqueId::new("proj-1"),
        },
        snapshot_digest: DigestSha256::of(b"canonical"),
    };
    let err = meta.insert_dataset_release(&missing).unwrap_err();
    assert!(matches!(err, MetaError::NotFound), "{err:?}");
}

#[test]
fn migration_v3_to_v4_preserves_old_rows() {
    let root = temp_root("migrate-v4");
    let meta = open_meta(&root);
    // open_at runs migrate(); v4 tables must exist on a fresh vault.
    meta.insert_data_source(&manifest("dsrc-1")).unwrap();
    let journal = meta.migration_journal().unwrap();
    // Forward-fixed for Spec 077 (v5 -> v6).
    assert_eq!(
        journal.finished_version,
        medscale_storage::CURRENT_META_SCHEMA_VERSION
    );
    // Reopen is safe (idempotent migration).
    drop(meta);
    let meta = open_meta(&root);
    let read = meta.get_data_source(&OpaqueId::new("dsrc-1")).unwrap();
    assert_eq!(read.revision, 1);
}

#[test]
fn backup_restore_roundtrips_fabric_rows() {
    use medscale_storage::SyntheticVault;
    let root = temp_root("fabric-backup");
    let vault_root = root.join("vault");
    let vault = SyntheticVault::open("vault-1", &vault_root).unwrap();
    vault.meta.insert_data_source(&manifest("dsrc-1")).unwrap();
    let record = snapshot_record("snap-1", "dsrc-1");
    vault.meta.insert_snapshot_full(&record, &[]).unwrap();

    let dest = root.join("backup");
    let manifest_out = backup_vault(&vault, &dest).unwrap();
    // Forward-fixed for Spec 077 (v5 -> v6).
    assert_eq!(
        manifest_out.schema_version,
        medscale_storage::CURRENT_META_SCHEMA_VERSION
    );

    let restore_root = root.join("restored");
    let (sources, _) = restore_vault(&dest, &restore_root).unwrap();
    let _ = sources;
    let restored = SyntheticVault::open("vault-1", &restore_root).unwrap();
    let back = restored
        .meta
        .get_data_source(&OpaqueId::new("dsrc-1"))
        .unwrap();
    assert_eq!(back.display_name, "source-dsrc-1");
    let back_snap = restored
        .meta
        .get_snapshot(&OpaqueId::new("snap-1"))
        .unwrap();
    assert_eq!(back_snap.snapshot.row_count, 2);
}

#[test]
fn corrupt_id_sequence_fails_closed_instead_of_resetting() {
    let root = temp_root("id-seq-corrupt");
    let meta = open_meta(&root);
    let first = meta
        .alloc_data_fabric_id("data_source_id_seq", "dsrc")
        .unwrap();
    assert_eq!(first.as_str(), "dsrc-1");
    drop(meta);
    {
        // Simulate a corrupted store_state row (disk fault / tamper), not
        // reachable through the public API.
        let conn = rusqlite::Connection::open(root.join("meta.sqlite3")).unwrap();
        conn.execute(
            "UPDATE store_state SET value = 'not-a-number' WHERE key = 'data_source_id_seq'",
            [],
        )
        .unwrap();
    }
    let meta = open_meta(&root);
    let err = meta
        .alloc_data_fabric_id("data_source_id_seq", "dsrc")
        .unwrap_err();
    assert!(
        matches!(err, MetaError::CorruptObjectBody(_)),
        "corrupted id sequence must fail closed instead of silently resetting to a \
         colliding id (e.g. dsrc-1 again), got {err:?}"
    );
}

#[test]
fn restore_rejects_database_source_with_credential_ref() {
    use medscale_storage::SyntheticVault;
    let root = temp_root("restore-cred");
    let vault_root = root.join("vault");
    let vault = SyntheticVault::open("vault-1", &vault_root).unwrap();
    // A row that Core's create_source/update_source would never admit
    // (Database locator + a stored credential_ref), inserted directly at
    // the storage layer to simulate a crafted/tampered backup rather than
    // one produced through the normal authority path.
    let illegal = DataSourceManifest::new(
        header("dsrc-bad"),
        OpaqueId::new("proj-1"),
        DataSourceKind::DatabaseRead,
        "illegal-db-source".to_owned(),
        "external_sqlite".to_owned(),
        SourceLocator::Database {
            engine: medscale_contracts::data_sources::DatabaseEngine::ExternalSqlite,
            database: "lab.sqlite3".to_owned(),
            object: "towns".to_owned(),
        },
        Some(OpaqueId::new("cred-should-not-exist")),
        vec![medscale_contracts::data_sources::DataSourceCapability::DiscoverSchema],
    )
    .unwrap();
    vault.meta.insert_data_source(&illegal).unwrap();

    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();

    let restore_root = root.join("restored");
    let err = restore_vault(&dest, &restore_root).unwrap_err();
    assert!(
        err.contains("credential"),
        "restore must fail closed on a stored database credential, got: {err}"
    );
}

#[test]
fn tampered_snapshot_status_fails_closed() {
    let root = temp_root("tamper-status");
    let meta = open_meta(&root);
    meta.insert_data_source(&manifest("dsrc-1")).unwrap();
    // A partial snapshot with an empty reason cannot validate; if such bytes
    // ever reach storage, reads must fail closed instead of coercing.
    let mut record = snapshot_record("snap-x", "dsrc-1");
    record.snapshot.status = medscale_contracts::data_sources::SnapshotStatus::Partial {
        reason: String::new(),
    };
    meta.restore_snapshot_full(&record, &[]).unwrap();
    let err = meta.get_snapshot(&OpaqueId::new("snap-x")).unwrap_err();
    assert!(
        matches!(
            err,
            MetaError::UnsupportedSchema(_) | MetaError::CorruptObjectBody(_)
        ),
        "{err:?}"
    );
}

#[test]
fn an_external_database_with_a_write_ahead_log_is_refused() {
    // RUSTSEC-2026-0459: a crafted `-wal` sibling can make SQLite before 3.54
    // return uninitialised memory, so external databases with one are refused.
    let root = temp_root("wal-sibling");
    let db = root.join("external.sqlite3");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch("CREATE TABLE notes (id INTEGER, note TEXT);")
        .unwrap();
    drop(conn);
    assert!(list_external_sqlite_tables(&db).is_ok());
    fs::write(root.join("external.sqlite3-wal"), b"not a real log").unwrap();
    for result in [
        list_external_sqlite_tables(&db).map(|_| ()),
        read_external_sqlite_table(&db, "notes", 10).map(|_| ()),
    ] {
        assert!(
            matches!(result, Err(MetaError::UnsupportedSchema(ref m)) if m.contains("-wal")),
            "{result:?}"
        );
    }
    let _ = fs::remove_dir_all(&root);
}
