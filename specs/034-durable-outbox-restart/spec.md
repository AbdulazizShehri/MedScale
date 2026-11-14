# Feature Specification: Durable Outbox Restart Fixtures (Q12 residual)

**Feature Branch**: `spec/034-durable-outbox-restart`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 014 CLOSED; Spec 016 durable audit-class persistence  
**Does not**: live NPHIES/partner transport; EncryptedVault authority sync; claim RELEASE_READY; clear PARTNER_EHR_NPHIES_ENDPOINT.

## User Stories

### US1 — Outbox survives vault restart (P1)
With SyntheticVault open, create ExternalActionIntent, transition to Unknown, close vault, reopen with a new CoreFacade, and ListOutbox returns the same action_id / effect_state / payload_digest.

### US2 — UNKNOWN still requires reconcile after reload (P1)
After restart, Unknown→Pending without reconcile token fails; with non-empty token succeeds.

### US3 — Doctor honesty (P1)
`ControlledActionsDoctorStatus.outbox_restart_qualified=true`; `nphies_authorized=false`; `unknown_blind_retry=false`.

## Requirements

- **FR-001**: Restart fixture proving audit-class intent durability (no new outbox table).
- **FR-002**: Preserve UNKNOWN reconcile and NPHIES external gate.
- **FR-003**: Doctor axis `outbox_restart_qualified`.
- **FR-004**: Spec Kit + evidence; BUILD_QUEUE 034 CLOSED; deferred **035+**.

## Out of scope

Live adapters; EncryptedVault authority graph sync; claiming live-action reliability; RELEASE_READY.

## Success Criteria

- `outbox_restart_034` green; doctor honesty; workspace gates green
