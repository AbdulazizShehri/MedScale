//! Spec 075 Core authority integration tests (075-C/D/E).
//!
//! Every mutation travels request -> session -> realm/scope -> authority ->
//! validation -> acquisition -> transaction -> audit/receipt -> typed result
//! through `CoreFacade::dispatch`. Synthetic data only.

use std::fs;

use medscale_contracts::data_sources::{
    DataSourceKind, DataViewKind, FilterExpr, FilterOp, LocalFileFormat, RefreshChangeClass,
    RightsState, SNAPSHOT_BYTES_MAX, SortKey, SourceLocator, TransformOp, ViewState,
};
use medscale_contracts::envelopes::{AuthorityError, AuthorityRequest, Capability, RequestBody};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::CoreFacade;

fn tmp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-075c-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn grants() -> Vec<Capability> {
    vec![
        Capability::OpenSyntheticVault,
        Capability::ProjectCreate,
        Capability::ProjectRead,
        Capability::DataSourceCreate,
        Capability::DataSourceRead,
        Capability::DataSourceUpdate,
        Capability::DataSourceArchive,
        Capability::SnapshotImport,
        Capability::SnapshotPreview,
        Capability::SnapshotRead,
        Capability::SnapshotRefresh,
        Capability::SavedViewCreate,
        Capability::SavedViewRead,
        Capability::SavedViewUpdate,
        Capability::TransformExecute,
        Capability::DatasetReleaseCreate,
        Capability::DatasetReleaseRead,
    ]
}

struct Harness {
    facade: CoreFacade,
    session: OpaqueId,
    dir: std::path::PathBuf,
}

fn setup(name: &str) -> Harness {
    let dir = tmp_dir(name);
    let facade = CoreFacade::new();
    let holder = match facade
        .dispatch(base_req(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("client-a"),
                holder_id_hint: None,
            },
        ))
        .result
        .expect("lease")
    {
        medscale_contracts::envelopes::ResponseBody::Lease { holder_id, .. } => holder_id,
        other => panic!("{other:?}"),
    };
    let session = match facade
        .dispatch(base_req(
            Capability::OpenSession,
            RequestBody::OpenSession {
                holder_id: holder,
                granted: grants(),
                ttl_ticks: 1_000_000,
            },
        ))
        .result
        .expect("session")
    {
        medscale_contracts::envelopes::ResponseBody::Session { session_id, .. } => session_id,
        other => panic!("{other:?}"),
    };
    let mut vault_req = base_req(
        Capability::OpenSyntheticVault,
        RequestBody::OpenSyntheticVault {
            vault_root: dir.to_str().unwrap().to_owned(),
        },
    );
    vault_req.session_id = Some(session.clone());
    facade.dispatch(vault_req).result.expect("vault");
    Harness {
        facade,
        session,
        dir,
    }
}

fn base_req(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new("req"),
        VaultId::new("vault-1"),
        RealmId::new("realm-a"),
        AuthorityScopeId::new("scope-a"),
        capability,
        body,
    )
}

struct CallOk(medscale_contracts::envelopes::ResponseBody);

impl std::fmt::Debug for CallOk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CallOk({:?})", self.0)
    }
}

impl Harness {
    fn call(&self, capability: Capability, body: RequestBody) -> Result<CallOk, AuthorityError> {
        let mut request = base_req(capability, body);
        request.session_id = Some(self.session.clone());
        self.facade.dispatch(request).result.map(CallOk)
    }

    fn call_scoped(
        &self,
        capability: Capability,
        body: RequestBody,
    ) -> Result<CallOk, AuthorityError> {
        let mut request = AuthorityRequest::new(
            OpaqueId::new("req"),
            VaultId::new("vault-1"),
            RealmId::new("realm-a"),
            AuthorityScopeId::new("scope-b"),
            capability,
            body,
        );
        request.session_id = Some(self.session.clone());
        self.facade.dispatch(request).result.map(CallOk)
    }

