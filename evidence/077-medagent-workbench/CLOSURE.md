# CLOSURE — Spec 077 MedAgent Workbench

## Terminal truth

```text
SPEC_077_CLOSED_CANONICAL=true
MERGE_SHA=ce8a40da88a78a0478039e5cfe8a0d4c5b058f34
EXACT_HEAD_CI=35735800476 (6/6 success on final head 2ea886c, PR #133)
POST_MERGE_MAIN_CI=35738904685 (6/6 success on merge commit ce8a40d)
MIGRATION_RECOVERY=PASS (storage suite incl. v5->v6 additive migration
  preserving pre-077 rows with a 076 ParticipantKind::Agent participant,
  repeated-open safety, backup/restore round-trip incl. RunReceipt
  consistency re-verification and hand-edited-backup rejection,
  tool-invocation transaction atomicity; see medagent_077.rs storage +
  Core tests and migration.md)
CORE_CLI_DESKTOP_PARITY=PASS (one Core authority path throughout;
  CLI/Desktop reach every MedAgent operation exclusively through
  CliSession, never storage or the model runtime directly;
  dependency-direction gate holds; CLI covers the full vertical slice
  (identity/context/run/tool/execute), Desktop covers run list/detail/
  create+start/cancel/turns -- agent identity registration and
  context-manifest creation remain CLI-only, an honest, documented gap
  matching Spec 076's own Notes/Approvals precedent, not silently
  claimed as covered)
SECURITY_ADVERSARIAL=PASS (T1..T5 mapped to tests: T1 structurally proven
  by grep -- zero references to authority::promote/authority::amend/
  contracts::actions anywhere in medagent.rs, independently re-verified
  during the exact-range review, not merely asserted; T2 typed
  deny_unknown_fields argument parsing before any use; T3 no shell/SQL/
  Rust-expression execution path for model output; T4
  require_artifact_in_context is the sole read-boundary check, proven
  both at the unit level (T077-04) and end-to-end through the facade
  against a real-but-out-of-context artifact (T077-06); T5 AgentIdentity
  status + admitted-Pack-version match re-checked at both
  create_agent_run and start_agent_run, never cached, proven by a
  revoke-then-attempt test)
EXACT_RANGE_REVIEW=evidence/077-medagent-workbench/EXACT_RANGE_REVIEW.md
  (OpenCodeReview delegation mode, per the founder's explicit standing
  directive, no other review tool/method; 2 findings -- 1 confirmed
  stale-documentation, 1 plausible unbounded-per-call-input-work --
  both fixed with a regression test in 90589fc; post-fix re-review on
  the exact post-fix head returned 0 findings)
DESKTOP_COVERAGE_RESIDUAL=no rendered PNG/screenshot evidence exists (no
  local Slint toolchain on this workstation, no CI rendering step, no
  headless AppWindow test harness in this codebase -- an inherited
  residual from Spec 076, not introduced by this spec); the Desktop data
  path is instead proven by a real-CliSession in-module test
  (medagent_workspace_flows_through_real_core_session), and the Slint
  markup itself compiled clean through the real Slint compiler on
  rust (windows-latest)'s exact-head CI job on the first push
SPEC_078_PLUS_IMPLEMENTATION_AUTHORIZED=false (no promotion exists;
  candidates 078+ remain planning-only pending fresh promotion)
RESEARCH_OS_COMPLETE=false
REAL_PHI_AUTHORIZED=false
RELEASE_READY=false (unchanged by this lane)
PRIVATE_DATA_READY=false (unchanged by this lane)
MULTI_CLIENT_RELEASE_READY=false (unchanged by this lane)
```

## What this closure establishes

MedScale has a durable, local-first MedAgent Workbench layered on top of
the existing encrypted vault and Project/Pack architecture: agent
identities bound to exactly one admitted local model Pack (with
pack_version captured at registration time and re-verified live, never
cached, against the currently admitted Pack on every run-start),
immutable-once-set capability manifests restricting which typed tool
kinds an identity may ever request, explicit context manifests naming the
exact set of Project artifacts a run may read (with live, never-cached
`ReferenceResolution` recomputed on every read), a frozen five-state run
lifecycle (`Pending -> Running -> {Cancelled, Completed, Failed}`, plus
early `Pending -> Cancelled`) with append-only turns, typed tool
dispatch (`ReadContextArtifact`/`SearchContextArtifacts`) policy-checked
against both the capability manifest and the context boundary before any
execution, a real local zero-network model execution lane against the
Spec 069 ONNX token-classifier runtime producing evidence-only
`AgentProposal`s linked to the existing `Proposal`/`ProducerKind::Agent`
authority object, and `RunReceipt`s threading a run's real
tool-invocation history committed atomically with every terminal
transition. One authority path throughout: CLI and Desktop reach every
MedAgent operation exclusively through Core, never touching
`medagent_*` storage tables or the model runtime directly. No new
production dependency was introduced (`medscale-pack`'s
`OnnxTokenClassifierRuntime` and `rusqlite`/`serde_json` were already
qualified before this spec). A full exact-range review using
OpenCodeReview (delegation mode, per the founder's explicit standing
directive) found and fixed a real, material gap (unbounded per-call scan
work in the search tool) before merge, with a regression test proving
the fix; this spec's own tests separately found and fixed two other real
gaps during implementation itself, before any review pass: a stale
`RunReceipt.tool_invocation_ids` provenance gap in the original
`cancel_agent_run` (T077-08, fixed by threading real invocation ids
through a shared `commit_terminal_run` helper used by all three terminal
states), and a UTF-8 char-boundary panic risk in the search tool's
snippet extraction (T077-06, fixed with `floor_char_boundary`/
`ceil_char_boundary` helpers before any test or review caught it).

## Honest residuals (not hidden)

- No rendered Desktop screenshot evidence exists for this spec or any
  prior one -- no CI rendering step, no headless `AppWindow` test
  harness in this codebase.
- Agent identity registration and context-manifest creation are CLI-only;
  the Desktop panel's run-creation form takes their ids as plain text
  entry, matching Spec 076's own precedent for Notes/Approvals.
- Content-leakage/no-secret-logging proof (this repository's own T11
  style) is by source-text inspection (grep for `log::`/`tracing::`/
  `println!`/`eprintln!`/`dbg!` across every MedAgent Core/storage/
  contracts file), not an automated log-capture test.
- `RunReceipt`'s frozen contract has no timestamp/duration/resource-usage
  field; "resource/timing facts" from `plan.md`'s T077-08 work bullet are
  not separately tracked beyond what the frozen shape already carries
  (turn count, tool-invocation count) -- amending that shape was outside
  this promotion's authority.
- `execute_agent_run` does not reuse `PacksEvaluateLocal`'s prepared-model
  cache (a deliberate v1 simplification: a run's model execution happens
  once, not in a hot loop).

## Verification performed

```text
git rev-parse origin/main            -> ce8a40da88a78a0478039e5cfe8a0d4c5b058f34
git rev-parse HEAD (at PR #133 head) -> 2ea886c71898890e152fc9d1924d41a10df8dd09
gh pr view 133 --json state,mergeCommit -> MERGED, ce8a40da88a78a0478039e5cfe8a0d4c5b058f34
```

```text
gh run view 35738904685 --json status,conclusion,jobs
  -> status completed, conclusion success, 6/6 (rust ubuntu-latest,
     rust macos-latest, rust windows-latest, perf delivery-plan scale
     (windows), supply-chain policy present, cargo-deny)
```

Post-main CI confirmed green. `SPEC_077_CLOSED_CANONICAL=true` with real,
directly-observed evidence.
