use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::data_sources::{
    AcquireOutcome, DataSourceKind, DataSourceManifest, DataViewKind, DatabaseEngine, DatasetCard,
    FieldType, FilterExpr, FilterOp, LocalFileFormat, RefreshChangeClass, RemoteDatasetProvider,
    RightsState, SchemaField, SortKey, SourceHealth, SourceLocator, SourceRevisionBinding,
    SourceSchema, SourceStatus, TransformOp, ViewState, canonical_snapshot_bytes,
    effective_list_limit, effective_rows_limit, fingerprint_schema, parse_row_cursor,
    render_row_cursor, validate_display_name,
};
use medscale_contracts::envelopes::{AuthorityError, Capability};
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};

fn header(id: &str) -> ObjectHeader {
    ObjectHeader {
        id: OpaqueId::new(id),
        schema_version: AUTHORITY_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-a"),
        authority_scope_id: AuthorityScopeId::new("scope-a"),
    }
}

fn scope_a() -> AuthorityScopeId {
    AuthorityScopeId::new("scope-a")
}

fn text_field(name: &str) -> SchemaField {
    SchemaField {
        name: name.to_owned(),
        field_type: FieldType::Text,
        nullable: true,
        declared_unit: None,
    }
}

fn csv_locator() -> SourceLocator {
    SourceLocator::LocalPath {
        path: "fixtures/table.csv".to_owned(),
        format: LocalFileFormat::Csv,
    }
}

fn manifest() -> DataSourceManifest {
    DataSourceManifest::new(
        header("dsrc-1"),
        OpaqueId::new("proj-1"),
        DataSourceKind::LocalTabularFile,
        "lab table".to_owned(),
        "csv".to_owned(),
        csv_locator(),
        None,
        vec![
            medscale_contracts::data_sources::DataSourceCapability::DiscoverSchema,
            medscale_contracts::data_sources::DataSourceCapability::ImportSnapshot,
        ],
    )
    .expect("manifest must validate")
}

#[test]
fn kind_vocabulary_roundtrips_and_rejects_unknown() {
    for (kind, name) in [
        (DataSourceKind::LocalTabularFile, "local_tabular_file"),
        (DataSourceKind::DatabaseRead, "database_read"),
        (DataSourceKind::RemoteDataset, "remote_dataset"),
    ] {
        assert_eq!(kind.as_str(), name);
        assert_eq!(DataSourceKind::parse(name).expect("parse"), kind);
    }
    assert!(DataSourceKind::parse("cloud_sync").is_err());
}

#[test]
fn format_vocabulary_roundtrips_and_rejects_unknown() {
    for (format, name) in [
        (LocalFileFormat::Csv, "csv"),
        (LocalFileFormat::Tsv, "tsv"),
        (LocalFileFormat::JsonLines, "json_lines"),
        (LocalFileFormat::Json, "json"),
        (LocalFileFormat::Parquet, "parquet"),
        (LocalFileFormat::ArrowIpc, "arrow_ipc"),
        (LocalFileFormat::Xlsx, "xlsx"),
    ] {
        assert_eq!(format.as_str(), name);
        assert_eq!(LocalFileFormat::parse(name).expect("parse"), format);
    }
    assert!(LocalFileFormat::parse("sqlite").is_err());
}

#[test]
fn engine_and_provider_vocabularies_reject_unknown() {
    assert_eq!(
        DatabaseEngine::parse("external_sqlite").expect("parse"),
        DatabaseEngine::ExternalSqlite
    );
    assert!(DatabaseEngine::parse("oracle").is_err());
    assert_eq!(
        RemoteDatasetProvider::parse("hugging_face").expect("parse"),
        RemoteDatasetProvider::HuggingFace
    );
    assert_eq!(
        RemoteDatasetProvider::parse("kaggle").expect("parse"),
        RemoteDatasetProvider::Kaggle
    );
    assert!(RemoteDatasetProvider::parse("s3").is_err());
}

