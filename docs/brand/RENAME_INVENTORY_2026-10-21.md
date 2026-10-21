# MedScale source-name migration inventory

Generated using `py -3 scripts/audit_medscale_rename.py` against tracked files.

**Interpretation:** A match is not proof of non-compliant branding. Immutable measurements, third-party copyright notices, model identifiers, and required scientific citations must remain accurate. This inventory tracks first-party migration work without modifying source content.

- Files containing tracked source-name terms: **430**
- Occurrences: **13710**

| Disposition | Matched files |
|---|---:|
| ACTIVE_CLI | 1 |
| ACTIVE_PRODUCT | 3 |
| BUILD_OR_EVALUATION | 12 |
| COMPATIBILITY_TEST | 23 |
| FIRST_PARTY_TECHNICAL | 36 |
| HISTORICAL_OR_SCIENTIFIC_RECORD | 334 |
| LEGACY_UI | 1 |
| OTHER_REVIEW | 16 |
| SOURCE_LEGAL_OR_VENDOR | 4 |

## First-party and customer-facing files requiring review

| Category | Path | Matching terms |
|---|---|---|
| OTHER_REVIEW | `.cursor/permissions.json` | mesc: 1 |
| OTHER_REVIEW | `.cursor/rules/00-medscale-foundation.mdc` | mesc: 2 |
| OTHER_REVIEW | `.cursor/rules/05-authority-supersession.mdc` | mesc: 1 |
| OTHER_REVIEW | `.cursor/rules/30-source-acquisition.mdc` | mesc: 1, openmed: 1 |
| OTHER_REVIEW | `.gitattributes` | mesc: 1 |
| OTHER_REVIEW | `AGENTS.md` | mesc: 3, openmed: 2 |
| OTHER_REVIEW | `CURSOR.md` | openmed: 1 |
| OTHER_REVIEW | `DESIGN.md` | mesc: 1 |
| OTHER_REVIEW | `README.md` | mesc: 1, openmed: 2 |
| ACTIVE_PRODUCT | `apps/desktop-tauri/src-tauri/src/cmd_ops.rs` | openmed: 5 |
| ACTIVE_PRODUCT | `apps/desktop-tauri/src-tauri/src/feature_tests.rs` | openmed: 1 |
| ACTIVE_PRODUCT | `apps/desktop-tauri/src/identity.test.ts` | mesc: 1, openmed: 8 |
| ACTIVE_CLI | `crates/medscale-cli/src/main.rs` | mesc: 11, openmed: 4 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-contracts/src/doctor/mod.rs` | mesc: 4 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-contracts/src/envelopes/mod.rs` | mesc: 14 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-contracts/src/fixture_ui/mod.rs` | mesc: 3 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-contracts/src/lib.rs` | mesc: 1 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-contracts/src/mesc/mod.rs` | mesc: 42 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-contracts/src/model_catalog.rs` | openmed: 1 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-contracts/src/packs/mod.rs` | openmed: 10 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-core/src/authority/facade.rs` | mesc: 21, openmed: 12 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-core/src/authority/privacy_recognizers.rs` | openmed: 1 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-core/src/doctor.rs` | mesc: 5 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-core/src/model_acquisition.rs` | openmed: 11 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-core/src/model_catalog.rs` | openmed: 2 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-desktop/src/main.rs` | openmed: 4 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-desktop/src/product_intelligence.rs` | openmed: 43 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-desktop/src/utility_surfaces.rs` | mesc: 2 |
| LEGACY_UI | `crates/medscale-desktop/ui/app.slint` | openmed: 4 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/Cargo.toml` | mesc: 1 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/candle_runtime.rs` | openmed: 6 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/catalog.rs` | openmed: 4 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/compatibility.rs` | openmed: 3549 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/entity_eval.rs` | openmed: 1 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/generation_runtime.rs` | openmed: 1 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/gliner_runtime.rs` | openmed: 14 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/hf_snapshot.rs` | openmed: 32 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/hir_const_matmul.rs` | openmed: 1 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/identifier_validators.rs` | openmed: 7 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/india_clinical.rs` | openmed: 9 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/lib.rs` | mesc: 6 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/mesc_verify.rs` | mesc: 66 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/mlx_layout.rs` | openmed: 9 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/onnx_optional_inputs.rs` | openmed: 1 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/onnx_runtime.rs` | openmed: 11 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/pii_merge.rs` | openmed: 14 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/pii_patterns.rs` | openmed: 3 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/residency.rs` | openmed: 2 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/token_windows.rs` | openmed: 14 |
| FIRST_PARTY_TECHNICAL | `crates/medscale-pack/src/variant_selection.rs` | openmed: 42 |
| OTHER_REVIEW | `docs/brand/FULL_MEDScale_RENAME_EXECUTION.md` | everhealth: 1, medorigin: 1, mesc: 2, openmed: 4 |
| OTHER_REVIEW | `docs/brand/PRE_RELEASE_IDENTITY_AND_PROVENANCE_AUDIT.md` | openmed: 6 |
| OTHER_REVIEW | `docs/engineering/CONTRIBUTING.md` | mesc: 1 |
| OTHER_REVIEW | `docs/product/POST_RELEASE_ROADMAP.md` | openmed: 2 |
| OTHER_REVIEW | `docs/product/RELEASE_CANDIDATE_2026-10-19.md` | openmed: 3 |
| OTHER_REVIEW | `docs/product/RELEASE_CANDIDATE_2026-10-20.md` | openmed: 4 |
| OTHER_REVIEW | `docs/security/TRUST_AND_PRIVACY_MODEL.md` | mesc: 3 |

## Next actions

1. Rebrand active product copy and first-party names wherever semantics permit.
2. For persisted IDs, schemas and APIs: design backwards-compatible migrations and tests.
3. For external model repository identifiers: mirror actual permitted assets and validate resolution before switching functional URLs or claiming a MedScale-owned model.
4. Keep science, license notices, benchmark provenance and historic evidence accurate.
5. Verify exact-head CI, Windows/macOS packages and E2E before merging/releasing.