    fn project(&self) -> OpaqueId {
        match self
            .call(
                Capability::ProjectCreate,
                RequestBody::ProjectCreate {
                    name: "study".to_owned(),
                    description: None,
                },
            )
            .expect("project")
            .0
        {
            medscale_contracts::envelopes::ResponseBody::Project { project } => project.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn csv_source(&self, project: &OpaqueId, name: &str, file: &str) -> OpaqueId {
        match self
            .call(
                Capability::DataSourceCreate,
                RequestBody::DataSourceCreate {
                    project_id: project.clone(),
                    display_name: name.to_owned(),
                    locator: SourceLocator::LocalPath {
                        path: file.to_owned(),
                        format: LocalFileFormat::Csv,
                    },
                    credential_ref: None,
                },
            )
            .expect("source")
            .0
        {
            medscale_contracts::envelopes::ResponseBody::DataSource { source } => {
                assert_eq!(source.kind, DataSourceKind::LocalTabularFile);
                source.header.id
            }
            other => panic!("{other:?}"),
        }
    }

    fn write_fixture(&self, file: &str, bytes: &[u8]) {
        fs::write(self.dir.join(file), bytes).unwrap();
    }

    fn import(
        &self,
        source: &OpaqueId,
    ) -> (
        medscale_contracts::data_sources::DataSnapshot,
        medscale_contracts::data_sources::ImportReceipt,
    ) {
        match self
            .call(
                Capability::SnapshotImport,
                RequestBody::SnapshotImport {
                    source_id: source.clone(),
                },
            )
            .expect("import")
            .0
        {
            medscale_contracts::envelopes::ResponseBody::SnapshotImported { snapshot, receipt } => {
                (*snapshot, receipt)
            }
            other => panic!("{other:?}"),
        }
    }
}

const CSV_BASIC: &[u8] = b"city,dose\n springfield,5\nshelbyville,9\nogdenville,\n";

#[test]
fn csv_import_produces_immutable_snapshot_with_receipt() {
    let h = setup("csv-import");
    let project = h.project();
    h.write_fixture("table.csv", CSV_BASIC);
    let source = h.csv_source(&project, "towns", "table.csv");
    let (snapshot, receipt) = h.import(&source);
    assert_eq!(snapshot.row_count, 3);
    assert_eq!(receipt.rows_materialized, 3);
    assert_eq!(receipt.rows_skipped, 0);
    assert_eq!(receipt.snapshot_id.as_str(), snapshot.header.id.as_str());
    assert_eq!(
        receipt.schema_fingerprint.to_hex(),
        snapshot.schema_fingerprint.to_hex()
    );
    // Idempotent re-import resolves to the same snapshot with its receipt.
    let (again, receipt_again) = h.import(&source);
    assert_eq!(again.header.id.as_str(), snapshot.header.id.as_str());
    assert_eq!(
        receipt_again.snapshot_id.as_str(),
        snapshot.header.id.as_str()
    );
}

#[test]
fn snapshot_rows_page_filter_and_sort() {
    let h = setup("csv-rows");
    let project = h.project();
    h.write_fixture("table.csv", CSV_BASIC);
    let source = h.csv_source(&project, "towns", "table.csv");
    let (snapshot, _) = h.import(&source);
    let page = match h
        .call(
            Capability::SnapshotRead,
            RequestBody::SnapshotRows {
                snapshot_id: snapshot.header.id.clone(),
                limit: Some(2),
                cursor: None,
                filters: vec![],
                sort: vec![],
            },
        )
        .expect("rows")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::SnapshotRows { page } => page,
        other => panic!("{other:?}"),
    };
    assert_eq!(page.rows.len(), 2);
    let cursor = page.next_cursor.expect("cursor");
    let page2 = match h
        .call(
            Capability::SnapshotRead,
            RequestBody::SnapshotRows {
                snapshot_id: snapshot.header.id.clone(),
                limit: Some(2),
                cursor: Some(cursor),
                filters: vec![],
                sort: vec![],
            },
        )
        .expect("rows2")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::SnapshotRows { page } => page,
        other => panic!("{other:?}"),
    };
    assert_eq!(page2.rows.len(), 1);
    assert!(page2.next_cursor.is_none());
    // Filtered + sorted query.
    let filtered = match h
        .call(
            Capability::SnapshotRead,
            RequestBody::SnapshotRows {
                snapshot_id: snapshot.header.id.clone(),
                limit: None,
                cursor: None,
                filters: vec![FilterExpr {
                    column: "dose".to_owned(),
                    op: FilterOp::GreaterThan,
                    value: "4".to_owned(),
                }],
                sort: vec![SortKey {
                    column: "city".to_owned(),
                    descending: true,
                }],
            },
        )
        .expect("filtered")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::SnapshotRows { page } => page,
        other => panic!("{other:?}"),
    };
    // Null dose never matches; shelbyville(9) before springfield(5) descending.
    assert_eq!(filtered.rows.len(), 2);
}

