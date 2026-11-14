# Spec 018 LIMITATIONS

- `MULTI_CLIENT_RELEASE_READY = FALSE`
- Historical READY_BASE was in-process only; **Spec 024** advances `os_ipc_qualified=true` for bounded localhost IPC (see `evidence/024-host-os-ipc-authority/`)
- Spec 018 opt-in lease-only mutation without `session_id` is superseded on the default path by Spec 024 Strict; engineering escape remains `new_legacy_lease_only_engineering()`
- Lock-file / Spec 016 writer lease is not session proof; sessions are a separate capability grant layer
- Workers still receive no ambient DB/keys/filesystem/network; no worker runtime shipped here
- Remote auth, hostile UI sandbox, and Q06 multi-client semantics are out of scope
