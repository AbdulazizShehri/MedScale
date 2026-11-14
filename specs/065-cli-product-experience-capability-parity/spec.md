# Feature Specification: CLI Product Experience + Capability Parity

**Feature Branch**: `spec/065-cli-product-experience-capability-parity`
**Created**: 2026-09-15
**Status**: CLOSED_CANONICAL
**Promotion**: POST_SPEC_064_PRODUCT_SURFACE_SEQUENCE

## Goal
Make `medscale` a discoverable, structured, launch-quality first-class client of the same Rust authority path as Desktop, without introducing privileged CLI shortcuts or inventing parity where a capability is intentionally visual-only.

## Requirements
- **FR-001**: Preserve facade-only authority; CLI MUST NOT depend directly on storage, SQL, keys, or network clients.
- **FR-002**: Existing command paths remain compatible while new grouped discovery paths are added.
- **FR-003**: Add a compact status surface and an explicit capability map with text and JSON output.
- **FR-004**: Add patient snapshot/read paths over timeline, Brief, and coverage without inventing clinical interpretation.
- **FR-005**: Add read-only controlled-action outbox and disclosure-list paths using existing `ListOutbox` / `ListDisclosures` authority contracts.
- **FR-006**: Add FHIR support-matrix inspection and preserve full-conformance/release non-claims.
- **FR-007**: Population Insights / Workflow Studio visual composition may be represented as Desktop-only presentation in the parity map; CLI MUST NOT create duplicate authority just to mimic UI.
- **FR-008**: Structured JSON paths MUST be deterministic and secret-marker safe.
- **FR-009**: Real PHI stdout, production credentials, direct partner egress, MESC work, and release/WCAG claims remain out of scope.

## Success Criteria
- Top-level help makes status, patient, actions, audit, FHIR, capabilities, host IPC, vault, packs, and journey paths discoverable.
- Read surfaces offer `--json` and use typed existing contracts.
- `CliSession` exposes outbox/disclosure/FHIR reads only through `CoreFacade` authority requests.
- A product guide documents common commands, output behavior, transient-host limits, and privacy boundaries.
- Tests bind parity/discovery and prove no direct storage/network dependency.
