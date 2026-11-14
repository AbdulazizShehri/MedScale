# ADR-021-001: Composed minimum lovable workflow + disclosure append

**Status**: Accepted  
**Context**: Trusted V1 Q07 requires a restartable import-review-timeline-export-backup journey with disclosure clarity, without inventing a second authority architecture.

**Decision**:

1. Implement the journey as a CoreFacade-composed runner + CLI `journey run` over existing vault/ingest/promote/presentation/export/backup/restore/close capabilities.
2. Add typed `DisclosureRecord` with local append/list facade ops persisted as durable audit detail.
3. Preview via Proposal + loss-aware export; Accept via PromoteProposal; Reject via non-promote audit.
4. Surface `WorkflowDoctorStatus` with `workflow_ready_base=true` and `release_ready=false`.
5. Qualify with CoreFacade integration tests including Spec 016-style two-process reopen.

**Consequences**:

- Operators get a documented synthetic end-to-end path with stable JSON errors.
- Completing the journey never implies RELEASE_READY or PRIVATE_DATA_READY.
- Final visual Desktop UX remains a later/external integration concern.
