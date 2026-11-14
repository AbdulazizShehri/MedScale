# Feature Specification: Controlled Actions / NPHIES (READY_BASE)

**Branch**: `spec/014-controlled-actions-nphies`  
**Status**: QUALIFIED for READY_BASE (013 CLOSED; NPHIES workflows remain EXTERNAL_GATE)  
**Input**: Durable external-action intents + outbox; effect state machine `PENDING → SENT → CONFIRMED|FAILED|UNKNOWN`; approval binds exact payload digest; UNKNOWN never blindly retried. NPHIES partner workflows deferred until `SPEC_014_WORKFLOW_EVIDENCE` + partner gates.

## User Stories

### US1 Create durable external-action intent (P1)
Actor creates an `ExternalActionIntent` with required `payload_digest`. Intent starts `Pending` and appears in the outbox.

### US2 Transition with reconcile discipline (P1)
Legal transitions only. `Unknown → *` requires non-empty reconcile token. `Pending → Sent` requires payload digest present (approval bound).

### US3 NPHIES refuse without workflow evidence (P1)
Any NPHIES invoke returns `ExternalGateRequired` — no production external action, no fabricated profiles.

### US4 Doctor controlled-actions axis (P1)
Doctor reports controlled_actions present, nphies_authorized=false, unknown_blind_retry=false.

## Anti-scope
Live NPHIES/EHR; production credentials; REAL_PHI; inventing workflow profiles/terminology; MCP skill authority.
