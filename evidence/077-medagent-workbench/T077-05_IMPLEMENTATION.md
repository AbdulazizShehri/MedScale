# T077-05 Implementation — AgentRun lifecycle

## Scope

`AgentRun` create/get/list/start/cancel through Core authority, the full
frozen state machine (`Pending -> Running -> {Cancelled, Completed,
Failed}`, plus early `Pending -> Cancelled`), and `AgentTurn` as an
append-only per-step record.

## Design decisions

**`AgentTurn`s are never externally appendable.** There is no
`Capability`/`RequestBody` that lets a caller inject an arbitrary turn.
`start_agent_run` auto-appends the run's initial `PromptSubmitted` turn
using the run's own already-known `prompt` field -- no caller input is
trusted for turn content at all. `ToolRequested`/`ToolResult` turns
(T077-06) and `ModelOutput` turns (T077-07) will be Core-internal side
effects of their own dispatch paths, appended the same way. This closes an
obvious integrity gap: if turns were externally appendable, a caller could
fabricate fake "the model said X" history.

**`security.md` T5 is enforced at both `create_agent_run` and
`start_agent_run`**, via a shared `require_active_identity` helper that
re-fetches the identity's `status` and the currently admitted
`PackManifestV0` for its `pack_id`, comparing `pack_version` -- never
cached across the run's lifetime. A revoked identity, or one whose
originally-bound Pack version has since been superseded, refuses both a
new run creation and starting an already-`Pending` one.

**`create_agent_run` refuses cross-project identity/context.** Neither
`AgentRunState` nor `AgentTurn`'s frozen shape says anything about this,
but it is an obvious integrity gap the founder's directive explicitly
named ("cross-project object IDs" as an adversarial case to challenge):
without this check, a caller could name `project_id: A` while supplying an
`agent_identity_id`/`context_manifest_id` that actually belong to Project
B, silently cross-wiring two unrelated projects' resources into one run.
Both `identity.project_id` and `context.project_id` are compared against
the run's own `project_id` and refused with `InvalidArgument` on mismatch.

**`cancel_agent_run` always commits an empty-`tool_invocation_ids`
`RunReceipt`.** No tool invocation exists yet at T077-05 (that is T077-06);
threading real invocation ids through a cancelled run's receipt is that
task's and T077-08's concern, not a T077-05 gap.

## Files changed

```text
crates/medscale-contracts/src/envelopes/mod.rs
  - Capability::AgentRunCreate/Read/Start/Cancel
  - RequestBody::AgentRunCreate/Get/List/Start/Cancel/TurnList
  - ResponseBody::MedAgentRun/MedAgentRunList/MedAgentRunStarted/
    MedAgentRunTerminal/MedAgentTurnList

crates/medscale-core/src/authority/medagent.rs
  - MedAgent::scoped_agent_run/require_active_identity/create_agent_run/
    get_agent_run/list_agent_runs/start_agent_run/cancel_agent_run/
    list_agent_turns

crates/medscale-core/src/authority/facade.rs
  - 6 dispatch arms + 6 capability_matches pairs

crates/medscale-core/src/cli_session.rs
  - CliSession::medagent_run_create/get/list/start/cancel/turns

crates/medscale-cli/src/medagent.rs
  - MedAgentCmd::RunCreate/RunShow/RunList/RunStart/RunCancel/RunTurns

crates/medscale-core/tests/medagent_077.rs
  - Harness::register_identity/create_context/create_run helpers
  - 6 new tests (see below)

specs/077-medagent-workbench/tasks.md
evidence/077-medagent-workbench/T077-05_IMPLEMENTATION.md (this file)
```

## Tests

`crates/medscale-core/tests/medagent_077.rs`, all through real
`CoreFacade::dispatch`:

1. `full_run_lifecycle_start_then_cancel_through_core` -- create, get
   (`Pending`, revision 1), start (asserts `Running`, revision 2, the
   auto-appended `PromptSubmitted` turn at `seq == 1` with the exact
   prompt payload), list turns (exactly 1), cancel (asserts `Cancelled`,
   revision 3, a committed `RunReceipt` with `final_state == Cancelled`,
   no `failure_reason`, empty `tool_invocation_ids`).
2. `pending_run_can_be_cancelled_directly_without_starting` -- the early
   `Pending -> Cancelled` edge; asserts zero turns were ever appended.
3. `cancellation_race_only_one_request_wins` -- two cancel requests both
   carrying `expected_revision: 1` (simulating two callers who both
   observed the same pre-race state); asserts exactly one succeeds and
   the other fails closed with `Conflict` (never silently re-applies), and
   that the run's final revision is 2, not 3 -- proving the loser mutated
   nothing, not merely that it returned an error.
4. `illegal_transitions_are_rejected` -- `Cancelled -> Running` and
   `Cancelled -> Cancelled` both rejected with `Conflict`, proving no edge
   outside the frozen table is reachable through Core.
5. `run_creation_rejects_cross_project_identity_and_context` -- an
   identity from Project A used with `project_id: B` is refused; an
   identity correctly paired with its own Project A but a context manifest
   from Project B is refused too (proving it is genuinely the context
   check firing, not just the identity check).
6. `revoked_identity_fails_closed_at_create_and_at_start` -- a run created
   while the identity is `Active`, then the identity is revoked before the
   run starts: `AgentRunStart` refuses with `Unauthorized`, and a fresh
   `AgentRunCreate` attempt against the same now-revoked identity is
   refused the same way.

No local compile/test run was possible (MSVC linker absent, same
constraint as every prior spec). `cargo fmt --check` is clean across the
whole workspace after this change.

## Exact-head CI qualification

Green on the first push, no fixes needed:

```text
HEAD f170422 -> SUCCESS, run 35714251561, 6/6.
```

T077-05 is qualified at exact head `f170422a71d406a022b295b7a75076d39332d14f`.
