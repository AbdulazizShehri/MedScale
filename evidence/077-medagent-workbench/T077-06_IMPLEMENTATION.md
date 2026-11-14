# T077-06 Implementation — Tool invocation

## Scope

Typed `ToolInvocation` dispatch through Core, policy-checked against
`AgentCapabilityManifest` (grant check) and `ContextManifest`
(`require_artifact_in_context`, added at T077-04 and now finally called),
executed entirely by Core -- the model may only request a tool by name and
typed arguments, never author the execution itself.

## Design decisions

**Refusal vs. hard error is a deliberate split.** Three distinct failure
classes exist in `invoke_tool`:

1. Run/session/identity problems (run not found, wrong scope, run not
   `Running`, identity revoked) -- these are the *caller's* fault, not the
   model's, and return a hard `AuthorityError` before any `ToolInvocation`
   row is even considered. No turn is appended either: the request never
   reached the point of being "the model asked for a tool."
2. Capability/argument/boundary problems (ungranted tool kind, oversized
   or malformed arguments, an artifact outside the bound `ContextManifest`)
   -- these are "the model asked for something disallowed," recorded as a
   `Refused` `ToolInvocation` with a human-readable reason, wrapped in a
   normal `Ok` response. The model (via T077-07's future loop) is meant to
   see this refusal as its next turn's input, not have the whole run error
   out.
3. Success -- an `Executed` `ToolInvocation` plus its `ToolReceipt`.

**Every invocation appends exactly two turns, unconditionally.**
`ToolRequested` is appended *before* the grant/boundary check (it records
what was asked, regardless of outcome -- an audit trail should show
refused requests too, not just successful ones), and `ToolResult` is
appended *after*, carrying either `{"status":"refused","reason":...}` or
`{"status":"executed","result":...}`. This mirrors `AgentTurnKind`'s
frozen four-variant vocabulary (`PromptSubmitted`/`ToolRequested`/
`ToolResult`/`ModelOutput`) exactly as designed at T077-01, rather than
inventing a combined "request+outcome" turn.

**Typed argument parsing (`security.md` T3).** `ReadContextArtifactArgs`
and `SearchContextArtifactsArgs` are private, `#[serde(deny_unknown_fields)]`
structs Core-internal to `medagent.rs` (not added to `contracts.md`'s
frozen `ToolManifest`/`ToolInvocation` shapes, since those already committed
to `arguments: Value` as the wire shape at T077-01 -- these structs are
the validation step *between* that `Value` and any actual use, exactly
what `security.md` T3 requires). A shape that fails to parse is a
`Refused` invocation with `"malformed arguments: <serde error>"`, never a
partial value used anyway.

**`ReadContextArtifact`/`SearchContextArtifacts` execution is a
deliberate simplification**, matching the note already recorded in
`T077-04_IMPLEMENTATION.md`: only `SourceRecord`/`DerivedSourceArtifact`
carry readable `bytes`; any other in-context artifact kind refuses with
"artifact kind does not carry readable content" rather than attempting a
generic `to_json()` dump. Content is lossily UTF-8 decoded and capped at
`TOOL_RESULT_MAX_BYTES / 2` bytes (leaving headroom for JSON structure and
lossy-decode replacement-character growth) rather than failing outright on
oversized content -- the result carries an explicit `truncated: bool`
rather than silently dropping the tail.

**UTF-8 char-boundary safety in search snippets.** The search implementation
performs `.find()` on the *lowercased* text and slices that same lowercased
text for the snippet (not the original), because `.to_lowercase()` can
change a character's UTF-8 byte length for some non-ASCII characters --
byte offsets found in the lowercased string are only valid against that
string. The snippet's start/end are additionally snapped to the nearest
valid char boundary (`floor_char_boundary`/`ceil_char_boundary`, a
stable-Rust reimplementation of the unstable `str` methods of the same
name) since the fixed-radius arithmetic around a match can otherwise land
mid-character and panic on slicing. This was caught and fixed during this
session's own review, not by CI -- worth flagging because it is exactly
the kind of latent bug (correct for ASCII test fixtures, panics on real
multi-byte clinical/i18n text) that a superficial pass would miss.

## Files changed

```text
crates/medscale-contracts/src/envelopes/mod.rs
  - Capability::AgentToolInvoke
  - RequestBody::AgentToolInvoke{run_id,kind,arguments}
  - ResponseBody::MedAgentToolInvocation{invocation, receipt: Option<...>}

crates/medscale-core/src/authority/medagent.rs
  - ReadContextArtifactArgs/SearchContextArtifactsArgs (typed args)
  - floor_char_boundary/ceil_char_boundary helpers
  - MedAgent::fetch_artifact_bytes/invoke_tool/append_turn/
    next_medagent_header/execute_read_context_artifact/
    execute_search_context_artifacts
  - require_artifact_in_context's #[allow(dead_code)] removed (real
    caller now exists)

crates/medscale-core/src/authority/facade.rs
  - 1 dispatch arm + capability_matches pair

crates/medscale-core/src/cli_session.rs
  - CliSession::medagent_tool_invoke

crates/medscale-cli/src/medagent.rs
  - MedAgentCmd::ToolInvoke

crates/medscale-core/tests/medagent_077.rs
  - Harness::source_record/start_run helpers,
    running_run_with_one_source setup helper
  - 6 new tests (see below)

specs/077-medagent-workbench/tasks.md
evidence/077-medagent-workbench/T077-06_IMPLEMENTATION.md (this file)
```

## Tests

`crates/medscale-core/tests/medagent_077.rs`, all through real
`CoreFacade::dispatch` against real `SourceRecord` objects (created via
`Capability::CreateSourceRecord`, mirroring `collaboration_076.rs`'s own
precedent for this exact helper):

1. `granted_read_context_artifact_executes_and_returns_content` -- full
   round trip: executes, returns the exact byte content, and the run's
   turn list shows exactly 3 turns in order
   (`prompt_submitted`/`tool_requested`/`tool_result`).
2. `ungranted_tool_kind_is_refused_before_execution` -- an identity
   granted only `SearchContextArtifacts` attempting `ReadContextArtifact`
   is refused with a reason and no receipt.
3. `granted_tool_refuses_artifact_outside_bound_context` -- a granted tool
   kind naming a *real, existing* `SourceRecord` that was never added to
   this run's `ContextManifest` is refused -- the T4 gate's core claim,
   proven end to end through the facade rather than only at the
   `require_artifact_in_context` unit-test level (T077-04).
4. `search_context_artifacts_finds_only_bound_content` -- bounded
   substring search returns the expected snippet.
5. `tool_invocation_refused_with_malformed_arguments` -- a payload missing
   the required `object_id` field is refused, not partially executed.
6. `tool_invocation_on_a_non_running_run_is_refused_closed` -- a `Pending`
   (never started) run refuses tool invocation with a hard `Conflict`,
   proving the run-state check fires before any turn/invocation is ever
   recorded.

No local compile/test run was possible (MSVC linker absent, same
constraint as every prior spec). `cargo fmt --check` is clean across the
whole workspace after this change.

## Exact-head CI qualification

Green on the first push, no fixes needed (the char-boundary bug above was
caught by this session's own review before pushing, not by CI):

```text
HEAD 9b92f1b -> SUCCESS, run 35717841248, 6/6.
```

T077-06 is qualified at exact head `9b92f1be6f39a7e4d3ef9ed4942242eeff2f42bb`.
