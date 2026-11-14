# Feature Specification: SBOM Lock Binding (Q05 residual)

**Promotion**: EXISTING_Q05_RESIDUAL  
**Does not**: claim RELEASE_READY or full release SBOM.

## Requirements
- FR-001: SBOM scaffold script binds Cargo.lock SHA-256.
- FR-002: Doctor `sbom_lock_bound=true`; release_ready=false.
- FR-003: Missing class `release_sbom_native_model_assets` remains.
