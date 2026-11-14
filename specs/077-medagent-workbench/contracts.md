# Contracts — Spec 077 MedAgent Workbench

**Status:** implementation contract (initial freeze at T077-00; field-level
freeze record completed at T077-01 against the real Rust source)

## 1. Reused primitives (no new foundation)

- `OpaqueId`, `ObjectHeader`, `DigestSha256`, `RealmId`, `AuthorityScopeId`
  (`medscale-contracts::objects`).
- `ProjectRevision`/`check_revision`/`initial_revision` for optimistic
  concurrency on every mutable 077 row.
- `ArtifactDescriptor`/`ArtifactVersionBinding`/`ReferenceResolution`
  (`medscale-contracts::project_graph`) for `ContextManifest` artifact
  selection.
- `Proposal`/`ProducerKind` (`medscale-contracts::objects::authority_classes`)
  as the terminal output authority for `AgentProposal`.
- `PackManifestV0`/`PackStore` (`medscale-contracts::packs`,
  `medscale-pack::store`) as the admitted-model-Pack source of truth.
- `PackRuntimeAdapter`/`FixtureRuntime`/`OnnxTokenClassifierRuntime`/
  `RuntimeOutput` (`medscale-pack::runtime`, `medscale-pack::onnx_runtime`)
  as the model execution surface.
- `ParticipantKind::Agent`/`AgentParticipantIdentity.agent_profile_ref`
  (`medscale-contracts::collaboration`, Spec 076) as the integration point:
  `AgentIdentity.header.id` is a valid value for `agent_profile_ref` once
  077 lands, resolving Spec 076's previously-opaque forward reference.

## 2. Participant/actor vocabulary

### `AgentIdentity`

```text
AgentIdentity {
  header: ObjectHeader,
  revision: ProjectRevision,
  project_id: OpaqueId,          // scoping Project (074)
  pack_id: OpaqueId,              // bound admitted model Pack (PackManifestV0.pack_id)
  pack_version: String,           // bound pack version at registration time
  display_name: String,           // bounded UTF-8
  status: AgentIdentityStatus,    // Active | Revoked
}
```

`pack_id`/`pack_version` are captured at registration time; if the admitted
Pack is later promoted/superseded, the identity keeps pointing at the exact
version it was registered against (no silent upgrade). A run against an
identity whose captured version no longer matches the currently admitted
Pack fails closed (T5, `security.md`).

**T077-03 implementation decision:** `pack_version` is never accepted as
caller input on `AgentIdentityRegister` -- the request carries only
`pack_id`; Core (`MedAgent::register_agent_identity`) resolves the
currently admitted `PackManifestV0` for that `pack_id` via
`PackStore::get` and captures its real `version` field. A `pack_id` the
local `PackStore` does not recognize (not admitted) fails closed with
`InvalidArgument` before any row is written. This closes an otherwise
open spoofing vector: a caller cannot claim a `pack_version` string that
does not match what is actually admitted.

### `AgentCapabilityManifest`

```text
AgentCapabilityManifest {
  agent_identity_id: OpaqueId,
  granted_tool_kinds: Vec<ToolKind>,   // closed vocabulary, see section 5
}
```

Immutable once set (created together with `AgentIdentity` in one
transaction; no separate update API in this spec -- widening capabilities
requires revoking and re-registering, an explicit, auditable action).

## 3. Context vocabulary

### `ContextManifest`

```text
ContextManifest {
  header: ObjectHeader,
  revision: ProjectRevision,
  project_id: OpaqueId,
  selected_artifacts: Vec<ArtifactDescriptor>,   // exact revision bindings
}
```

A `ContextManifest` is immutable once created (revision exists only to
support the standard CAS-on-archive/tombstone pattern if this spec adds
one; no in-place artifact-list mutation). Building a *new* context for a
follow-up run is a new `ContextManifest`, never an edit of an in-flight
run's bound manifest.

## 4. Run vocabulary

### `AgentRunState`

```text
Pending -> Running -> Cancelled
                    -> Completed
                    -> Failed
```

No other edge exists. `Pending -> Cancelled` directly (cancel before start)
is permitted. Every other transition is `Conflict`/`InvalidArgument`.

### `AgentRun`

```text
AgentRun {
  header: ObjectHeader,
  revision: ProjectRevision,
  project_id: OpaqueId,
  agent_identity_id: OpaqueId,
  context_manifest_id: OpaqueId,
  prompt: String,                 // bounded UTF-8
  status: AgentRunState,
}
```