#[test]
fn view_kind_and_rights_vocabularies_reject_unknown() {
    for (kind, name) in [
        (DataViewKind::Grid, "grid"),
        (DataViewKind::Form, "form"),
        (DataViewKind::Gallery, "gallery"),
        (DataViewKind::Kanban, "kanban"),
        (DataViewKind::Calendar, "calendar"),
        (DataViewKind::Summary, "summary"),
    ] {
        assert_eq!(kind.as_str(), name);
        assert_eq!(DataViewKind::parse(name).expect("parse"), kind);
    }
    assert!(DataViewKind::parse("graph").is_err());
    assert!(RightsState::parse("public_domain").is_err());
}

#[test]
fn manifest_create_assigns_revision_one_and_matches_scope() {
    let manifest = manifest();
    assert_eq!(manifest.revision, 1);
    assert_eq!(manifest.status, SourceStatus::Active);
    assert_eq!(manifest.health, SourceHealth::Healthy);
    assert!(manifest.scope_matches(&scope_a()));
    assert!(!manifest.scope_matches(&AuthorityScopeId::new("scope-b")));
}

#[test]
fn manifest_rejects_locator_kind_mismatch() {
    let err = DataSourceManifest::new(
        header("dsrc-2"),
        OpaqueId::new("proj-1"),
        DataSourceKind::DatabaseRead,
        "db".to_owned(),
        "external_sqlite".to_owned(),
        csv_locator(),
        None,
        vec![medscale_contracts::data_sources::DataSourceCapability::BoundedQuery],
    )
    .expect_err("locator/kind mismatch must fail");
    assert!(err.contains("locator kind"), "{err}");
}

#[test]
fn manifest_rejects_bad_names_and_empty_capabilities() {
    assert!(validate_display_name("").is_err());
    assert!(validate_display_name("   ").is_err());
    assert!(validate_display_name(&"n".repeat(129)).is_err());
    let err = DataSourceManifest::new(
        header("dsrc-3"),
        OpaqueId::new("proj-1"),
        DataSourceKind::LocalTabularFile,
        "ok".to_owned(),
        "csv".to_owned(),
        csv_locator(),
        None,
        vec![],
    )
    .expect_err("empty capabilities must fail");
    assert!(err.contains("capabilities"), "{err}");
}

#[test]
fn locator_validation_rejects_nul_and_empty_files() {
    let bad = SourceLocator::LocalPath {
        path: "a\0b".to_owned(),
        format: LocalFileFormat::Csv,
    };
    assert!(bad.validate().is_err());
    let bad_remote = SourceLocator::RemoteDataset {
        provider: RemoteDatasetProvider::HuggingFace,
        repo: "org/ds".to_owned(),
        revision: "v1".to_owned(),
        files: vec![],
    };
    assert!(bad_remote.validate().is_err());
    let ok_remote = SourceLocator::RemoteDataset {
        provider: RemoteDatasetProvider::Kaggle,
        repo: "owner/slug".to_owned(),
        revision: "3".to_owned(),
        files: vec!["data.csv".to_owned()],
    };
    assert!(ok_remote.validate().is_ok());
    assert_eq!(ok_remote.kind(), DataSourceKind::RemoteDataset);
}

#[test]
fn remote_dataset_file_names_reject_traversal_and_absolute_paths() {
    for bad_file in ["../secret", "a/../../escape", "/etc/passwd", "sub/../x"] {
        let locator = SourceLocator::RemoteDataset {
            provider: RemoteDatasetProvider::HuggingFace,
            repo: "org/ds".to_owned(),
            revision: "v1".to_owned(),
            files: vec![bad_file.to_owned()],
        };
        assert!(
            locator.validate().is_err(),
            "expected rejection for {bad_file:?}"
        );
    }
    let ok = SourceLocator::RemoteDataset {
        provider: RemoteDatasetProvider::HuggingFace,
        repo: "org/ds".to_owned(),
        revision: "v1".to_owned(),
        files: vec!["nested/data.csv".to_owned()],
    };
    assert!(ok.validate().is_ok());
}

