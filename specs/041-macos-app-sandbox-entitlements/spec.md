# Feature Specification: macOS App Sandbox Entitlements READY_BASE (Q09 residual)

**Feature Branch**: `spec/041-macos-app-sandbox-entitlements`  
**Created**: 2026-09-10  
**Status**: READY (implementation)  
**Depends on**: Spec 031 Seatbelt CLOSED; Spec 040 LPAC CLOSED  
**Promotion**: `EXISTING_Q09_RESIDUAL_ELIGIBLE_FOR_PROMOTION` — see `docs/planning/SPEC_041_PROMOTION.md`  
**Does not**: claim signed enforcement; PLATFORM_QUALIFIED; RELEASE_READY; conflate Seatbelt with App Sandbox.

## Requirements

- **FR-001**: Entitlements fixture with `com.apple.security.app-sandbox=true`.
- **FR-002**: Detection probe `APP_SANDBOX_CONTAINER_ID` (`app_sandbox_container_active`).
- **FR-003**: Doctor `macos_app_sandbox_entitlements_measured=true`; `macos_app_sandbox_enforcement_measured=false`.
- **FR-004**: EXTERNAL_GATES sandbox gate stays OPEN (signed enforcement + composition).
- **FR-005**: Spec Kit + evidence; BUILD_QUEUE 040 CLOSED / 041 then CLOSED; deferred **042+**.