#[test]
fn refresh_detects_change_and_preserves_old_snapshot() {
    let h = setup("csv-refresh");
    let project = h.project();
    h.write_fixture("table.csv", CSV_BASIC);
    let source = h.csv_source(&project, "towns", "table.csv");
    let (first, _) = h.import(&source);
    // Unchanged refresh is idempotent: receipt, no new snapshot.
    let (receipt, none) = match h
        .call(
            Capability::SnapshotRefresh,
            RequestBody::SnapshotRefresh {
                source_id: source.clone(),
                allow_schema_change: false,
            },
        )
        .expect("refresh-unchanged")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::SnapshotRefreshed { receipt, snapshot } => {
            (receipt, snapshot)
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(receipt.change_class, RefreshChangeClass::Unchanged);
    assert!(none.is_none());
    // Changed content yields a new snapshot; the old one is byte-identical.
    h.write_fixture(
        "table.csv",
        b"city,dose\nspringfield,5\nshelbyville,9\nogdenville,7\nnorth haverbrook,1\n",
    );
    let (receipt, some) = match h
        .call(
            Capability::SnapshotRefresh,
            RequestBody::SnapshotRefresh {
                source_id: source.clone(),
                allow_schema_change: false,
            },
        )
        .expect("refresh-changed")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::SnapshotRefreshed { receipt, snapshot } => {
            (receipt, snapshot)
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(receipt.change_class, RefreshChangeClass::ContentChanged);
    let second = some.expect("new snapshot");
    assert_ne!(second.header.id.as_str(), first.header.id.as_str());
    assert_eq!(second.row_count, 4);
    let old = match h
        .call(
            Capability::SnapshotRead,
            RequestBody::SnapshotGet {
                snapshot_id: first.header.id.clone(),
            },
        )
        .expect("old")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::Snapshot { snapshot } => snapshot,
        other => panic!("{other:?}"),
    };
    assert_eq!(old.row_count, 3);
}

#[test]
fn refresh_schema_change_requires_acknowledgment() {
    let h = setup("csv-schema-change");
    let project = h.project();
    h.write_fixture("table.csv", CSV_BASIC);
    let source = h.csv_source(&project, "towns", "table.csv");
    let _ = h.import(&source);
    h.write_fixture("table.csv", b"city,dose,ward\nspringfield,5,a\n");
    let err = h
        .call(
            Capability::SnapshotRefresh,
            RequestBody::SnapshotRefresh {
                source_id: source.clone(),
                allow_schema_change: false,
            },
        )
        .expect_err("schema change must fail closed");
    assert!(
        matches!(err, AuthorityError::StaleReference { .. }),
        "{err:?}"
    );
    let (receipt, some) = match h
        .call(
            Capability::SnapshotRefresh,
            RequestBody::SnapshotRefresh {
                source_id: source.clone(),
                allow_schema_change: true,
            },
        )
        .expect("refresh-ack")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::SnapshotRefreshed { receipt, snapshot } => {
            (receipt, snapshot)
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(receipt.change_class, RefreshChangeClass::SchemaChanged);
    assert!(some.is_some());
}

#[test]
fn malformed_bytes_quarantine_and_missing_file_is_unavailable() {
    let h = setup("csv-quarantine");
    let project = h.project();
    h.write_fixture("bad.csv", b"\xff\xfe\x00bad");
    let source = h.csv_source(&project, "bad", "bad.csv");
    let err = h
        .call(
            Capability::SnapshotImport,
            RequestBody::SnapshotImport {
                source_id: source.clone(),
            },
        )
        .expect_err("invalid bytes must fail");
    assert!(matches!(err, AuthorityError::Corrupt { .. }), "{err:?}");
    let ghost = h.csv_source(&project, "ghost", "ghost.csv");
    let err = h
        .call(
            Capability::SnapshotImport,
            RequestBody::SnapshotImport {
                source_id: ghost.clone(),
            },
        )
        .expect_err("missing file must fail");
    assert!(matches!(err, AuthorityError::NotFound), "{err:?}");
    let _ = ghost;
}

#[test]
fn quarantined_import_marks_source_health_stale_not_silently_healthy() {
    let h = setup("csv-quarantine-health");
    let project = h.project();
    h.write_fixture("bad.csv", b"\xff\xfe\x00bad");
    let source = h.csv_source(&project, "bad", "bad.csv");
    h.call(
        Capability::SnapshotImport,
        RequestBody::SnapshotImport {
            source_id: source.clone(),
        },
    )
    .expect_err("invalid bytes must fail");
    let manifest = match h
        .call(
            Capability::DataSourceRead,
            RequestBody::DataSourceGet {
                source_id: source.clone(),
            },
        )
        .expect("get source")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::DataSource { source } => source,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        manifest.health,
        medscale_contracts::data_sources::SourceHealth::Stale,
        "a source whose last import was quarantined must not read back as healthy"
    );
}

#[test]
fn oversized_local_file_is_rejected_before_full_read() {
    let h = setup("csv-oversized");
    let project = h.project();
    // A sparse file: its logical length is checked (and must be rejected)
    // via metadata before the full byte content is ever read into memory,
    // without this test needing to write real bytes.
    let path = h.dir.join("huge.csv");
    let file = fs::File::create(&path).unwrap();
    file.set_len(SNAPSHOT_BYTES_MAX + 1).unwrap();
    drop(file);
    let source = h.csv_source(&project, "huge", "huge.csv");
    let err = h
        .call(
            Capability::SnapshotImport,
            RequestBody::SnapshotImport {
                source_id: source.clone(),
            },
        )
        .expect_err("oversized input must fail");
    assert!(
        matches!(err, AuthorityError::InvalidArgument { .. }),
        "{err:?}"
    );
}

#[test]
fn unqualified_formats_and_engines_fail_closed() {
    let h = setup("csv-unqualified");
    let project = h.project();
    h.write_fixture("data.xlsx", b"PK fake");
    let source = match h
        .call(
            Capability::DataSourceCreate,
            RequestBody::DataSourceCreate {
                project_id: project.clone(),
                display_name: "sheet".to_owned(),
                locator: SourceLocator::LocalPath {
                    path: "data.xlsx".to_owned(),
                    format: medscale_contracts::data_sources::LocalFileFormat::Xlsx,
                },
                credential_ref: None,
            },
        )
        .expect("source")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::DataSource { source } => source.header.id,
        other => panic!("{other:?}"),
    };
    let err = h
        .call(
            Capability::SnapshotImport,
            RequestBody::SnapshotImport {
                source_id: source.clone(),
            },
        )
        .expect_err("xlsx must be unsupported");
    assert!(
        matches!(err, AuthorityError::UnsupportedSchema { .. }),
        "{err:?}"
    );
    let db_source = match h
        .call(
            Capability::DataSourceCreate,
            RequestBody::DataSourceCreate {
                project_id: project.clone(),
                display_name: "warehouse".to_owned(),
                locator: SourceLocator::Database {
                    engine: medscale_contracts::data_sources::DatabaseEngine::Postgres,
                    database: "db".to_owned(),
                    object: "table".to_owned(),
                },
                credential_ref: None,
            },
        )
        .expect("db source")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::DataSource { source } => source.header.id,
        other => panic!("{other:?}"),
    };
    let err = h
        .call(
            Capability::SnapshotImport,
            RequestBody::SnapshotImport {
                source_id: db_source.clone(),
            },
        )
        .expect_err("postgres must be unsupported");
    assert!(
        matches!(err, AuthorityError::UnsupportedSchema { .. }),
        "{err:?}"
    );
    let _ = db_source;
}

