# Evidence SUMMARY — Spec 024 Host OS IPC Authority

**Status:** CLOSED_CANONICAL READY_BASE  
**Branch:** `spec/024-host-os-ipc-authority`  
**Design:** Strict mutating sessions + length-prefixed JSON over `interprocess` local sockets (Windows named pipe / Unix UDS); IPC host holds Spec 016 WriterLock.

## Delivered

- `SessionEnforcement::Strict` default on `CoreFacade`; mutating ops require live `session_id`
- Engineering escape: `CoreFacade::new_legacy_lease_only_engineering()` (doctor-visible name)
- `HostIpcServer` / `HostIpcClient` library + `medscale host-ipc serve|call` CLI
- Two-client IPC revoke / capability-deny evidence
- Second IPC host fails on WriterHeld while first holds lock
- Doctor: `os_ipc_qualified=true`, `multi_client_release_ready=false`
- CliSession + Spec 021 journey open client sessions after lease

## Honesty

- MULTI_CLIENT_RELEASE_READY = FALSE
- RELEASE_READY = FALSE
- PRIVATE_DATA_READY = FALSE
- REAL_PHI unauthorized
- No remote auth; no hostile WebView sandbox claim; no worker ambient authority
