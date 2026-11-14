# Security and Threat Delta — Spec 078

**Status:** implementation contract

Spec 078 adds local-first multi-lane orchestration and comparison on top of
Spec 077's already-qualified agent execution. It adds no new product
runtime network path, no remote/bounded worker, no browser/external-
provider capability, no Hub sync execution, no real-PHI authorization, and
no modification to any Spec 077 contract, storage table, or Core authority
function.

## 1. Trust boundary

```text
CLI/Desktop
   -> typed Core command/query
      -> actor/session/scope checks (Core Capability, unchanged mechanism)
         -> AgentLane policy check (narrows, never widens, the bound
            AgentIdentity's AgentCapabilityManifest and ContextManifest)
            -> per-lane dispatch into Spec 077's UNMODIFIED
               create_agent_run / start_agent_run / execute_agent_run /
               cancel_agent_run (each lane's run is a real Spec 077
               AgentRun, checked by Spec 077's own T2/T4/T5 controls
               exactly as any other caller)
               -> FleetRunState aggregation (read-only aggregation of
                  each lane's independent AgentRunState; never a second
                  authority decision)
                  -> ComparisonReport computation (read-only over
                     completed lanes' AgentProposal/RunReceipt; no write
                     path back into any lane's run)
```

No CLI/Desktop component may access 078 tables directly. No 078 mutation
path may call any other subsystem's mutation path (effects, actions/
outbox, proposal promotion, clinical-assertion creation) directly or
transitively, and no 078 code path may call any Spec 077 Core function
other than the existing public entry points Spec 077 itself already
exposes to other callers.

## 2. Threats and required controls

### T1 — Fleet/comparison output triggers effect escalation

Attack: a `ComparisonReport`, or a `FleetRun`'s aggregation logic, is
wired, now or by a future careless change, to automatically call
`PromoteProposal`, `TransitionEffect`, `CreateExternalActionIntent`, or any
clinical/external effect, or to automatically "resolve" a disagreement
between lanes into a single accepted answer.

Controls:

- `ComparisonReport` computation is a pure read: it reads existing
  `AgentProposal`/`RunReceipt`/`ToolReceipt` rows already committed by
  Spec 077's own machinery and writes exactly one new 078-owned row; it has
  no code path to any effect/action/`ClinicalAssertion`-creation/proposal-
  promotion function, verified structurally by dependency-direction review,
  not just by test;
- `FleetRun` state aggregation never selects a "winning" lane's
  `AgentProposal` for any downstream authority purpose; every participating
  lane's `AgentProposal` remains individually addressable and none is
  privileged over another anywhere in 078's own code;
- the acceptance test suite includes a structural check that
  `authority::model_fleet` (078's Core module) contains no call into
  `authority::promote`, `authority::amend`, or `contracts::actions`
  (mirroring Spec 076/077's T1 discipline exactly).

Tests: computing a `ComparisonReport` produces no side effect outside 078
tables; a direct unit test asserting no cross-module call exists into
promote/amend/actions.

### T2 — Lane policy escalation beyond the underlying identity/context

Attack: an `AgentLane`'s `LanePolicy` grants a tool kind the bound
`AgentIdentity`'s `AgentCapabilityManifest` does not grant, or names a
context artifact the bound `ContextManifest` does not select, effectively
widening what a lane's runs may do beyond what Spec 077's own checks would
otherwise allow.

Controls:

- lane creation validates `LanePolicy.granted_tool_kinds` is a subset of
  the bound identity's `AgentCapabilityManifest.granted_tool_kinds` (when
  `Some`), and `LanePolicy.context_artifact_ids` is a subset of the bound
  `ContextManifest.selected_artifacts` (when `Some`); a superset request is
  refused (`InvalidArgument`) before any row is written;
- this check is re-run, not cached, at every lane-run dispatch — a
  `ContextManifest` or `AgentCapabilityManifest` cannot be narrowed after a
  lane was created (both are immutable per Spec 077) so this is a
  defense-in-depth re-check against implementation drift, not a response to
  a live narrowing threat;
- 078 never constructs a Spec 077 `ContextManifest`/`AgentCapabilityManifest`
  wider than what the lane's policy allows when dispatching that lane's
  `AgentRun` — the dispatch call passes the lane's own already-bound
  `context_manifest_id`/`agent_identity_id` unchanged; 078 has no code path
  that fabricates a new, wider manifest or capability set on a lane's
  behalf.

Tests: a `LanePolicy` naming an ungranted tool kind or an out-of-manifest
artifact is refused at lane creation; a lane's dispatched `AgentRun` is
verified to bind the exact `context_manifest_id`/`agent_identity_id` the
lane was created with, never a substituted wider one.

### T3 — Cross-lane permission union / information leakage

Attack: one lane's in-flight run reads another lane's context, tool
results, or in-progress output, either because a `FleetRun` shares state
across lanes or because a Core function meant for one lane accidentally
resolves data scoped to a sibling lane.

Controls:

- each lane's `AgentRun` executes through Spec 077's existing,
  per-run-scoped `execute_agent_run` call, which already resolves context
  only through that run's own bound `ContextManifest` (Spec 077's T4
  control, unmodified); 078 introduces no shared read path across lanes;