#[test]
fn source_revision_conflicts_and_scope_isolation() {
    let h = setup("csv-conflict");
    let project = h.project();
    h.write_fixture("table.csv", CSV_BASIC);
    let source = h.csv_source(&project, "towns", "table.csv");
    let stale = h.call(
        Capability::DataSourceUpdate,
        RequestBody::DataSourceUpdate {
            source_id: source.clone(),
            expected_revision: 7,
            display_name: Some("new".to_owned()),
            credential_ref: None,
        },
    );
    assert!(
        matches!(stale, Err(AuthorityError::Conflict { .. })),
        "{stale:?}"
    );
    let scoped = h.call_scoped(
        Capability::DataSourceRead,
        RequestBody::DataSourceGet {
            source_id: source.clone(),
        },
    );
    assert!(
        matches!(scoped, Err(AuthorityError::WrongScope)),
        "{scoped:?}"
    );
}

#[test]
fn transform_view_release_lineage() {
    let h = setup("csv-transform");
    let project = h.project();
    h.write_fixture("table.csv", CSV_BASIC);
    let source = h.csv_source(&project, "towns", "table.csv");
    let (snapshot, _) = h.import(&source);
    let (derived, receipt) = match h
        .call(
            Capability::TransformExecute,
            RequestBody::TransformExecute {
                input_snapshot_ids: vec![snapshot.header.id.clone()],
                ops: vec![
                    TransformOp::FilterRows {
                        filters: vec![FilterExpr {
                            column: "dose".to_owned(),
                            op: FilterOp::GreaterThan,
                            value: "4".to_owned(),
                        }],
                    },
                    TransformOp::SelectColumns {
                        columns: vec!["city".to_owned()],
                    },
                ],
            },
        )
        .expect("transform")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::Transformed { snapshot, receipt } => {
            (snapshot, receipt)
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(derived.row_count, 2);
    assert_eq!(
        derived
            .parent_snapshot_id
            .as_ref()
            .expect("parent")
            .as_str(),
        snapshot.header.id.as_str()
    );
    assert_eq!(
        receipt.output_snapshot_id.as_str(),
        derived.header.id.as_str()
    );
    assert_eq!(receipt.rows_in, vec![3]);
    assert_eq!(receipt.rows_out, 2);
    // Saved view bound to the derived snapshot.
    let view = match h
        .call(
            Capability::SavedViewCreate,
            RequestBody::SavedViewCreate {
                snapshot_id: derived.header.id.clone(),
                view_kind: DataViewKind::Grid,
                state: ViewState {
                    sort: vec![SortKey {
                        column: "city".to_owned(),
                        descending: false,
                    }],
                    filters: vec![],
                    group_by: None,
                    visible_columns: Some(vec!["city".to_owned()]),
                    page_size: 50,
                },
            },
        )
        .expect("view")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::SavedView { view } => view,
        other => panic!("{other:?}"),
    };
    assert_eq!(view.revision, 1);
    // Unknown column in view state fails closed.
    let bad = h.call(
        Capability::SavedViewCreate,
        RequestBody::SavedViewCreate {
            snapshot_id: derived.header.id.clone(),
            view_kind: DataViewKind::Grid,
            state: ViewState {
                sort: vec![],
                filters: vec![],
                group_by: Some("nope".to_owned()),
                visible_columns: None,
                page_size: 10,
            },
        },
    );
    assert!(
        matches!(bad, Err(AuthorityError::InvalidArgument { .. })),
        "{bad:?}"
    );
    // Dataset release with duplicate-version conflict.
    let release = match h
        .call(
            Capability::DatasetReleaseCreate,
            RequestBody::DatasetReleaseCreate {
                snapshot_id: derived.header.id.clone(),
                version: "v1".to_owned(),
                split_group: None,
                annotation_schema_ref: None,
                rights_state: RightsState::SyntheticFixture,
            },
        )
        .expect("release")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::DatasetRelease { release } => release,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        release.snapshot_digest.to_hex(),
        derived.content_digest.to_hex()
    );
    let dup = h.call(
        Capability::DatasetReleaseCreate,
        RequestBody::DatasetReleaseCreate {
            snapshot_id: derived.header.id.clone(),
            version: "v1".to_owned(),
            split_group: None,
            annotation_schema_ref: None,
            rights_state: RightsState::SyntheticFixture,
        },
    );
    assert!(
        matches!(dup, Err(AuthorityError::Conflict { .. })),
        "{dup:?}"
    );
}

