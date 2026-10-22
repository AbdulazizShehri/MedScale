# MedScale-owned Rust artifact paths — 2026-10-22

## Purpose

Move first-party Rust module and test **path names** away from a retired adjacent project's label. The product continues to be named **MedScale**. This migration changes owned code organization, not the identity, bytes or provenance of an upstream artifact.

## Path migration

| Previous owned path | Current owned path |
| --- | --- |
| `crates/medscale-contracts/src/mesc/mod.rs` | `crates/medscale-contracts/src/external_artifact/mod.rs` |
| `crates/medscale-pack/src/mesc_verify.rs` | `crates/medscale-pack/src/external_artifact_verify.rs` |
| `crates/medscale-core/tests/mesc_admit_012.rs` | `crates/medscale-core/tests/external_artifact_admit_012.rs` |
| `crates/medscale-core/tests/mesc_boundary_012.rs` | `crates/medscale-core/tests/external_artifact_boundary_012.rs` |
| `crates/medscale-core/tests/mesc_optional_decoupling.rs` | `crates/medscale-core/tests/external_artifact_optional_decoupling.rs` |
| `crates/medscale-core/tests/mesc_verifier_036.rs` | `crates/medscale-core/tests/external_artifact_verifier_036.rs` |

The canonical Rust import path for new code is `medscale_contracts::external_artifact`. The old `medscale_contracts::mesc` public re-export is retained, marked `doc(hidden)`, exclusively to preserve source consumers and historic integration test imports while downstream users migrate. Runtime product presentation remains MedScale-only.

The older `Mesc*` public type names, the doctor JSON wire key `mesc_artifact`, legacy schema values and persisted compatibility fields remain stable at this stage. A future renaming of machine-consumed contracts must provide explicit versioned migration and actual replay fixtures rather than replacing existing data silently. Current product doctor output already renders `medscale_research_artifact` as its human-facing heading.

## Verification

- New integration test `external_artifact_import_compatibility` confirms old and new public import paths yield identical serialized doctor-state bytes.
- `cargo fmt --all -- --check` passed locally on macOS after the move.
- `cargo test -p medscale-contracts --locked` passed locally with no failed tests.
- Full repository CI and platform desktop/E2E qualification still gate PR merge. No release published.

## Honest scope boundaries

This change renames only **MedScale-owned** module/test paths. Names identifying another real project, the pinned `openmed` PyPI import, actual Hugging Face repository IDs, scientific comparator labels and recorded historical evidence continue to name what those inputs really are. That does not make them MedScale's first-party brand. Third-party legal notices and provenance records remain intact.

Do not remove `specs/`, `evidence/`, pinned external URLs, historical source receipts or third-party license texts to achieve a superficial zero-match audit. They are separate from the MedScale-owned names in active code.
