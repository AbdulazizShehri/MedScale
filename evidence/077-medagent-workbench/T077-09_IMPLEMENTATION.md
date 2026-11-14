# T077-09 Implementation — Native Desktop and CLI parity

## Scope

MedAgent navigation through the current native Slint composition, backed
exclusively by Core: run list/detail, context selection, prompt
submission, cancel/interrupt. Design-system/keyboard/focus/accessibility/
light-dark parity preserved. CLI vertical slice complete. Real-Core-session
test for every new Desktop view-model function.

## Pattern followed

`crates/medscale-desktop/src/collaboration_workspace.rs` (Spec 076) is the
exact template: a plain Rust module of `fn(&mut CliSession, ...) ->
Result<RowVm, AuthorityError>` functions, never touching storage or (for
MedAgent) the model runtime directly, mapped in `main.rs` to Slint model
rows via `ModelRc::new(VecModel::from_iter(...))`, with UI actions routed
through the single `callback ui-action(string)` every panel in this
Desktop shell already uses (a colon-delimited action string, e.g.
`"medagent-run-cancel:" + item.id + ":" + item.revision`, parsed in
`main.rs` via `action.strip_prefix(...)`/`rsplit_once(':')`).

## Design decisions

**Context/identity selection is CLI-parity text entry, not a picker
widget.** The task bullet says "context selection," and this panel
provides it as two text fields (agent identity id, context manifest id)
the operator pastes in before creating a run. This mirrors
`collaboration_workspace.rs`'s own established precedent exactly:
`collab-thread-artifact-input` is the same kind of "paste the id you
already have" field for opening a thread, not a artifact browser. Agent
identity registration and `ContextManifest` creation remain CLI-only in
this slice -- consistent with Spec 076's own Desktop panel leaving
Notes/Approvals CLI-only, not a new precedent.

**"Create + start" is one action, not two.** The panel's only run-creation
control creates a `Pending` run and immediately starts it
(`Pending -> Running`) in one operator action, rather than exposing a
separate "start" button for a run sitting in `Pending`. This is a genuine
UX simplification for v1: a `Pending` run with no started work is not a
state the current panel needs an operator to linger in on purpose (there
is no reason to create a run now and start it later through this
surface). The `RunRowVm`/CLI layer still support the two-step flow
independently (`RunCreate` then `RunStart` are still separate Core
capabilities); the Desktop panel simply chooses not to expose that as two
clicks.

**Cancel ("interrupt") is only offered for `pending`/`running` rows.**
The Slint template gates the Cancel `QuickAction` on
`item.status == "pending" || item.status == "running"`, matching the
frozen `AgentRunState` transition table exactly (only those two states can
transition to `Cancelled`) -- a terminal row never shows a control that
would just bounce off `Conflict`.

**No new CLI surface.** T077-03 through T077-08 already built the full
CLI vertical slice (`medagent identity-*`/`context-*`/`run-*`/
`tool-invoke`, all with `--json`). This task's CLI-parity bullet is
satisfied by that existing surface; nothing new was added here.

## Session-holder-identity assumption -- checked, found not applicable

Spec 076's own `collab_workspace_flows_through_real_core_session` test
caught a real bug (`ensure_self_participant` registering under the
literal string `"desktop-operator"` instead of the session's real bound
holder id) because Collaboration requires a caller to self-register a
`ParticipantIdentity` tied to the session's actual holder before any
room/thread/message/task action can succeed. MedAgent's authority model
has no analogous step: `AgentRunCreate`/`AgentIdentityRegister`/etc. are
scoped by Project and `AgentIdentity`, never by a holder-registered
participant row. There is therefore no equivalent "wrong holder id"
assumption for this panel to get wrong, and this session verified that by
inspection (reading `MedAgent`'s Core authority paths, none of which
resolve a caller-registered participant) before concluding there was
nothing further to test here -- a checked-and-ruled-out consideration,
not a skipped one.

## Rendered Desktop evidence

None captured. This workstation cannot compile the Slint UI locally
(MSVC toolchain absent, the same constraint recorded for every prior
spec), and this repository's CI has no rendering step or headless
`AppWindow` test harness (`evidence/077-medagent-workbench/LIVE_TRUTH.md`
already recorded this as an inherited residual from Spec 076, not
introduced here). The Desktop data path is proven by the real-`CliSession`
test instead, exactly like every other Desktop panel in this codebase.

## Files changed

```text
crates/medscale-desktop/src/medagent_workspace.rs (new)
  - RunRowVm, TurnRowVm, status_message, refresh_runs, create_run,
    start_run, cancel_run, refresh_turns
  - inline #[cfg(test)] mod tests: medagent_workspace_flows_through_real_core_session,
    status_message_never_empty

crates/medscale-desktop/src/main.rs
  - mod medagent_workspace;
  - medagent_run_row/refresh_medagent_runs/refresh_medagent_turns/select_medagent_run helpers
  - ui-action arms: medagent-runs-refresh, medagent-run-create-start,
    medagent-run-select:<id>:<rev>, medagent-run-cancel:<id>:<rev>

crates/medscale-desktop/ui/app.slint
  - AgentRunRowItem, AgentTurnRowItem structs
  - root properties: medagent-status/runs/agent-id-input/context-id-input/
    prompt-input/active-run-id/active-run-revision/turns
  - NavItem "MedAgent" (OPERATIONS group, alongside Collaboration)
  - route subtitle line
  - MedAgent content panel (create-and-start form, run list with
    status-gated Cancel, turn list for the selected run)

specs/077-medagent-workbench/tasks.md
evidence/077-medagent-workbench/T077-09_IMPLEMENTATION.md (this file)
```

No local compile/test run was possible for the Rust side (MSVC linker
absent, same constraint as every prior spec); the Slint side additionally
cannot be locally compiled or rendered at all on this workstation (no
working native toolchain for the `slint-build` step). `cargo fmt --check`
is clean across the whole workspace for the Rust files; the `.slint` file
was hand-verified for brace balance (`807` open, `807` close across the
whole file) and cross-checked line-by-line against
`collaboration_workspace.rs`'s/`app.slint`'s existing, already-compiling
Collaboration block for syntax fidelity, since no local Slint compiler
was available to check it directly. ## Exact-head CI qualification

Green on the first push, no fixes needed -- including `rust
(windows-latest)`'s job, which compiles the native Desktop binary and
the hand-written `.slint` markup through the real Slint compiler:

```text
HEAD f1ba3d7 -> SUCCESS, run 35728751836, 6/6.
```

T077-09 is qualified at exact head `f1ba3d762c3049a34b8a2dce891d89e3d526c1e5`.