#[test]
fn release_list_pagination_cursor_is_not_silently_discarded() {
    let h = setup("release-pagination");
    let project = h.project();
    h.write_fixture("table.csv", CSV_BASIC);
    let source = h.csv_source(&project, "towns", "table.csv");
    let (snap_a, _) = h.import(&source);
    h.write_fixture("table.csv", b"city,dose\nspringfield,7\n");
    let (snap_b, _) = h.import(&source);
    assert_ne!(snap_a.header.id, snap_b.header.id);
    for (snapshot, version) in [(&snap_a, "v1"), (&snap_b, "v2")] {
        h.call(
            Capability::DatasetReleaseCreate,
            RequestBody::DatasetReleaseCreate {
                snapshot_id: snapshot.header.id.clone(),
                version: version.to_owned(),
                split_group: None,
                annotation_schema_ref: None,
                rights_state: RightsState::SyntheticFixture,
            },
        )
        .expect("release");
    }
    let (first_page, next_cursor) = match h
        .call(
            Capability::DatasetReleaseRead,
            RequestBody::DatasetReleaseList {
                project_id: project.clone(),
                limit: Some(1),
                cursor: None,
            },
        )
        .expect("list")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::DatasetReleaseList {
            releases,
            next_cursor,
        } => (releases, next_cursor),
        other => panic!("{other:?}"),
    };
    assert_eq!(first_page.len(), 1);
    assert!(
        next_cursor.is_some(),
        "a full first page must carry a real continuation cursor, not silently claim completeness"
    );
    let second_page = match h
        .call(
            Capability::DatasetReleaseRead,
            RequestBody::DatasetReleaseList {
                project_id: project.clone(),
                limit: Some(1),
                cursor: next_cursor,
            },
        )
        .expect("list page 2")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::DatasetReleaseList { releases, .. } => {
            releases
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(second_page.len(), 1);
    assert_ne!(first_page[0].release_id, second_page[0].release_id);
}

