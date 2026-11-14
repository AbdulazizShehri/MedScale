# Spec 036 evidence summary

## Claims
- Synthetic MESC release-dir verifier READY_BASE present.
- Doctor `mesc_artifact.verifier_ready_base=true`.
- `MescArtifactAdmit` still `ExternalGateRequired(MESC_RELEASED_ARTIFACT)`.

## Non-claims
- Not `PLATFORM_QUALIFIED` for MESC runtime.
- Not product Pack enablement.
- Not clearance of EXTERNAL_GATES `MESC_RELEASED_ARTIFACT`.

## Evidence pointers
- `crates/medscale-pack/src/mesc_verify.rs`
- `crates/medscale-core/tests/mesc_verifier_036.rs`
- `evidence/012-mesc-artifact-integration/fixtures/`
- `docs/planning/MESC_ARTIFACT_ACCEPTANCE.md`
