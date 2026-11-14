# ADR-024-001: Local OS IPC + strict mutating sessions

## Status

Accepted (Spec 024 READY_BASE)

## Context

Spec 018 shipped in-process sessions with opt-in `session_id`. Q04 residual requires OS IPC qualification and fail-closed mutation without inventing multi-client release.

## Decision

1. Default `SessionEnforcement::Strict` on CoreFacade; mutating capabilities require live session.
2. Serve CoreFacade over length-prefixed JSON on `interprocess` local sockets.
3. IPC host holds Spec 016 WriterLock for the vault root.
4. Doctor may set `os_ipc_qualified=true` for this bounded READY_BASE only; `multi_client_release_ready` stays false.

## Consequences

- Existing in-process tests may use `new_legacy_lease_only_engineering()`.
- CliSession opens a client session after lease.
- No claim of hostile-client release readiness or remote auth.
