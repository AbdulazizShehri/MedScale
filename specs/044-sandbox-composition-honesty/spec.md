# Feature Specification: Multi-OS Sandbox Composition Honesty (Q09 residual)

**Feature Branch**: `spec/044-sandbox-composition-honesty`  
**Status**: READY  
**Promotion**: `EXISTING_Q09_RESIDUAL_ELIGIBLE_FOR_PROMOTION` — see `SPEC_044_PROMOTION.md`  
**Does not**: claim PLATFORM_QUALIFIED; clear WORKER_OS_SANDBOX_PLATFORM_QUALIFIED; measure signed App Sandbox enforcement.

## Requirements

- **FR-001**: Ship `OsSandboxCompositionInventory` listing ReadyBaseMeasured axes.
- **FR-002**: Doctor exposes `composition_inventory_present` + `composition_residuals_open`.
- **FR-003**: Residuals include signed App Sandbox enforcement + multi-OS PLATFORM_QUALIFIED composition.
- **FR-004**: `platform_qualified=false`; EXTERNAL_GATES sandbox gate stays OPEN.
- **FR-005**: Evidence + tests; advanced deferred **045+**.
