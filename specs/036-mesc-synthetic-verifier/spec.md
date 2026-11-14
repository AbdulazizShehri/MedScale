# Feature Specification: MESC Synthetic Verifier READY_BASE

**Feature Branch**: `spec/036-mesc-synthetic-verifier`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 012 gate-blocked admit path; Pack ARTIFACT_IMPORT boundary  
**Does not**: clear `MESC_RELEASED_ARTIFACT`; admit real MESC bytes; mutate TheHalfMoon/MESC; execute model weights; claim product Pack enablement.

## Requirements

- **FR-001**: Versioned `MescReleaseManifestV0` with producer/release/tag/commit/tree, model/tokenizer/base/corpus ids, training/evaluation receipt digests, SBOM + rights paths/digests, limitations, runtime requirements, epoch, and artifact enumeration (path/size/sha256).
- **FR-002**: Fail-closed local directory verifier: missing/malformed/unsupported schema, duplicate paths, size/digest mismatch, missing rights/SBOM/files → stable `MescVerifyReason`.
- **FR-003**: Successful synthetic verify yields `MescAdmissionState::Verified` with `product_admit_authorized=false`.
- **FR-004**: In-process anti-rollback / replay store by producer+epoch.
- **FR-005**: `MescArtifactAdmit` remains `ExternalGateRequired(MESC_RELEASED_ARTIFACT)`.
- **FR-006**: Doctor `mesc_artifact.verifier_ready_base=true` while `artifact_admitted=false`.
- **FR-007**: Synthetic fixtures under `evidence/012-mesc-artifact-integration/fixtures/` (good + adversarial). Never fake a real MESC release.
