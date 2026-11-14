# Feature Specification: Final UI Product Qualification

**Feature Branch**: `spec/067-final-ui-product-qualification`
**Created**: 2026-09-15
**Status**: CLOSED_CANONICAL
**Promotion**: POST_SPEC_066_FINAL_PRODUCT_QUALIFICATION_SEQUENCE

## Goal
Close the repository-owned Desktop+CLI product-launch phase by qualifying the final native product surface that can be proven from repository/CI evidence, and by converting the remaining hardware, signing, and assistive-technology work into exact external action packets without claiming results that are not measured.

## Requirements
- **FR-001**: The final Desktop route inventory MUST have no future-slice placeholder, invented clinical-risk copy, or real-looking patient roster data.
- **FR-002**: Final native controls MUST retain keyboard focus/default-action semantics, accessible names, text status cues, and the Spec 066 engineering contrast floor.
- **FR-003**: Doctor MUST continue to report `final_v0_ui_present=true`, `wcag_conformance_claimed=false`, and accessibility `release_ready=false` until external assistive-technology qualification is actually supplied.
- **FR-004**: Performance methodology MUST stop claiming the final UI is absent. Final interaction response MUST be classified as final-UI-present but externally unmeasured on qualified hardware.
- **FR-005**: Existing launch/idle-memory engineering measurements remain evidence only and MUST NOT become budget-attainment claims.
- **FR-006**: Produce an exact final accessibility action packet covering keyboard-only traversal, focus visibility/order, VoiceOver/NVDA/Orca naming/state announcements, 200% text/reflow inspection, contrast verification, conflict/error/empty-state recovery, and source/provenance drill-down.
- **FR-007**: Produce an exact final performance action packet covering declared release hardware, cold launch, final UI interaction latency, idle RSS, run count, source/tree/lock binding, and p50/p95 evidence.
- **FR-008**: External signing/notarization, macOS signed-product/App Sandbox enforcement, qualified-hardware attainment, real PHI, partner credentials, and terminology rights MUST remain external gates.
- **FR-009**: Spec 012/MESC remains optional/deferred and untouched.
- **FR-010**: Repository-owned implementation completion MUST remain separate from `RELEASE_READY`, `PRIVATE_DATA_READY`, and `MULTI_CLIENT_RELEASE_READY`.

## Success Criteria
- Final static surface/accessibility/honesty regressions pass.
- Runtime/performance evidence correctly distinguishes final-UI presence from unmeasured final interaction latency.
- Exact external accessibility and performance packets are committed.
- Rust 1.88 checks, full required CI, portable package qualification, merge, and post-merge main verification pass.
- After canonical closure, the Desktop+CLI product-launch implementation can truthfully become complete pending external gates; no release/private-data/WCAG claim is inferred.