**T077-01 reconciliation:** `started_at_seq`/`ended_at_seq` from the
pre-implementation sketch were dropped during the real T077-01 freeze
(`crates/medscale-contracts/src/medagent.rs`) -- no acceptance requirement
in `security.md` or `migration.md` names them, and run timing/ordering is
already fully recoverable without them: `AgentTurn.seq` gives the exact
in-run step order, and `RunReceipt` (section 6) is the durable marker of
"this run reached a terminal state," committed atomically with the
transition. Reintroducing them would be pure restatement of information
the append-only `AgentTurn` stream and `RunReceipt` already carry. If a
later task needs "seq at which Running began" specifically (as opposed to
"the first `ToolRequested`/`ModelOutput` turn"), add it then as an
additive field, not preemptively here.

### `AgentTurn`

```text
AgentTurn {
  header: ObjectHeader,
  run_id: OpaqueId,
  seq: u64,                       // append-only, storage-assigned
  kind: AgentTurnKind,            // PromptSubmitted | ToolRequested | ToolResult | ModelOutput
  payload: Value,                 // bounded, kind-shaped
}
```

## 5. Tool vocabulary

### `ToolKind` (closed vocabulary, frozen at T077-01)

Minimum viable set for this spec's vertical slice (no browser/network/
external-provider tool is ever a member):

```text
ReadContextArtifact   // read one artifact named in the run's ContextManifest
SearchContextArtifacts // bounded lexical search over the run's ContextManifest artifacts only
```

Additional in-context-only tool kinds may be added during implementation
if a genuine need appears, but the vocabulary stays closed and every
member must be provably boundable to the run's `ContextManifest` -- no
member may name an unscoped resource.

### `ToolManifest`

```text
ToolManifest {
  kind: ToolKind,
  // fixed, versioned JSON-schema-shaped argument/result contract per kind;
  // enforced via serde(deny_unknown_fields) typed structs, not a raw Value
  // passthrough.
}
```

### `ToolInvocation`

```text
ToolInvocation {
  header: ObjectHeader,
  run_id: OpaqueId,
  turn_seq: u64,                  // the AgentTurn that requested this
  seq: u64,                       // append-only, storage-assigned
  kind: ToolKind,
  arguments: Value,               // typed per ToolKind, validated before dispatch
  status: ToolInvocationStatus,   // Refused | Executed
}
```

### `ToolReceipt`

```text
ToolReceipt {
  header: ObjectHeader,
  invocation_id: OpaqueId,        // 1:1 with ToolInvocation once Executed
  result: Value,                  // typed per ToolKind
}
```

A `Refused` invocation never has a `ToolReceipt` (the refusal reason lives
on `ToolInvocation` itself); an `Executed` invocation always has exactly
one.

**T077-01 reconciliation:** the pre-implementation sketch's
`resolution: Option<ReferenceResolution>` was dropped during the real
T077-01 freeze -- no acceptance requirement in `security.md` or
`migration.md` names it, and section 1's reuse list only requires
`ReferenceResolution` to be available where semantically valid, not that
every 077 type carry one. `ReadContextArtifact`'s `result: Value` is free
to embed whatever revision-binding evidence T077-06's real dispatch
implementation needs (e.g. a `ReferenceResolution`-shaped value inside the
typed result payload for that `ToolKind`) without a dedicated top-level
field forcing every other `ToolKind`'s receipt to carry an always-`None`
column. T077-06 must still decide, and record here, exactly what
`ReadContextArtifact`'s result payload contains.

## 6. Receipt and proposal vocabulary

### `RunReceipt`

```text
RunReceipt {
  header: ObjectHeader,
  run_id: OpaqueId,               // 1:1 with AgentRun's terminal state
  pack_id: OpaqueId,
  pack_version: String,
  context_manifest_id: OpaqueId,
  context_manifest_revision: ProjectRevision,
  tool_invocation_ids: Vec<OpaqueId>,   // in seq order
  final_state: AgentRunState,     // Cancelled | Completed | Failed
  failure_reason: Option<String>, // set only when final_state == Failed
}
```

Committed atomically with the run's terminal transition
(`migration.md` section 5); never exists for a non-terminal run; a terminal
run never lacks one.

### `AgentProposal`

```text
AgentProposal {
  header: ObjectHeader,
  run_id: OpaqueId,
  proposal_id: OpaqueId,          // the real Proposal object this submits
}
```