- `FleetRun`/`LaneRunRef` aggregation only ever reads already-terminal (or
  currently-independent, still-running) lane state for status/comparison
  purposes; it never feeds one lane's turn/tool output as input to
  another lane's run;
- reviewed structurally (dependency-direction / call-graph check): no 078
  function accepts one lane's run/context/turn data as a parameter to
  another lane's dispatch call.

Tests: two lanes in the same `FleetRun`, each bound to disjoint
`ContextManifest`s, are proven to be unable to resolve each other's
artifacts through any exposed Core path, even via the `FleetRun`/
`ComparisonReport` surface; a lane's tool invocation cannot name another
lane's `AgentRun`/context/tool-invocation id as an argument and succeed.

### T4 — Fabricated or duplicated comparison ("fake diversity")

Attack: because the repository currently has only one real admitted local
model Pack, a shortcut implementation "compares" a single `AgentRun`'s
output against itself (duplicated), or fabricates a second lane's result
without actually executing an independent run, silently defeating the
entire point of the comparison and misleading a reader into believing two
genuinely independent lanes were exercised.

Controls:

- every `LaneRunRef` in a `FleetRun` must reference a distinct, genuinely
  independently-created and independently-executed Spec 077 `AgentRun`
  (distinct `AgentRun.header.id`, distinct `AgentTurn`/`ToolInvocation`
  history where applicable); reusing one `AgentRun`'s id across two
  `LaneRunRef`s is refused at the storage layer (unique constraint) and at
  the Core layer (explicit check before dispatch);
- the closure-gate fixture proving "two distinct admitted lanes run the
  same task" is required to show two different `AgentRun.header.id`
  values, two different `RunReceipt`s, and (per the promotion's
  lane-plurality decision) deliberately different lane policy so the two
  runs are not merely coincidentally identical inputs — this is checked in
  review, not merely asserted in prose.

Tests: a fixture attempting to bind the same `AgentRun.header.id` to two
`LaneRunRef`s in one `FleetRun` is rejected; the two-lane closure fixture
asserts `lane_a.agent_run_id != lane_b.agent_run_id` and that both runs'
`RunReceipt`s exist independently.

### T5 — Stale-write overwrite on mutable 078 state

Attack: two callers update an `AgentLane` or `FleetRun` from the same old
state and the later request silently overwrites the newer result.

Controls:

- explicit `expected_revision` precondition on every mutable 078 row;
- mismatch -> `Conflict`, no write, no last-write-wins;
- `FleetRunState` transitions are additionally gated by the frozen
  transition table (`migration.md` section 13), so even a revision-matched
  request cannot force an illegal state edge.

Tests: stale lane/fleet-run update is `Conflict`; an illegal fleet state
transition (e.g. `Completed -> Running`) is rejected even with a correct
`expected_revision`.

### T6 — Fleet cancellation race / zombie lane

Attack: a fleet-level cancel request races one or more lanes'
concurrently-completing runs, leaving a lane not actually stopped, or the
fleet landing in an ambiguous/dual terminal state.

Controls:

