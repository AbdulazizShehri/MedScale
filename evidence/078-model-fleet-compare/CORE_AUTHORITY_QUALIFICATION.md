# Core Authority Qualification — Spec 078 (T078-03..T078-06)

All 078 operations enter through `CoreFacade::dispatch` ->
`capability_matches` -> lease -> `ModelFleet` (`crates/medscale-core/src/
authority/model_fleet.rs`). Spec 077 objects are reached only through
`MedAgent`'s public entry points (`get_agent_identity`,
`get_context_manifest`, `get_agent_run`, `create_agent_run`,
`start_agent_run`, `execute_agent_run`, `complete_agent_run`,
`fail_agent_run`, `cancel_agent_run`). The comparison reads a completed
lane run's committed `RunReceipt`/`AgentProposal` rows read-only, after that
run has been scope-checked through `get_agent_run`, because Spec 077 exposes
no public Core read for them.

## Operation matrix

| Request | Capability | Mutation | Refusals (tested) |
|---|---|---|---|
| `AgentLaneCreate` | `AgentLaneCreate` | lane row | superset / ungranted tool kind, out-of-manifest or partial-overlap artifact, empty subsets, revoked identity, cross-Project identity/context, missing objects, bounded role label |
| `AgentLaneGet` / `AgentLaneList` | `AgentLaneRead` | none | wrong scope |
| `AgentLaneRetire` | `AgentLaneRetire` | status + revision (CAS) | stale revision, already retired, missing |
| `FleetRunCreate` | `FleetRunCreate` | fleet row (`Pending`) | bounded prompt (contract) |
| `FleetRunGet` / `FleetRunList` | `FleetRunRead` | none | wrong scope |
| `FleetRunDispatch` | `FleetRunDispatch` | fleet `Pending -> Running` (CAS); per lane: Spec 077 run create -> `LaneRunRef` -> Spec 077 start | fewer than 2 or more than 8 lanes, duplicate lane, retired lane, other-Project lane, revoked identity, non-admitted Pack, stale revision, second dispatch; all with zero writes |
| `FleetRunExecuteLane` | `FleetRunExecuteLane` | Spec 077 execute + complete/fail on the lane's own run; fleet re-aggregated (CAS) | real-PHI gate (`ExternalGateRequired`, lane untouched), unbound lane (`NotFound`), fleet not `Running`, lane run not `Running` |
| `FleetRunCancel` | `FleetRunCancel` | in-flight lanes via Spec 077 cancel; fleet terminal (CAS) | stale revision, terminal fleet |
| `ComparisonCompute` | `ComparisonCompute` | one new report row | fleet not `Completed`/`PartiallyFailed` (`InvalidArgument`), missing fleet |
| `ComparisonReportList` | `ComparisonRead` | none | wrong scope |
| `AgentToolInvoke` (Spec 077) | `AgentToolInvoke` | unchanged Spec 077 behavior, after the lane-policy guard for lane-bound runs | lane-excluded tool kind / artifact, context-wide search on a narrowed lane, malformed args |

A capability/body mismatch is refused before any authority code
(`lane_capabilities_are_distinct_from_spec_077_capabilities`).

## Decisions recorded

- `FLEET_RUN_MIN_LANES = 2` (Core constant): a fleet compares lanes, and a
  single-lane fleet cannot produce any multi-lane observation. The frozen
  contract bounds only the maximum (`FLEET_RUN_MAX_LANES = 8`).
- A lane run that fails execution (including a refused execution such as
  a Pack-identity mismatch) is closed `Failed` through Spec 077 with a fixed
  reason, not left `Running`: the fleet's partial-failure state must reflect
  what actually happened. The real-PHI gate is checked first and never
  touches the lane run.
- A fleet whose lanes were all closed directly through Spec 077 stays
  `Running` until a fleet operation re-aggregates it; `FleetRunCancel` then
  lands the aggregate of the lanes' real states.

## Tests

`crates/medscale-core/tests/model_fleet_078.rs` (T078-03: 8 tests,
T078-04: 7 tests, T078-05/06: 3 tests) plus unit tests in
`authority/model_fleet.rs` (T1 structural) and
`authority/model_fleet_compare.rs` (6). Results are recorded against the
exact CI head in `EXACT_HEAD_QUALIFICATION.md`.