`AgentProposal` is a thin, 077-owned linking record; the actual claim
content lives in the reused `Proposal` object
(`payload`/`claim_kind`/`confidence`/`evidence_refs`/`producer`), submitted
through the existing `CreateProposal` capability with
`producer: ProducerKind::Agent(agent_identity_id)` (see section 7 -- this
additive `ProducerKind` variant is the one closed-Spec-002-file touch this
spec makes, following the same additive-extension discipline Spec 076 used
for its own touches to already-closed files).

## 7. `ProducerKind` extension (additive, Spec 002 file)

`objects::authority_classes::ProducerKind` currently has
`Human | Rule | WorkerStub | Other(String)`. This spec adds one variant:

```text
Agent(OpaqueId)   // the AgentIdentity that produced this Proposal
```

This is the only change to an already-closed spec's contract file this
promotion authorizes, and it is purely additive (a new enum variant on an
already-`#[non_exhaustive]`-equivalent-in-practice producer taxonomy, not a
removal or semantic change to any existing variant). If `ProducerKind` is
not already forward-compatible with a new variant (e.g., if downstream
`match` statements are exhaustive without a wildcard arm), every such match
site must be updated to handle `Agent` explicitly, never papered over with
a wildcard that silently treats an agent-produced proposal the same as a
human-produced one where that distinction matters.

## 8. Freeze record (T077-01, filled in against real Rust source)

```text
CONTRACTS_MODULE = crates/medscale-contracts/src/medagent.rs (799 lines)
SCHEMA_VERSION_CONST = MEDAGENT_SCHEMA_VERSION: u32 = 1
BOUND_CONSTANTS = AGENT_DISPLAY_NAME_MAX_CHARS = 128, PROMPT_MAX_BYTES =
  32_768, TOOL_ARGUMENT_MAX_BYTES = 8_192, TOOL_RESULT_MAX_BYTES = 32_768,
  FAILURE_REASON_MAX_CHARS = 1_024, CONTEXT_MANIFEST_MAX_ARTIFACTS = 64,
  CAPABILITY_MANIFEST_MAX_TOOL_KINDS = 32
ENUM_VOCABULARIES = AgentIdentityStatus {Active, Revoked}, AgentRunState
  {Pending, Running, Cancelled, Completed, Failed}, AgentTurnKind
  {PromptSubmitted, ToolRequested, ToolResult, ModelOutput}, ToolKind
  {ReadContextArtifact, SearchContextArtifacts}, ToolInvocationStatus
  {Refused, Executed}. Every enum: const as_str() + parse(&str) closed-
  vocabulary round trip (contracts.md convention, mirrors collaboration.rs).
VALIDATION_HELPERS = bounded_text/bounded_bytes/bounded_optional_text,
  deliberately re-implemented in this module (not imported from
  collaboration.rs) rather than factored into a shared helper module --
  this spec's own convention, matching how storage/medagent.rs deliberately
  duplicates its small helpers from collaboration.rs "so this module never
  depends on Spec 076's closed file" (see that file's own header comment).
FIELD_DRIFT_FROM_PRE_IMPLEMENTATION_SKETCH = AgentRun.started_at_seq/
  ended_at_seq and ToolReceipt.resolution (sections 4 and 6 above) were
  dropped during the real freeze; reconciled in-place above with rationale
  (T077-01 reconciliation notes), found and fixed during this session's
  T077-00-style re-verification rather than left silently stale.
TEST_COUNT = 10 (#[test] functions in crates/medscale-contracts/src/medagent.rs:
  agent_identity_status_round_trips, tool_kind_round_trips_and_rejects_unknown,
  agent_run_state_transition_table_is_frozen,
  agent_run_state_terminal_classification,
  agent_identity_new_validates_display_name,
  agent_capability_manifest_rejects_empty_grants,
  context_manifest_rejects_empty_and_checks_membership,
  agent_run_new_rejects_empty_and_oversized_prompt,
  tool_invocation_validate_enforces_refusal_reason_pairing,
  run_receipt_validate_requires_terminal_state_and_failure_reason_pairing)
IMPORT_PURITY = only serde/serde_json + crate::objects::{ObjectHeader,
  OpaqueId} + crate::project_graph::{ArtifactDescriptor, ProjectRevision,
  check_revision, initial_revision} -- no storage/network/UI/model-runtime
  dependency (verified by inspection of the `use` block, this session).
```
