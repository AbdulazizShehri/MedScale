# Feature Specification: MESC Artifact Integration

**Branch**: `spec/012-mesc-deny-path-gate-refresh`  
**Status**: `DEFERRED_BY_CANONICAL_DESIGN` (optional external integration); fail-closed contracts QUALIFIED; admit lane stays gated on a real upstream release  
**Input**: Admit immutable MESC artifacts via Pack with exact hashes/rights/SBOM/evaluation. Never copy MESC Python/runtime. Never mutate TheHalfMoon/MESC.

## User Stories

### US1 Fail-closed admit while gate open (P1)
`MescArtifactAdmit` requires `pack_path_required` and returns `ExternalGateRequired(MESC_RELEASED_ARTIFACT)` until a qualifying release exists.

### US2 Refuse Python import (P1)
Architecture tests refuse linking/importing MESC Python package, shared DB, or shared keys.

### US3 Doctor MESC axis (P1)
Doctor reports `mesc_artifact` present, `artifact_admitted=false`, gate name.

### US4 Admit released artifact (P1 — GATED)
When `MESC_RELEASED_ARTIFACT` clears with hashes/rights/SBOM/evaluation, admit through Pack only.

## Anti-scope
Python import; shared DB/keys; ambient MESC service authority; inventing release assets; clearing the gate without live evidence.
