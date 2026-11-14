# Feature Specification: Authenticated Host / Client Authority (Q04)

**Feature Branch**: `spec/018-host-client-authority`  
**Created**: 2026-09-09  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 016 writer lock; Spec 017 privacy honesty; Specs 002/006  

## User Stories

### US1 — Session bound to lease holder (P1)
A local client acquires a vault lease, opens a session with explicit capability grants, and mutating calls require a live non-expired session matching the holder. Revocation and expiry deny further use.

### US2 — No ambient authority for workers (P1)
Doctor and contracts document that workers receive no ambient DB/keys/filesystem/network; READY_BASE does not ship a worker runtime.

## Requirements

- **FR-001**: Typed `ClientSession` with session_id, vault_id, holder_id, granted capabilities, expires_at_seq.
- **FR-002**: `OpenSession` / `RevokeSession` capabilities through CoreFacade.
- **FR-003**: When a session is active for a vault, mutating requests must present matching `session_id` (via request extension field) or fail closed.
- **FR-004**: Expiry and revoke are explicit; lock-file presence is not session proof (Spec 016 writer lock remains separate).
- **FR-005**: Doctor `host_authority` axis; `MULTI_CLIENT_RELEASE_READY = false`.
- **FR-006**: Synthetic-only tests.

## Out of scope

Remote auth; OS IPC transport qualification beyond in-process; full hostile UI sandbox; Q06 semantics.

## Success Criteria

- Session open/revoke/expiry tests PASS
- Existing lease tests remain green
- Evidence LIMITATIONS honest
