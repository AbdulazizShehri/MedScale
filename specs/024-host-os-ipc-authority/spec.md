# Feature Specification: Host OS IPC Authority (Q04 residual)

**Feature Branch**: `spec/024-host-os-ipc-authority`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 018 `CLOSED_CANONICAL` (in-process sessions); Spec 016 writer lock  
**Does not**: claim `MULTI_CLIENT_RELEASE_READY`, `RELEASE_READY`, or `PRIVATE_DATA_READY`; remote auth; hostile WebView sandbox; worker ambient authority; invent plugins/GraphRAG.

## User Stories

### US1 — Strict session for mutating authority (P1)
A local client mutating through CoreFacade (qualified path) must present a live `session_id` with the required capability. Legacy lease-only mutation without `session_id` is disabled on the default/qualified path.

### US2 — Localhost OS IPC transport (P1)
A host serves CoreFacade request/response over localhost OS IPC (Windows named pipe / Unix domain socket). Session open/revoke and a mutating call succeed via IPC.

### US3 — Cross-client revoke/deny evidence (P1)
Two IPC clients prove that revoke / capability deny is enforced across the IPC boundary.

### US4 — Writer lock bound to IPC host (P1)
The IPC host holds Spec 016 `WriterLock` for the vault root; a second host cannot take over while the first holds.

### US5 — Honest doctor posture (P1)
Doctor `host_authority.os_ipc_qualified=true` only for this bounded localhost IPC READY_BASE; `multi_client_release_ready=false` always in this unit.

## Requirements

- **FR-001**: Default (fail-closed) session enforcement: mutating capabilities require valid `session_id` + granted capability. Clearly named engineering/test escape only if needed (`LegacyLeaseOnlyEngineering`).
- **FR-002**: Length-prefixed JSON `AuthorityRequest`/`AuthorityResponse` over portable local OS IPC (`interprocess` local sockets → named pipe on Windows, UDS on Unix).
- **FR-003**: `HostIpcServer` / `HostIpcClient` library API; optional CLI host-ipc subcommand.
- **FR-004**: IPC host acquires Spec 016 `WriterLock` on vault root at bind; second bind fails closed.
- **FR-005**: Integration evidence: session open + mutate + revoke/deny across two IPC clients; writer-lock exclusivity.
- **FR-006**: Doctor: `os_ipc_qualified=true`, `multi_client_release_ready=false`, `ready_base=true`.
- **FR-007**: Update BUILD_QUEUE / roadmap / START_HERE: Spec 024 CLOSED_CANONICAL READY_BASE; deferred advanced **025+**.
- **FR-008**: Synthetic-only tests; no remote auth; no worker ambient authority.

## Out of scope

Multi-client release readiness; remote authentication; hostile UI/WebView sandbox claims; OS keyring/swap/snapshot privacy; plugins/GraphRAG/replicas/imaging; MESC mutation; real PHI.

## Success Criteria

- Focused IPC/session tests PASS; workspace `--locked` green on Windows (+ Linux CI)
- Doctor honesty flags as above
- Evidence LIMITATIONS honest; no MULTI_CLIENT / RELEASE / PRIVATE_DATA claims