#[test]
fn schema_fingerprint_is_deterministic_and_validated() {
    let fields = vec![text_field("city"), text_field("dose")];
    let fp = fingerprint_schema(&fields);
    assert_eq!(fp, fingerprint_schema(&fields));
    let schema = SourceSchema {
        source_id: OpaqueId::new("dsrc-1"),
        schema_fingerprint: fp,
        fields: fields.clone(),
    };
    assert!(schema.validate().is_ok());
    let mut tampered = schema.clone();
    tampered.schema_fingerprint = DigestSha256::of(b"other");
    assert!(tampered.validate().is_err());
    let dup = SourceSchema {
        source_id: OpaqueId::new("dsrc-1"),
        schema_fingerprint: fingerprint_schema(&[text_field("a"), text_field("a")]),
        fields: vec![text_field("a"), text_field("a")],
    };
    assert!(dup.validate().is_err());
}

#[test]
fn revision_helper_reuses_single_definition() {
    let manifest = manifest();
    assert_eq!(manifest.check_mutation(1).expect("revision"), 2);
    assert!(manifest.check_mutation(7).is_err());
    assert!(!manifest.status.can_transition_to(SourceStatus::Active));
    assert!(manifest.status.can_transition_to(SourceStatus::Archived));
}

#[test]
fn source_revision_bindings_validate() {
    let local = SourceRevisionBinding::LocalFile {
        digest: DigestSha256::of(b"bytes"),
        byte_length: 5,
    };
    assert!(local.validate().is_ok());
    let remote = SourceRevisionBinding::Remote {
        provider: RemoteDatasetProvider::HuggingFace,
        repo: "org/ds".to_owned(),
        revision: "abc".to_owned(),
        file_digests: vec![],
    };
    assert!(remote.validate().is_err());
}

#[test]
fn transform_ops_validate_against_schema() {
    let schema = vec![text_field("city"), text_field("dose")];
    let select = TransformOp::SelectColumns {
        columns: vec!["city".to_owned()],
    };
    assert!(select.validate(&schema).is_ok());
    let bad = TransformOp::DropColumns {
        columns: vec!["nope".to_owned()],
    };
    assert!(bad.validate(&schema).is_err());
    let rename = TransformOp::RenameColumn {
        from: "city".to_owned(),
        to: "dose".to_owned(),
    };
    assert!(rename.validate(&schema).is_err());
    let cast = TransformOp::CastType {
        column: "dose".to_owned(),
        to: FieldType::Integer,
        strict: true,
    };
    assert!(cast.validate(&schema).is_ok());
    let transform = medscale_contracts::data_sources::DataTransformation {
        input_snapshot_ids: vec![OpaqueId::new("snap-1")],
        ops: vec![select],
        parameters_digest: DigestSha256::of(b"ops"),
    };
    assert!(transform.validate().is_ok());
}

#[test]
fn view_state_validates_columns_and_bounds() {
    let schema = vec![text_field("city"), text_field("dose")];
    let state = ViewState {
        sort: vec![SortKey {
            column: "city".to_owned(),
            descending: false,
        }],
        filters: vec![FilterExpr {
            column: "dose".to_owned(),
            op: FilterOp::Equals,
            value: "5".to_owned(),
        }],
        group_by: Some("city".to_owned()),
        visible_columns: Some(vec!["city".to_owned()]),
        page_size: 50,
    };
    assert!(state.validate(&schema).is_ok());
    let bad_page = ViewState {
        page_size: 0,
        ..state.clone()
    };
    assert!(bad_page.validate(&schema).is_err());
    let bad_col = ViewState {
        group_by: Some("nope".to_owned()),
        ..state.clone()
    };
    assert!(bad_col.validate(&schema).is_err());
    assert!(FilterOp::parse("equals").is_ok());
    assert!(FilterOp::parse("regex").is_err());
}

