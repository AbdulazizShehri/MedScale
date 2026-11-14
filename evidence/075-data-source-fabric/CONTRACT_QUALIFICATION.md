# CONTRACT_QUALIFICATION — Spec 075

## Frozen field mapping (T075-01, FROZEN_FOR_075)

| Contract item | Rust path | Tests |
|---|---|---|
| `DataSourceKind` + `parse` | `medscale-contracts/src/data_sources.rs` | `contracts/tests/data_sources_075.rs: kind_vocabulary_roundtrips_and_rejects_unknown` |
| `LocalFileFormat` + `parse` | same | `format_vocabulary_roundtrips_and_rejects_unknown` |
| `DatabaseEngine`, `RemoteDatasetProvider` + `parse` | same | `engine_and_provider_vocabularies_reject_unknown` |
| `DataSourceCapability`, `SourceHealth`, `SourceStatus` + transitions | same | manifest create test + `revision_helper_reuses_single_definition` |
| `SourceLocator` + `validate` + `kind` | same | `locator_validation_rejects_nul_and_empty_files`, `manifest_rejects_locator_kind_mismatch` |
| `FieldType` + `parse` | same | transform/view tests (parse covered via cast DSL in CLI + Core tests) |
| `SchemaField` + `validate` + `canonical_descriptor` | same | `schema_fingerprint_is_deterministic_and_validated` |
| `SourceSchema` + `validate`, `fingerprint_schema` | same | same |
| `SourceRevisionBinding` + `validate` | same | `source_revision_bindings_validate` |
| `CellValue` + `validate` | same | canonical bytes test + Core transform tests |
| `SnapshotCanonicalDoc`, `canonical_snapshot_bytes`, `canonical_part_bytes` | same | `canonical_snapshot_bytes_are_deterministic` |
| `SnapshotStatus` + `validate` | same | storage tamper test (`tampered_snapshot_status_fails_closed`) |
| `DataSnapshot` + `validate` | same | Core import test (row counts, digest equality) |
| `SnapshotPart` + `validate` | same | storage snapshot test (part row range) |
| `AcquireOutcome`, `RefreshChangeClass` | same | `acquire_outcome_and_refresh_classes_have_stable_names` |
| `ImportReceipt`/`RefreshReceipt` + `validate` | same | Core import/refresh tests (receipt field equality) |
| `TransformOp` + `validate`, `FilterExpr`, `FilterOp` + `parse`, `SortKey` | same | `transform_ops_validate_against_schema`, `view_state_validates_columns_and_bounds` |
| `DataTransformation` + `validate`, `TransformationReceipt` + `validate` | same | Core transform test (lineage, row counts, cast failures) |
| `SnapshotLineage` union | same | Core `lineage_for` (covered indirectly; transformation/import/refresh receipts) |
| `DataSourceManifest::new` + `check_mutation` + `scope_matches` | same | `manifest_create_assigns_revision_one_and_matches_scope`, `manifest_rejects_bad_names_and_empty_capabilities` |
| `DataViewKind` + `parse`, `ViewState` + `validate`, `SavedDataView` | same | `view_kind_and_rights_vocabularies_reject_unknown`, `view_state_validates_columns_and_bounds`, Core view tests |
| `RightsState` + `parse`, `DatasetCard` + `validate`, `ReleaseManifest` | same | `dataset_card_validates_bounds`, Core release tests |
| Summaries (`DataSourceSummary`, `SnapshotSummary`, `SavedViewSummary`, `DatasetReleaseSummary`) | same | CLI/Core list tests |
| `SnapshotRowPage`, limit/cursor helpers | same | `cursor_and_limit_helpers`, Core paging test |
| `deny_unknown_fields` on authority structs | same | `manifest_rejects_unknown_fields_on_decode` |
| 14 `Capability` variants + read/health classification + operator grants | `contracts/src/envelopes/mod.rs` | `new_capabilities_exist_and_reads_are_session_free` |
| 18 `RequestBody` + 14 `ResponseBody` variants + `capability_matches` pairs | `envelopes/mod.rs`, `core/src/authority/facade.rs` | Core dispatch tests (every op travels a typed pair; mismatch is denied) |
| `AuthorityError::Cancelled` (single additive variant) | `envelopes/mod.rs` | `cancelled_error_variant_exists_and_is_distinct`, CLI `cancelled` code, Desktop `Cancelled:` status |
| `EgressPurpose::DatasetMirrorRead` (single additive purpose) | `contracts/src/network/mod.rs` | remote adapter tests (brokered fetch + deny) |

## Invariant suite result

```text
CONTRACT_TESTS = PASS: exact-head CI run 35580861670 (head 9f84a6e) green
6/6 required jobs; PR #129 merged as 89a88cf; post-merge main run
35582548200 green 6/6. Includes the exact-range-review addition
`remote_dataset_file_names_reject_traversal_and_absolute_paths`.
COMMAND = cargo test -p medscale-contracts --locked
```

## Serialization/versioning proof

- All authority structs carry `deny_unknown_fields` (decode test above).
- Durable schema version for 075 rows: `DATA_SOURCE_SCHEMA_VERSION = 1` (storage schema v4).
- Unknown enum strings fail closed via `parse_*` (tests above).
- JSON CLI output is versioned through existing `CliJsonError`/stable JSON conventions (CLI tests + logs).
