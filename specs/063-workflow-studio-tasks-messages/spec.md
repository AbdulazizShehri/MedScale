# Feature Specification: Workflow Studio + Tasks/Messages

**Feature Branch**: `spec/063-workflow-studio-tasks-messages`
**Created**: 2026-09-15
**Status**: CLOSED_CANONICAL
**Promotion**: POST_SPEC_062_PRODUCT_SURFACE_SEQUENCE

## Goal
Add native review-first Workflow Studio, Tasks, and Messages surfaces grounded in the existing durable `OutboxEntry`, payload-digest, and `EffectState` authority model without inventing a second task/message/action authority.

## Requirements
- **FR-001**: Preserve native Slint 1.16.1, Rust MSRV 1.88, femtovg 0.23.2, AccessKit, and no WebView/Tauri.
- **FR-002**: Workflow review state MUST derive from existing controlled-action/outbox vocabulary; Desktop MUST NOT own canonical action state.
- **FR-003**: `Unknown` effects MUST visibly require reconciliation and MUST NOT be exposed as retryable by default.
- **FR-004**: Payload identity MUST remain visibly digest-bound before consequential action semantics.
- **FR-005**: Tasks MUST be derived review cues over durable outbox intents, not a new canonical task datastore.
- **FR-006**: Messages MUST be local review/status previews only; no external messaging transport or delivery receipt is claimed.
- **FR-007**: Workflow Studio MAY visualize evidence→review→payload identity→outbox→reconcile sequencing but MUST NOT silently create/commit controlled actions.
- **FR-008**: Existing patient/Insights behavior and three-OS package/release honesty MUST remain intact.
- **FR-009**: Synthetic demo state MUST remain deterministic and visibly labeled.
- **FR-010**: Real PHI, MESC, NPHIES authorization, external messaging, production signing/notarization, WCAG conformance, and release-readiness claims remain out of scope.

## Success Criteria
- Workflows, Tasks, and Messages are real native surfaces rather than placeholders.
- All five effect states are represented and `Unknown` is fail-closed with reconciliation language.
- Derived tasks expose action id, state, safe next step, and payload digest.
- Message previews explicitly avoid transport/delivery authority claims.
- No direct storage/network dependency is introduced into Desktop.
- Local qualification and exact-head required CI/package evidence pass before canonical closure.

## Anti-scope
Audit/exports/settings/integrations (064); CLI parity/polish (065); final hardening/qualification (066-067); new workflow DSL/runtime; new canonical task database; messaging provider integration; blind retry; real PHI.
