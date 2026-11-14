# T077-08 Implementation — RunReceipt + run history

## Scope

`RunReceipt` committed atomically with a run's terminal transition
(all three: `Cancelled`/`Completed`/`Failed`), and bounded, filterable
run-history read (by Project, agent identity, and state).

## What was found and fixed

T077-05's `cancel_agent_run` already committed its `RunReceipt`
atomically with the terminal transition (reusing storage's own
`commit_terminal_transition_with_receipt`, which itself commits the
primary-row write and the receipt insert in one SQLite transaction). But
it always set `tool_invocation_ids: Vec::new()`, unconditionally -- at
T077-05 that was accurate (no tool invocation existed yet), but by
T077-08, after T077-06 added real tool dispatch, a run cancelled
*mid-execution*, after one or more tools had already run, would get a
`RunReceipt` claiming zero tool invocations happened, when some genuinely
did. This is exactly the class of gap this spec's own evidence discipline
exists to catch (mirroring Spec 076's own exact-range review finding a
similar restore-time gap): found and fixed in this session before any
external review, not left for later discovery.

## Design decisions

**One shared `commit_terminal_run` helper for all three terminal
states.** `cancel_agent_run`, `complete_agent_run`, and `fail_agent_run`
all call the same private helper, parameterized by `final_state` and an
optional `failure_reason`. This guarantees the three states can never
drift into recording different receipt shapes or different
tool-invocation-threading behavior -- there is exactly one place that
decides what goes into a terminal `RunReceipt`.

**`tool_invocation_ids` includes both `Executed` and `Refused`
invocations, in `seq` order.** The acceptance bullet's "exact provenance"
reading is the full record of what was requested and what happened to it,
not merely a list of successes -- a refused tool call is still part of
the run's real history and belongs in its receipt.

**"Resource/timing facts" (from `plan.md`'s work bullet) are not
separately tracked.** `RunReceipt`'s frozen contract (`contracts.md`,
T077-01) has exactly these fields: `header`, `run_id`, `pack_id`,
`pack_version`, `context_manifest_id`, `context_manifest_revision`,
`tool_invocation_ids`, `final_state`, `failure_reason` -- no
timestamp/duration/resource-usage field. This promotion's authority did
not extend to amending that already-frozen shape, and no other T077-*
task bullet asks for one either. Turn count (`list_agent_turns`) and tool
invocation count (`tool_invocation_ids.len()`) are the closest "resource
facts" this spec's actual frozen contract supports. Recorded here as an
honest residual rather than silently ignored or falsely claimed as done.

**Filterable run history uses a real SQL `WHERE`, not a post-fetch
filter.** `migration.md` section 3 already specifies
`idx_medagent_runs_project(project_id, status)` -- an index that existed
since T077-02 but was never actually used by any query, because
`list_agent_runs` had no status parameter until now. A post-fetch filter
(fetch `limit` rows, then discard non-matching ones in Rust) would have
been simpler to write but silently wrong for pagination: it could return
fewer than `limit` matching rows even when more exist beyond the
pre-filter window. The storage function now branches over all four
`(agent_id, status)` presence combinations with the appropriate `WHERE`
clause, so `limit` always bounds the actual matching set.

## Files changed

```text
crates/medscale-storage/src/medagent.rs
  - list_agent_runs gains status: Option<AgentRunState>, branches over
    4 WHERE-clause combinations using idx_medagent_runs_project

crates/medscale-contracts/src/envelopes/mod.rs
  - Capability::AgentRunComplete/AgentRunFail
  - RequestBody::AgentRunComplete/AgentRunFail
  - RequestBody::AgentRunList gains status: Option<AgentRunState>

crates/medscale-core/src/authority/medagent.rs
  - MedAgent::commit_terminal_run (shared helper) + cancel_agent_run
    refactored onto it + complete_agent_run/fail_agent_run
  - list_agent_runs gains the status parameter, passed through to storage

crates/medscale-core/src/authority/facade.rs
  - AgentRunList dispatch arm passes status through
  - 2 new dispatch arms (AgentRunComplete/AgentRunFail) + capability_matches pairs

crates/medscale-core/src/cli_session.rs
  - medagent_run_list gains a status parameter
  - CliSession::medagent_run_complete/medagent_run_fail

crates/medscale-cli/src/medagent.rs
  - RunList gains --status; MedAgentCmd::RunComplete/RunFail

crates/medscale-core/tests/medagent_077.rs
  - 4 new tests (see below)

specs/077-medagent-workbench/tasks.md
evidence/077-medagent-workbench/T077-08_IMPLEMENTATION.md (this file)
```

## Tests

`crates/medscale-core/tests/medagent_077.rs`, all through real
`CoreFacade::dispatch`:

1. `completed_run_receipt_threads_real_tool_invocation_ids` -- invokes a
   real tool mid-run, then completes the run, and asserts the receipt's
   `tool_invocation_ids` contains exactly that invocation's real id (the
   gap this session found and fixed, proven not to regress).
2. `fail_agent_run_records_failure_reason_in_receipt` -- asserts the exact
   failure reason string round-trips into the committed receipt.
3. `completing_a_pending_run_is_rejected` -- `Pending -> Completed` is not
   in the frozen table; fails closed with `Conflict`.
4. `run_history_is_filterable_by_status` -- two runs in different states
   (`Pending`, `Cancelled`) in one Project; filtering by each status
   returns exactly the matching one, and no filter returns both.

No local compile/test run was possible (MSVC linker absent, same
constraint as every prior spec). `cargo fmt --check` is clean across the
whole workspace after this change.

## Exact-head CI qualification

Green on the first push, no fixes needed:

```text
HEAD 9955cad -> SUCCESS, run 35725135366, 6/6.
```

T077-08 is qualified at exact head `9955cadda2e14e0fc845090355daa70d812bdfdd`.