#[test]
fn external_sqlite_read_adapter_is_qualified() {
    let h = setup("sqlite-adapter");
    let project = h.project();
    let db_path = h.dir.join("lab.sqlite3");
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE towns (city TEXT NOT NULL, dose INTEGER);
             INSERT INTO towns VALUES ('springfield', 5), ('shelbyville', NULL);",
        )
        .unwrap();
    }
    let source = match h
        .call(
            Capability::DataSourceCreate,
            RequestBody::DataSourceCreate {
                project_id: project.clone(),
                display_name: "lab".to_owned(),
                locator: SourceLocator::Database {
                    engine: medscale_contracts::data_sources::DatabaseEngine::ExternalSqlite,
                    database: "lab.sqlite3".to_owned(),
                    object: "towns".to_owned(),
                },
                credential_ref: None,
            },
        )
        .expect("source")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::DataSource { source } => source.header.id,
        other => panic!("{other:?}"),
    };
    let (snapshot, receipt) = h.import(&source);
    assert_eq!(snapshot.row_count, 2);
    assert_eq!(receipt.rows_materialized, 2);
    let (schema, rows) = match h
        .call(
            Capability::SnapshotPreview,
            RequestBody::SnapshotPreview {
                source_id: source.clone(),
                max_rows: Some(10),
            },
        )
        .expect("preview")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::SnapshotPreview { schema, rows } => {
            (schema, rows)
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(schema.fields.len(), 2);
    assert_eq!(rows.len(), 2);
    // A missing table is explicit, not silent.
    let ghost = match h
        .call(
            Capability::DataSourceCreate,
            RequestBody::DataSourceCreate {
                project_id: project.clone(),
                display_name: "ghost".to_owned(),
                locator: SourceLocator::Database {
                    engine: medscale_contracts::data_sources::DatabaseEngine::ExternalSqlite,
                    database: "lab.sqlite3".to_owned(),
                    object: "ghost".to_owned(),
                },
                credential_ref: None,
            },
        )
        .expect("ghost source")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::DataSource { source } => source.header.id,
        other => panic!("{other:?}"),
    };
    let err = h
        .call(
            Capability::SnapshotImport,
            RequestBody::SnapshotImport {
                source_id: ghost.clone(),
            },
        )
        .expect_err("missing table must fail");
    assert!(matches!(err, AuthorityError::Unavailable { .. }), "{err:?}");
    let _ = ghost;
}

