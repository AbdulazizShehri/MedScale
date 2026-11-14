# Contract: Authority Facade + Process Topology

**Spec**: 002-trusted-object-source-authority-foundation  
**Status**: Message/API sketch for implement (in-process now; OS IPC later)

## Roles

| Role | May |
|---|---|
| `CoreHost` | Hold single-writer lease; execute authorized mutations; own future DB/key material |
| `Client` (CLI/Desktop/SDK) | Call facade over local IPC; never open canonical store; never receive DB/key handles |
| `TransientHostOwner` | CLI-equivalent that spawns host when none exists, still under single-writer rules |
| `Worker` | Future; receives only capability-scoped handles/data (policy stub in 002) |

## Transport (intent)

- Windows: named pipe  
- Unix: Unix domain socket / local equivalent  
- Versioned frames; request capabilities; explicit deadlines and size limits  
- **No** DB connection handle or key material in any message  
- Spec 002: in-process `CoreFacade` implementing the same logical API

## Envelope

```text
AuthorityRequest {
  schema_version: u32,
  request_id: OpaqueId,
  vault_id: VaultId,
  authority_scope_id: OpaqueId,
  capability: Capability,
  deadline_ms: u64,
  max_response_bytes: u64,
  body: RequestBody,
}

AuthorityResponse {
  schema_version: u32,
  request_id: OpaqueId,
  result: Ok(ResponseBody) | Err(AuthorityError),
}
```

## Capabilities (initial set)

```text
Capability::AcquireLease
Capability::ReleaseLease
Capability::Ping
Capability::CreateSourceRecord          // synthetic/in-memory in 002
Capability::CreateDerivedArtifact
Capability::CreateProposal
Capability::PromoteProposal
Capability::CreateIdentityAssertion
Capability::DecideIdentityMerge
Capability::AppendAudit
Capability::TransitionEffect
Capability::ReadObject                  // by id, scope-checked
```

Unknown capability → `AuthorityError::Unauthorized`.

## Request bodies (sketch)

### AcquireLease

```text
AcquireLease { client_id, holder_id_hint? }
→ Ok { lease: CoreHostLease }
→ Err AlreadyHeld { holder_id } | InvalidVault
```

### ReleaseLease

```text
ReleaseLease { holder_id }
→ Ok | Err NotHolder | Err NotHeld
```

### PromoteProposal

```text
PromoteProposal { proposal_id, authorized_by }
→ Ok { assertion_id, audit_id }
→ Err NotFound | Unauthorized | WrongScope | AlreadyPromoted?
```

### TransitionEffect

```text
TransitionEffect { action_id, to: EffectState, reconcile_token? }
→ Ok { action_id, state }
→ Err IllegalTransition | UnknownRequiresReconcile | Unauthorized
```

### CreateSourceRecord

```text
CreateSourceRecord { media_type, bytes, acquired_at? }
→ Ok { source_id, content_digest }
→ Err LimitExceeded | WrongScope | Unauthorized
```

## Errors (typed)

```text
AuthorityError =
  | Unauthorized
  | WrongScope
  | AlreadyHeld
  | NotHolder
  | NotFound
  | IllegalTransition
  | UnknownRequiresReconcile
  | LimitExceeded
  | DeadlineExceeded
  | SchemaMismatch
  | InvalidClosed
  | Internal
```

## Single-writer rules

1. At most one active `CoreHostLease` per `vault_id`.
2. Mutations require valid holder (or in-process equivalent).
3. Second `AcquireLease` while held → `AlreadyHeld` (fail closed).
4. Drop/expiry of holder releases lease (simulator defines drop semantics).
5. Clients must not open storage engines directly — compile-time/docs enforced; 003+ storage APIs are host-only.

## Mobile note

Mobile uses one app-owned Rust core instance implementing the same facade semantics in-process; background services route through the facade. No separate multi-writer path.

## Explicit non-goals in this contract

- REST/gRPC/GraphQL product plane  
- Transfer of SQLite handles or key bytes  
- Network Broker messages (013)  
- Real worker IPC frames (008) — only policy stubs  
