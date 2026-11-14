# Contracts — Spec 078 Model Fleet + Compare

**Status:** implementation contract (initial freeze at T078-00; field-level
freeze record to be completed at T078-01 against the real Rust source,
mirroring Spec 077's `contracts.md` section 8 discipline)

## 1. Reused primitives (no new foundation)

- `OpaqueId`, `ObjectHeader`, `DigestSha256`, `RealmId`, `AuthorityScopeId`
  (`medscale-contracts::objects`).
- `ProjectRevision`/`check_revision`/`initial_revision` for optimistic
  concurrency on every mutable 078 row.
- Spec 077's `AgentIdentity`, `AgentIdentityStatus`,
  `AgentCapabilityManifest`, `ContextManifest`, `AgentRun`,
  `AgentRunState`, `AgentTurn`, `AgentTurnKind`, `ToolKind`,
  `ToolInvocation`, `ToolInvocationStatus`, `ToolReceipt`, `RunReceipt`,
  `AgentProposal` (`medscale-contracts::medagent`) — reused exactly as
  frozen, zero modification.
- `Proposal`/`ProducerKind` (`medscale-contracts::objects::authority_classes`)
  is reached only indirectly, through Spec 077's existing
  `AgentProposal`/`CreateProposal` path inside each lane's own `AgentRun`;
  078 adds no new `ProducerKind` variant and no direct `CreateProposal`
  call of its own.
- `PackManifestV0`/`PackStore` (`medscale-contracts::packs`,
  `medscale-pack::store`) as the admitted-model-Pack source of truth,
  reached only through Spec 077's existing `AgentIdentity` registration
  path (078 never calls `PackStore` directly).
- Spec 077's Core module (`medscale-core::authority::medagent::MedAgent`)
  exposes the run-execution surface 078 drives; in particular
  `execute_agent_run`, `create_agent_run`, `start_agent_run`,
  `cancel_agent_run` are called by 078's Core module exactly as any other
  caller would call them, never re-implemented.

## 2. Lane vocabulary

### `AgentLane`

```text
AgentLane {
  header: ObjectHeader,
  revision: ProjectRevision,
  project_id: OpaqueId,           // scoping Project (074)
  agent_identity_id: OpaqueId,    // Spec 077 AgentIdentity this lane wraps
  context_manifest_id: OpaqueId,  // Spec 077 ContextManifest this lane's runs bind
  role_label: String,             // bounded UTF-8, e.g. "baseline", "strict-evidence"
  policy: LanePolicy,
  status: AgentLaneStatus,        // Active | Retired
}
```

An `AgentLane` never widens what its bound `AgentIdentity`/
`ContextManifest` already allow; `policy` (below) may only *narrow* the
effective grant a lane's runs use, never grant anything the underlying
identity's `AgentCapabilityManifest` or the bound `ContextManifest` does
not already permit. This is enforced at lane-creation time (a policy
naming a tool kind the identity's capability manifest does not grant, or an
artifact the context manifest does not name, is refused, `InvalidArgument`)
and re-checked at every lane-run dispatch (never cached).

### `LanePolicy`

```text
LanePolicy {
  granted_tool_kinds: Option<Vec<ToolKind>>,   // None = inherit identity's
                                                // full capability manifest;
                                                // Some(set) = narrow subset,
                                                // must be a subset of the
                                                // identity's granted set
  context_artifact_ids: Option<Vec<OpaqueId>>, // None = inherit the full
                                                // bound ContextManifest;
                                                // Some(set) = narrow subset,
                                                // must be a subset of the
                                                // manifest's own selected
                                                // artifacts
}
```

`LanePolicy` is immutable once set (created together with `AgentLane` in
one transaction, matching Spec 077's `AgentCapabilityManifest` precedent;
widening a lane's policy requires retiring and re-creating the lane, an
explicit, auditable action).

### `LaneTransform`

```text
LaneTransform {
  // Reserved, closed-empty vocabulary at 078: no transform kind is defined
  // yet. A LaneTransform is named in the roadmap's contract list but this
  // spec's authorized scope has no concrete transform to apply (078 does
  // not admit new Packs, does not build a privacy transform pipeline --
  // that is Spec 079's authority). Declared here as a typed, closed-empty
  // enum (not a Value passthrough) so that a later spec extends it
  // additively rather than this spec inventing an untyped placeholder.
}
```

`LaneTransform` is frozen as a closed enum with **zero variants** at T078-01
unless T078-01 discovers a genuine, minimal, in-scope transform need (e.g.
a bounded prompt-prefix per role label) while implementing `LanePolicy`
above — if so, that variant is added and recorded here with rationale,
never spun up as unscoped general infrastructure.

### `AgentLaneStatus`

```text
Active | Retired
```

Mirrors `AgentIdentityStatus`'s `Active | Revoked` shape and semantics
(closed vocabulary, `as_str()`/`parse()` round trip); `Retired` is the lane
equivalent of "revoked" — a retired lane may not be bound to any new
`FleetRun`, but its past `LaneRunRef`s and any `ComparisonReport`s that
already reference it remain fully inspectable.

## 3. Fleet vocabulary

### `FleetRunState`

```text
Pending -> Running -> Completed
                    -> PartiallyFailed
                    -> Failed
                    -> Cancelled
```

`Pending`: created, no lane run dispatched yet. `Running`: at least one
lane run is `Pending`/`Running` in Spec 077 terms. `Completed`: every lane
reached `AgentRunState::Completed`. `PartiallyFailed`: at least one lane
reached `Completed` and at least one other lane reached
`Cancelled`/`Failed` — the fleet has *some* usable comparison material but
not from every lane. `Failed`: every lane reached `Cancelled`/`Failed`
(zero usable lane output). `Cancelled`: the fleet itself was cancelled
before any lane reached a terminal state (this cancels every still-running
lane through Spec 077's own `cancel_agent_run`, it does not invent a
second cancellation mechanism).

No other edge exists. `Pending -> Cancelled` directly (cancel before any
lane starts) is permitted, mirroring Spec 077's own early-cancel edge.
Every other transition is `Conflict`/`InvalidArgument`.

### `FleetRun`

```text
FleetRun {
  header: ObjectHeader,
  revision: ProjectRevision,
  project_id: OpaqueId,
  task_prompt: String,            // bounded UTF-8, dispatched identically
                                   // to every lane's own AgentRun.prompt
  status: FleetRunState,
}
```

### `LaneRunRef`

```text
LaneRunRef {
  header: ObjectHeader,
  fleet_run_id: OpaqueId,
  agent_lane_id: OpaqueId,
  agent_run_id: OpaqueId,         // the real Spec 077 AgentRun this lane's
                                   // task dispatch created; 1:1
}
```

Created atomically with each lane's own `AgentRun` creation
(`migration.md` section 5); a `FleetRun` with N bound lanes always has
exactly N `LaneRunRef` rows once dispatch completes, never a partial subset
silently missing a lane.

## 4. Comparison vocabulary

### `ComparisonRequest`

```text
ComparisonRequest {
  fleet_run_id: OpaqueId,         // must be in a terminal-or-partial state:
                                   // Completed | PartiallyFailed (a
                                   // Failed/Cancelled fleet has no usable
                                   // output to compare; requested anyway,
                                   // it is refused, InvalidArgument)
}
```

Not a durable row — a `ComparisonRequest` is the typed input to the
comparison computation; only its output (`ComparisonReport`) is persisted.

### `ComparisonObservation`

```text
ComparisonObservation {
  kind: ComparisonObservationKind, // Agreement | Disagreement |
                                    // ContradictionCandidate |
                                    // EvidenceOverlap | UnsupportedClaim |
                                    // Abstention | SchemaValidity |
                                    // ResourceRuntimeFact
  participating_lane_ids: Vec<OpaqueId>, // which lanes this observation
                                          // concerns (>=1; >=2 for
                                          // Agreement/Disagreement/
                                          // ContradictionCandidate/
                                          // EvidenceOverlap)
  detail: String,                 // bounded UTF-8, human-readable factual
                                   // description; never a score, never a
                                   // ranking phrase
  evidence_refs: Vec<OpaqueId>,   // AgentProposal/RunReceipt/tool-receipt
                                   // ids this observation is grounded in
}
```

**Structural constraint (frozen, security-relevant):** `ComparisonObservation`
has no numeric quality/confidence/rank field anywhere in its shape. Every
kind is a factual, evidence-grounded observation, never a verdict. This is
proven by contract shape (the struct has no such field, not merely "the
field happens to be unused") and re-verified by review.

### `ComparisonReport`

```text
ComparisonReport {
  header: ObjectHeader,
  fleet_run_id: OpaqueId,         // 1:1 with the FleetRun it was computed over
  observations: Vec<ComparisonObservation>,
  participating_lane_ids: Vec<OpaqueId>,   // every lane actually included
  excluded_lane_ids: Vec<OpaqueId>,        // lanes bound to the FleetRun
                                             // but excluded from this report
                                             // (failed/cancelled lanes;
                                             // named explicitly, never
                                             // silently dropped)
}
```

**T078-01 reconciliation:** the pre-implementation sketch above (and
`SPEC_078_PROMOTION.md` constraint 6) named a `classification:
DataClassification` field that would inherit the most restrictive
classification among participating lanes' contexts. Live inspection during
T078-01 (`grep -rn "Sensitivity\|DataClass\|privacy\|Privacy" crates/
medscale-contracts/src`) found **no existing data-classification/privacy-
sensitivity primitive anywhere in this repository** -- `RealmId`/
`AuthorityScopeId` are authorization-scope identifiers, not a
sensitivity/classification taxonomy, and no `DataClass`-shaped type exists
before Spec 079 (Privacy Gate), which is this repository's actual owner of
that contract (`RESEARCH_OS_V2_SPEC_IMPLEMENTATION_CONTRACTS.md`'s 079
entry: `DataClass`). Building even a minimal classification enum here would
be exactly the out-of-scope "new data-class/egress policy engine" the
promotion's "Explicitly not authorized" list forbids. Per the same
reconciliation discipline Spec 077's own `contracts.md` used for
`AgentRun.started_at_seq`/`ToolReceipt.resolution` (real gaps found and
fixed during implementation, not silently carried forward), the
`classification` field is dropped from `ComparisonReport`'s v1 frozen
shape. `SPEC_078_PROMOTION.md`'s mandatory architecture constraint 6 is
amended accordingly (see that document's own T078-01 amendment note): a
`ComparisonReport` in this spec's scope carries no classification field at
all, rather than a fabricated or misapplied one; a future spec (Spec 079,
or a dedicated classification-primitive addition) may add it back
additively once a real classification primitive exists to inherit from.

A `ComparisonReport` is computed once per completed comparison request and
persisted immutably; re-running a comparison over the same `FleetRun`
(e.g. after a later spec adds a new observation kind) creates a new
`ComparisonReport`, never an in-place mutation of a prior report (matching
the append-only, audit-preserving discipline Spec 077 established for
`RunReceipt`).

## 5. Lane-plurality design decision (frozen at promotion, see
## `docs/planning/SPEC_078_PROMOTION.md`)

`AgentLane.agent_identity_id` may point at an `AgentIdentity` that another
`AgentLane` in the same or a different `FleetRun` also points at. Two lanes
sharing one `AgentIdentity` (and therefore one admitted Pack) are still two
distinct `AgentLane` objects with independently-chosen `context_manifest_id`
and `LanePolicy`, and therefore produce two independently-executed,
independently-receipted `AgentRun`s. Nothing in this spec's contracts
requires `AgentLane.agent_identity_id` to be unique within a `FleetRun`;
requiring that would silently assume a repository model inventory this
spec cannot guarantee.

## 6. Freeze record (T078-01, to be filled in against real Rust source)

```text
CONTRACTS_MODULE = crates/medscale-contracts/src/model_fleet.rs (771 lines)
SCHEMA_VERSION_CONST = MODEL_FLEET_SCHEMA_VERSION: u32 = 1
BOUND_CONSTANTS = ROLE_LABEL_MAX_CHARS = 128, TASK_PROMPT_MAX_BYTES =
  32_768, OBSERVATION_DETAIL_MAX_CHARS = 2_048, FLEET_RUN_MAX_LANES = 8
  (enforced by Core, not a struct field), COMPARISON_REPORT_MAX_OBSERVATIONS
  = 512
ENUM_VOCABULARIES = AgentLaneStatus {Active, Retired}, FleetRunState
  {Pending, Running, Completed, PartiallyFailed, Failed, Cancelled},
  ComparisonObservationKind {Agreement, Disagreement,
  ContradictionCandidate, EvidenceOverlap, UnsupportedClaim, Abstention,
  SchemaValidity, ResourceRuntimeFact}. Every enum: const as_str() +
  parse(&str) closed-vocabulary round trip (contracts.md convention,
  mirrors medagent.rs).
VALIDATION_HELPERS = bounded_text/bounded_bytes, deliberately
  re-implemented in this module (not imported from medagent.rs), mirroring
  medagent.rs's own T077-01 convention of staying independent of
  collaboration.rs's private helpers -- this module stays independent of
  medagent.rs's private helpers the same way, even though it imports
  medagent.rs's public types directly (AgentCapabilityManifest,
  AgentRunState, ContextManifest, ToolKind).
LANETRANSFORM_VARIANT_COUNT = 0 (closed-empty enum, `pub enum
  LaneTransform {}`; proven uninhabited at the type level by
  lane_transform_is_closed_empty(), not merely documented in prose)
CLASSIFICATION_TYPE_REUSED = none -- see the T078-01 reconciliation note
  above section 6: no classification/privacy-sensitivity primitive exists
  anywhere in this repository yet (verified by grep); the `classification`
  field was dropped from `ComparisonReport`'s frozen shape rather than
  fabricated or misapplied.
TEST_COUNT = 12 (#[test] functions in crates/medscale-contracts/src/
  model_fleet.rs: agent_lane_status_round_trips,
  lane_transform_is_closed_empty, fleet_run_state_transition_table_is_frozen,
  fleet_run_state_terminal_classification,
  fleet_run_aggregate_state_covers_every_case,
  lane_policy_validate_within_rejects_superset_tool_kind,
  lane_policy_validate_within_rejects_out_of_manifest_artifact,
  agent_lane_new_validates_role_label,
  fleet_run_new_rejects_empty_and_oversized_prompt,
  comparison_observation_kind_requires_multiple_lanes_is_correct,
  comparison_observation_validate_enforces_multi_lane_kinds,
  comparison_report_validate_rejects_lane_in_both_lists)
IMPORT_PURITY = only serde + crate::medagent::{AgentCapabilityManifest,
  AgentRunState, ContextManifest, ToolKind} (Spec 077's own frozen
  contracts, reused unmodified) + crate::objects::{ObjectHeader, OpaqueId}
  + crate::project_graph::{ProjectRevision, check_revision,
  initial_revision} -- no storage/network/UI/model-runtime dependency,
  and no serde_json dependency in this module (no Value-typed field, unlike
  medagent.rs's AgentTurn/ToolInvocation/ToolReceipt payloads) -- verified
  by inspection of the `use` block, this session.
```
