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

The canonical source types are now `ExternalArtifact*` (and `ExternalReleaseManifestV0`), with `ExternalArtifactEpochStore`, `ExternalArtifactVerifyError` and `verify_external_artifact_release_dir` in MedScale Pack. The former `Mesc*` public symbols and `verify_mesc_release_dir` are retained only as hidden Rust aliases so older consumers compile; the doctor JSON wire key `mesc_artifact`, legacy schema values, and persisted compatibility fields remain byte-stable. A future renaming of machine-consumed contracts must provide explicit versioned migration and actual replay fixtures rather than replacing existing data silently. Current product doctor output already renders `medscale_research_artifact` as its human-facing heading.

## Verification

- New integration test `external_artifact_import_compatibility` confirms canonical `ExternalArtifactDoctorStatus` and the old Rust import yield identical serialized doctor-state bytes.
- `cargo fmt --all -- --check` passed locally on macOS after the move.
- `cargo test -p medscale-contracts --locked` passed locally with no failed tests.
- `cargo check -p medscale-pack --locked` passed locally on macOS after the owned type/path migration.
- Full repository CI and platform desktop/E2E qualification still gate PR merge. No release published.

## Honest scope boundaries

This change renames only **MedScale-owned** module/test paths. Names identifying another real project, the pinned `openmed` PyPI import, actual Hugging Face repository IDs, scientific comparator labels and recorded historical evidence continue to name what those inputs really are. That does not make them MedScale's first-party brand. Third-party legal notices and provenance records remain intact.

Do not remove `specs/`, `evidence/`, pinned external URLs, historical source receipts or third-party license texts to achieve a superficial zero-match audit. They are separate from the MedScale-owned names in active code.


## Product-visible and first-party qualification naming follow-on

The subsequent scoped pass also moved first-party model-evaluation entry points to neutral MedScale-owned paths:

- `scripts/evaluation/reference_predict.py` for the pinned external reference runner.
- `crates/medscale-pack/tests/reference_python_external_103.rs` and `reference_snapshot_external_103.rs`.
- Corresponding workflow, batch-selector, evidence-command and environment-variable names now refer to a `reference` rather than branding MedScale's own commands after the comparator.
- Human-readable CLI catalog help hides the source repository's implicit default; internal model acquisition continues to use the real pinned source identifier.
- The legacy Slint comparison card now uses `reference` bindings and anonymous pinned-baseline labels. The modern Tauri product UI continues to use the MedScale catalog presentation.
- Qualification-model display labels are generic while their real identity, revision and source integrity remain recorded in machine-readable evidence.

The model reference script still imports the actual installed benchmark dependency. Legal notices, historical artifacts, source catalog JSONL and pinned repository IDs are **not changed** by these UI/owned-name edits. Replacing those dependencies requires working admitted MedScale-owned mirrors or native implementations, followed by full tests and provenance preservation.

Local verification: `cargo test -p medscale-pack --test reference_python_external_103 --test reference_snapshot_external_103 --locked --no-run` PASS; model CLI `packs catalog --help` visible-label scan PASS; `cargo check -p medscale-desktop --locked` PASS on macOS; workflow YAML parses; Rust formatting and Python syntax PASS. Remote CI remains the merge gate.
