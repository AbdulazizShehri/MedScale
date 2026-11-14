# Contracts — Spec 084 MedScale Hub Foundation

`crates/medscale-contracts/src/hub.rs` (schema version 1, protocol 1):

| Contract | Meaning |
|---|---|
| `HubIdentity` | the Hub role of one vault; its header's realm and scope are the tenant scope of every Hub row |
| `HubInvitation`, `HubInvitationCode` | one-time invitation to one Project (token digest stored; the code is shown once) |
| `DeviceIdentity` | enrolled device: public key, Hub participant and holder, the one Project it may sync |
| `HubChallenge`, `HubHandshake`, `HubSession` | versioned single-use nonce, signed answer, device session |
| `SyncIntent`, `SyncEnvelopeBody`, `SyncEnvelope` | one signed collaboration intent (`message_post`, `task_create`, `task_update`, `note_create`, `note_edit`) with a device-scoped sequence |
| `SyncOutcome`, `SyncConflict`, `SyncRefusal` | `applied`, `conflict_copy`, `conflict`, `refused` with a closed reason |
| `HubEvent`, `HubEventKind`, `HubAuditCheckpoint`, `HubEventPage` | per-Project hash-chained log: `device_enrolled` (binds the public key), `device_revoked`, `submission` |
| `HubStatus` | operator view with verified chain heads |
| `HubLink`, `HubOutboxEntry`, `OutboxState`, `SyncReport` | client side |

Signed payloads are domain separated (`medscale-hub-enroll-v1`,
`medscale-hub-handshake-v1`, `medscale-hub-envelope-v1`). Hex is strict
lowercase of exact length. Unknown intent kinds fail to parse.

Capabilities (`envelopes/mod.rs`): `HubAdmin` (init, invite, revoke),
`HubRead` (status, links, outbox, mirror), `HubBootstrap` (enroll,
challenge, handshake; no session), `HubSync` (submit, pull; granted only
to device sessions), `HubClient` (join, queue, sign, record, mirror).
