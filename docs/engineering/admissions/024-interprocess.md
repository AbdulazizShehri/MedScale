# Dependency admission: interprocess (Spec 024)

| Crate | Version | Features |
|---|---|---|
| interprocess | **2.4.4** (workspace pin) | default |

| Field | Value |
|---|---|
| Owning Spec | 024 |
| Placement | `medscale-core` host IPC server/client only |
| Purpose | Localhost OS IPC (Windows named pipe / Unix domain socket) for CoreFacade envelopes |
| License | MIT OR Apache-2.0; transitive `recvmsg` **0BSD** (allowed in `deny.toml`) |
| Security | Local sockets only; no remote bind; session + WriterLock still enforced by CoreFacade |
| Tests required | Two-client revoke/capability deny; second host WriterHeld; doctor `os_ipc_qualified` honesty |
| Update strategy | Pin minor; re-deny; re-run `host_os_ipc_024` |
| Exit strategy | Replace with platform-native `std`/mio pipe helpers behind `HostIpcServer`/`HostIpcClient` |

Not admitted: remote TCP listeners, privileged service accounts, ambient worker IPC.