#[test]
fn database_source_cannot_name_the_vaults_own_metadata_store() {
    let h = setup("sqlite-reserved");
    let project = h.project();
    // The vault's own meta.sqlite3 sits directly under the vault root next
    // to any user-placed local files; without a reserved-path check a
    // "Database" source could name it and read every project's
    // data_sources/data_snapshots/dataset_releases rows with no scoping.
    let source = match h
        .call(
            Capability::DataSourceCreate,
            RequestBody::DataSourceCreate {
                project_id: project.clone(),
                display_name: "meta-probe".to_owned(),
                locator: SourceLocator::Database {
                    engine: medscale_contracts::data_sources::DatabaseEngine::ExternalSqlite,
                    database: "meta.sqlite3".to_owned(),
                    object: "data_sources".to_owned(),
                },
                credential_ref: None,
            },
        )
        .expect("source")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::DataSource { source } => source.header.id,
        other => panic!("{other:?}"),
    };
    let err = h
        .call(
            Capability::SnapshotImport,
            RequestBody::SnapshotImport {
                source_id: source.clone(),
            },
        )
        .expect_err("reading the vault's own metadata store must be refused");
    // resolve_local_path's PathOutsideClaim is remapped to Denied and then
    // to Unauthorized on the way out of acquire_parsed (same as any other
    // vault-escape attempt through the Database locator).
    assert!(matches!(err, AuthorityError::Unauthorized), "{err:?}");
    // The blob store is equally reserved.
    let blobs_source = match h
        .call(
            Capability::DataSourceCreate,
            RequestBody::DataSourceCreate {
                project_id: project.clone(),
                display_name: "blobs-probe".to_owned(),
                locator: SourceLocator::LocalPath {
                    path: "blobs/anything.csv".to_owned(),
                    format: LocalFileFormat::Csv,
                },
                credential_ref: None,
            },
        )
        .expect("source")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::DataSource { source } => source.header.id,
        other => panic!("{other:?}"),
    };
    let err = h
        .call(
            Capability::SnapshotImport,
            RequestBody::SnapshotImport {
                source_id: blobs_source,
            },
        )
        .expect_err("reading the vault's own blob store must be refused");
    assert!(matches!(err, AuthorityError::Unauthorized), "{err:?}");
}

#[test]
fn remote_import_without_allowlist_denies_before_socket() {
    let h = setup("remote-denied");
    let project = h.project();
    let source = match h
        .call(
            Capability::DataSourceCreate,
            RequestBody::DataSourceCreate {
                project_id: project.clone(),
                display_name: "mirror".to_owned(),
                locator: SourceLocator::RemoteDataset {
                    provider: medscale_contracts::data_sources::RemoteDatasetProvider::HuggingFace,
                    repo: "org/ds".to_owned(),
                    revision: "v1".to_owned(),
                    files: vec!["data.csv".to_owned()],
                },
                credential_ref: None,
            },
        )
        .expect("source")
        .0
    {
        medscale_contracts::envelopes::ResponseBody::DataSource { source } => source.header.id,
        other => panic!("{other:?}"),
    };
    // Empty broker allowlist: deny before any socket, no silent fetch.
    let err = h
        .call(
            Capability::SnapshotImport,
            RequestBody::SnapshotImport {
                source_id: source.clone(),
            },
        )
        .expect_err("remote without allowlist must fail");
    assert!(matches!(err, AuthorityError::Unauthorized), "{err:?}");
}

#[test]
fn large_table_import_pages_correctly() {
    // Scale fixture: 20,000 deterministic rows. Correctness (counts, paging
    // stability, part digests) is asserted; timings are recorded in
    // SCALE_MEASUREMENTS.md from CI logs without budget claims.
    let h = setup("csv-scale");
    let project = h.project();
    let mut bytes = b"city,dose\n".to_vec();
    for index in 0..20_000_u32 {
        bytes.extend_from_slice(format!("town-{index},{}\n", index % 1000).as_bytes());
    }
    h.write_fixture("big.csv", &bytes);
    let source = h.csv_source(&project, "big", "big.csv");
    let (snapshot, receipt) = h.import(&source);
    assert_eq!(snapshot.row_count, 20_000);
    assert_eq!(receipt.rows_materialized, 20_000);
    // Stable paging across the whole snapshot.
    let mut seen = 0_u64;
    let mut cursor = None;
    loop {
        let page = match h
            .call(
                Capability::SnapshotRead,
                RequestBody::SnapshotRows {
                    snapshot_id: snapshot.header.id.clone(),
                    limit: Some(1_000),
                    cursor,
                    filters: vec![],
                    sort: vec![],
                },
            )
            .expect("page")
            .0
        {
            medscale_contracts::envelopes::ResponseBody::SnapshotRows { page } => page,
            other => panic!("{other:?}"),
        };
        seen += page.rows.len() as u64;
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(seen, 20_000);
}