- fleet-level cancel is implemented as: for every lane whose `AgentRun` is
  still `Pending`/`Running`, issue that lane's own Spec 077
  `cancel_agent_run` (which already has cancellation-race safety, Spec
  077's T7); 078 does not invent a second cancellation primitive;
- the `FleetRun`'s own terminal-state write is revision-guarded like any
  other; the first transition to reach a terminal state wins.

Tests: a simulated race between a fleet cancel request and a lane
concurrently completing proves exactly one terminal state is reached for
both the lane (reusing Spec 077's own T7 test) and the fleet, never both
`Cancelled` and `Completed` for either.

### T7 — Malformed/hostile metadata injection

Attack: oversized/control-character/markup/SQL-like task prompts, role
labels, or comparison-observation detail text cause storage/query/UI
issues.

Controls:

- bounded UTF-8 validation with explicit constants on every free-text
  field (task prompt, role label, observation detail);
- parameterized storage APIs (no string-built SQL);
- UI/CLI renders text as text, never as executable markup;
- logging safely escapes/bounds user/model metadata; task prompt/
  observation bodies never appear unbounded in operational logs.

Tests: empty/whitespace, max boundary, over-max, unusual Unicode, control
characters, SQL-like strings per frozen policy (mirrors Spec 076/077's T8/
T9 suite).

### T8 — Audit tampering / fleet-history rewrite

Attack: a `LaneRunRef`, `FleetRun` terminal record, or `ComparisonReport`
is edited or deleted out of band to hide what a fleet actually did or to
alter a recorded comparison after the fact.

Controls:

- `LaneRunRef`/`ComparisonReport` rows have no update API; storage exposes
  append/terminal-write and read only, exactly like Spec 077's
  `AgentTurn`/`ToolReceipt`/`RunReceipt` precedent;
- migration/backup/restore evidence includes a re-verification pass
  proving a `FleetRun`'s terminal state still matches its recorded
  `LaneRunRef` set and each referenced lane's actual terminal
  `AgentRunState` after restore (learning directly from Spec 076's and
  077's own exact-range review findings that restore paths must
  re-verify cross-row consistency, not merely assume it from insert-time
  checks).

Tests: tamper with a stored `LaneRunRef`/`FleetRun` row directly in a test
fixture and prove the mismatch is detectable; restore-from-backup
re-verifies fleet/lane/comparison consistency and fails closed on
mismatch.

### T9 — Content leakage into logs or cross-scope caches

Attack: task prompt text, lane policy content, or comparison-observation
detail (which may carry sensitive research/clinical discussion) leaks into
operational logs, crash dumps, or a shared cache visible across
authorization scopes.

Controls:

- logging conventions already required elsewhere apply identically:
  free-text bodies are never logged in full at info/debug level by
  default;
- 078 introduces no new shared cache; comparison computation reads are
  scoped per `FleetRun`, not memoized across authorization contexts.

Tests: debug-log capture scans across every 078 mutation path assert no
raw task-prompt/policy/observation body appears unbounded in log output.

### T10 — Half-committed state after crash

Attack: a `FleetRun`'s terminal-state write commits but a `LaneRunRef` or
`ComparisonReport` append does not (or the reverse), leaving an
untraceable fleet or an orphaned comparison.

Controls:

- every terminal-state transaction includes its dependent rows in the same
  commit, mirroring Spec 077's `RunReceipt`-with-terminal-transition
  discipline exactly (`migration.md` section 5);
- migration journal fail-closed semantics from the existing framework are
  preserved for v6 -> v7.

Tests: crash-point matrix from `migration.md` section 6, including "before
a `ComparisonReport` commits."

### T11 — Dependency and supply-chain smuggling

Attack: 078 pulls in a new dependency (e.g. for a second inference engine
to manufacture lane diversity) that introduces copyleft/transitive risk,
unreviewed native-code execution, or unqualified behavior.

Controls:

- 078's foundation requires no new dependency beyond what `medscale-pack`/
  Spec 008/069 already qualifies and what Spec 077 already depends on; the
  lane-plurality scope decision (`SPEC_078_PROMOTION.md`) explicitly
  forbids building a second inference engine to manufacture fake model
  diversity;
- if a later slice genuinely needs a new dependency, exact version/
  license/security/exit review is recorded in the evidence packet before
  admission, and `cargo-deny`/supply-chain gates must pass on the exact
  head.

Tests: dependency-direction and supply-chain gates green; if `Cargo.toml`
gains any new dependency, a review record exists explaining why the
existing `medscale-pack`/Spec 077 surface was insufficient.

## 3. Explicit non-capabilities

078 introduces no capability for: triggering a clinical/external effect
from fleet/comparison output, automatically resolving a lane disagreement,
cross-lane permission union, ambient vault access outside a lane's already-
bound `ContextManifest`, browser/network tool access, a second inference
engine, remote/bounded-worker run execution, or any modification to a Spec
077 contract, storage table, or Core authority function. Any test or review
finding suggesting such a capability is a blocking defect, not a scope
question.
