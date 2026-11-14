# Spec 018 evidence summary

**Status**: READY_BASE in-process host/client sessions shipped; `MULTI_CLIENT_RELEASE_READY = FALSE`  
**ADR**: ADR-018-001 in-process SessionRegistry before OS IPC

## Delivered

- Envelope ops `OpenSession` / `RevokeSession` + optional `AuthorityRequest.session_id`
- `SessionRegistry` open / revoke / validate / monotonic tick
- CoreFacade lease-holder binding for OpenSession
- Doctor `host_authority` axis (ready_base=true; multi-client/OS IPC false)
- Tests `host_session_018` (open/revoke/expiry deny; legacy no-session path)

## Not claimed

See LIMITATIONS.md.
