# Spec 014 evidence summary

## Delivered (READY_BASE)
- Durable `ExternalActionIntent` with required payload digest
- Outbox list capability
- Effect SM: PENDING→SENT→CONFIRMED|FAILED|UNKNOWN; UNKNOWN requires reconcile
- NPHIES invoke refused with `SPEC_014_WORKFLOW_EVIDENCE` / partner gates
- Doctor `controlled_actions` axis

## Limitations
- No live NPHIES/EHR workflows
- No production external actions
- Terminology/profile gates for selected NPHIES workflows remain OPEN