#[test]
fn dataset_card_validates_bounds() {
    let card = DatasetCard {
        snapshot_id: OpaqueId::new("snap-1"),
        version: "v1".to_owned(),
        split_group: None,
        annotation_schema_ref: None,
        rights_state: RightsState::SyntheticFixture,
        project_id: OpaqueId::new("proj-1"),
    };
    assert!(card.validate().is_ok());
    let bad = DatasetCard {
        version: "".to_owned(),
        ..card
    };
    assert!(bad.validate().is_err());
}

#[test]
fn canonical_snapshot_bytes_are_deterministic() {
    use medscale_contracts::data_sources::CellValue;
    let fields = vec![text_field("city")];
    let rows = vec![vec![CellValue::Text("x".to_owned())]];
    let a = canonical_snapshot_bytes(&fields, &rows);
    let b = canonical_snapshot_bytes(&fields, &rows);
    assert!(!a.is_empty());
    assert_eq!(a, b);
    assert_eq!(DigestSha256::of(&a), DigestSha256::of(&b));
}

#[test]
fn cursor_and_limit_helpers_behave() {
    assert_eq!(effective_rows_limit(None), 100);
    assert_eq!(effective_rows_limit(Some(0)), 1);
    assert_eq!(effective_rows_limit(Some(99_999)), 1_000);
    assert_eq!(effective_list_limit(None), 25);
    assert_eq!(parse_row_cursor(&None).expect("none"), 0);
    let cursor = render_row_cursor(500);
    assert_eq!(parse_row_cursor(&Some(cursor)).expect("offset"), 500);
    assert!(parse_row_cursor(&Some("bogus".to_owned())).is_err());
}

#[test]
fn acquire_outcome_and_refresh_classes_have_stable_names() {
    assert_eq!(AcquireOutcome::Quarantined.as_str(), "quarantined");
    assert_eq!(RefreshChangeClass::SchemaChanged.as_str(), "schema_changed");
}

#[test]
fn manifest_rejects_unknown_fields_on_decode() {
    let value = serde_json::json!({
        "header": {
            "id": "dsrc-1",
            "schema_version": AUTHORITY_SCHEMA_VERSION,
            "realm_id": "realm-a",
            "authority_scope_id": "scope-a"
        },
        "revision": 1,
        "project_id": "proj-1",
        "kind": "local_tabular_file",
        "display_name": "lab table",
        "format_or_engine": "csv",
        "locator": {"locator": "local_path", "path": "f.csv", "format": "csv"},
        "credential_ref": null,
        "capabilities": ["import_snapshot"],
        "health": "healthy",
        "status": "active",
        "extra_unknown": true
    });
    let decoded: Result<DataSourceManifest, _> = serde_json::from_value(value);
    assert!(decoded.is_err(), "unknown fields must fail closed");
}

#[test]
fn new_capabilities_exist_and_reads_are_session_free() {
    for cap in [
        Capability::DataSourceCreate,
        Capability::DataSourceRead,
        Capability::SnapshotImport,
        Capability::TransformExecute,
        Capability::DatasetReleaseCreate,
    ] {
        assert!(Capability::operator_grants().contains(&cap));
    }
    for cap in [
        Capability::DataSourceRead,
        Capability::SnapshotPreview,
        Capability::SnapshotRead,
        Capability::SavedViewRead,
        Capability::DatasetReleaseRead,
    ] {
        assert!(
            !cap.requires_client_session(),
            "{cap:?} must be read/health"
        );
    }
    assert!(Capability::TransformExecute.requires_client_session());
}

#[test]
fn cancelled_error_variant_exists_and_is_distinct() {
    let err = AuthorityError::Cancelled {
        message: "caller cancelled".to_owned(),
    };
    assert!(matches!(err, AuthorityError::Cancelled { .. }));
    assert!(!matches!(
        err,
        AuthorityError::Unavailable { .. } | AuthorityError::Internal { .. }
    ));
}
