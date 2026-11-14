# Spec 012 / 036 evidence summary

## Live gate (independent re-check historically recorded)
- MESC `main`: `4b193c01f5f94447afd359b0420640647b449a69` / tree `3f48f3a9af89d8e82f04c0501019b09084ff860a`
- `TRAINING_CODE_READY = YES`; `REAL_TRAINING_AUTHORIZED = NO`; `RELEASE_STATUS = BLOCKED`
- TheHalfMoon/MESC `v0.1.0` (release_id `352847712`) assets: **empty**
- `v0.2.0` tag: **no GitHub Release**
- Gate `MESC_RELEASED_ARTIFACT` remains `NOT_AVAILABLE`
- `MEDSCALE_SPEC_012_ADMISSION_READINESS = NOT_READY`

## Spec 012 delivered while gated
- `MescArtifactAdmit` → `ExternalGateRequired(MESC_RELEASED_ARTIFACT)`
- Requires `pack_path_required=true` (ARTIFACT_IMPORT / Pack boundary)
- Doctor `mesc_artifact` axis (`artifact_admitted=false`)
- Architecture refuse MESC Python dependency test

## Spec 036 READY_BASE (in-repo verifier)
- `MescReleaseManifestV0` + `MescArtifactVerify` / `MescVerifyReport`
- Local size/sha256/rights/SBOM/schema fail-closed checks
- Producer+epoch anti-rollback / replay store
- Synthetic fixtures under `fixtures/` (good + adversarial)
- Doctor `verifier_ready_base=true`; `product_admit_authorized` always false on verify
- Tests: `mesc_verifier_036`, pack unit tests

## Not delivered
- Actual Pack admission of MESC bytes
- CLOSED_CANONICAL for Spec 012
- Clearing `MESC_RELEASED_ARTIFACT`
