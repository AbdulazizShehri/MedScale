# Spec 024 LIMITATIONS

- `MULTI_CLIENT_RELEASE_READY = FALSE` — localhost IPC READY_BASE only; not multi-client release qualification
- `RELEASE_READY = FALSE`
- `PRIVATE_DATA_READY = FALSE`
- Bounded localhost OS IPC (named pipe / UDS); not remote auth, not hostile-peer sandboxing
- `LegacyLeaseOnlyEngineering` remains available for in-process engineering tests; qualified path (default + IPC host) is Strict
- Workers still receive no ambient DB/keys/filesystem/network; no worker runtime shipped here
- Lock-file / Spec 016 writer lease is process exclusivity for the IPC host, not multi-client session proof by itself
- Q06 multi-client semantics and hostile UI sandbox remain out of scope
