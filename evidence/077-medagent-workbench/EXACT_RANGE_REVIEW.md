# EXACT RANGE REVIEW — Spec 077 (T077-10)

## Binding

```text
RANGE_BASE = 80aefaecc513b504b89a8a6cf06b2a0bbd5e49ea (origin/main, unchanged
  throughout T077-01..T077-09)
FIRST_REVIEWED_HEAD = f1ba3d762c3049a34b8a2dce891d89e3d526c1e5 (T077-01..
  T077-09 complete; PR #133 exact-head CI green, run 35728751836, 6/6)
POST_FIX_HEAD = 90589fc6e64833d05d0888007d099df4d14aaf32 (this review's
  2 findings applied; the merge-candidate head)
REVIEW_METHOD = alibaba/open-code-review delegation mode (`ocr` v1.12.7,
  installed locally: `ocr delegate preview --from origin/main --to HEAD`
  for deterministic file selection, `ocr delegate rule <paths...>` for
  deterministic rule resolution; the actual review reasoning performed by
  a forked Claude session with full implementation context, per this
  tool's own documented "Delegation Mode: your coding agent runs the
  review using its own LLM" design). No other review tool used -- Cubic
  and CodeRabbit both auto-posted on PR #133 per their installed GitHub
  integrations; both are non-authoritative and were not consulted for
  this evidence.
```

## File selection (`ocr delegate preview --from origin/main --to HEAD`)

```text
21 of 41 changed files reviewable (.rs only; .md/.slint excluded by OCR's
own file-type gate, matching Spec 076's own precedent for this exact
tool):

crates/medscale-cli/src/main.rs [modified]
crates/medscale-cli/src/medagent.rs [added]
crates/medscale-contracts/src/envelopes/mod.rs [modified]
crates/medscale-contracts/src/lib.rs [modified]
crates/medscale-contracts/src/medagent.rs [added]
crates/medscale-contracts/src/objects/authority_classes.rs [modified]
crates/medscale-core/src/authority/facade.rs [modified]
crates/medscale-core/src/authority/medagent.rs [added]
crates/medscale-core/src/authority/mod.rs [modified]
crates/medscale-core/src/cli_session.rs [modified]
crates/medscale-core/tests/medagent_077.rs [added]
crates/medscale-desktop/src/main.rs [modified]
crates/medscale-desktop/src/medagent_workspace.rs [added]
crates/medscale-storage/src/backup.rs [modified]
crates/medscale-storage/src/lib.rs [modified]
crates/medscale-storage/src/medagent.rs [added]
crates/medscale-storage/src/sqlite_meta.rs [modified]
crates/medscale-storage/tests/collaboration_076.rs [modified]
crates/medscale-storage/tests/data_sources_075.rs [modified]
crates/medscale-storage/tests/medagent_077.rs [added]
crates/medscale-storage/tests/project_graph_074.rs [modified]

Excluded (unsupported_ext, per OCR's own gate): app.slint, and every
.md file under docs/planning/, evidence/077-medagent-workbench/,
specs/077-medagent-workbench/. app.slint was reviewed manually (see
"Manual .slint review" below), matching the exact precedent Spec 076's
own EXACT_RANGE_REVIEW.md recorded for the same tool limitation.
```

## Rule set (`ocr delegate rule <21 paths>`)

Standard Rust system ruleset: Obvious Typos, Ownership and Lifetime
Correctness, Error Handling and Panics, Unsafe Code Boundaries,
Concurrency and Shared State, Async and Cancellation Safety, Collections/
Iterators/Performance, Type and API Design, Macros and Metaprogramming
(diff defines no macro -- not applicable), Security-Sensitive Code
(path/SQL/command injection, secret logging, integer/byte-slicing/UTF-8
boundary overflow, crypto/auth soundness).

## Findings (2 confirmed material, both fixed before merge)

**1. CONFIRMED (stale documentation) --
`crates/medscale-core/src/authority/medagent.rs`, `commit_terminal_run`.**
The doc comment retained a stale, factually wrong paragraph inherited
from the pre-refactor `cancel_agent_run` implementation ("No tool
invocations exist yet at T077-05, so `tool_invocation_ids` is always
empty here"), merged with the correct replacement doc block directly
above it with no blank line separating them -- misleading to any reader,
since the function actually threads real tool-invocation ids into every
terminal `RunReceipt` for all three terminal states (T077-08's own fix).
Fixed by removing the stale paragraph.

**2. PLAUSIBLE, treated as material (unbounded per-call input work) --
`crates/medscale-core/src/authority/medagent.rs`,
`execute_search_context_artifacts`.** The function allocated a full
lowercase copy of every context artifact's raw bytes on every
`SearchContextArtifacts` call, with no bound on per-artifact input size --
only the *output* (match count, snippet radius) was bounded, never the
input work. A `ContextManifest` may name up to
`CONTEXT_MANIFEST_MAX_ARTIFACTS` (64) artifacts, and
`SourceRecord`/`DerivedSourceArtifact.bytes` carries no size cap from
Spec 002/074 (out of this spec's authority to add one there), so a
granted agent run could trigger materially unbounded per-call work by
repeatedly searching against large bound artifacts. Fixed by adding
`SEARCH_ARTIFACT_SCAN_MAX_BYTES` (256 KiB) applied as a plain byte-slice
cap (`&bytes[..scan_len]`, always in-bounds by construction, no
char-boundary concern since it slices `&[u8]` not `str`) before the
lossy-decode/lowercase/search work, matching `ReadContextArtifact`'s own
pre-existing `READ_CONTENT_MAX_BYTES` bound-the-input discipline.
Regression test:
`search_context_artifacts_does_not_scan_past_the_size_cap` (places a
real, findable marker past the cap in 300+ KiB of synthetic content and
asserts it is not found -- a test that would fail without the fix).

Both findings were fixed in commit `90589fc`. A second, independent
review pass by the same reviewing session against the exact post-fix
head (`90589fc`, diffed against the first-reviewed `f1ba3d7`) confirmed:
the stale paragraph is fully gone; the byte-slice cap is sound (no
off-by-one, no panic risk, correctly distinct from the pre-existing
`floor_char_boundary`/`ceil_char_boundary` machinery that guards the
downstream `str` slicing); the new test is a genuine regression test, not
a tautology; and the fix commit itself introduces no new issue under the
same rule categories. That second pass returned zero further findings.

## Manual `.slint` review

`crates/medscale-desktop/ui/app.slint` (+101/-1) cannot be selected by
OCR (excluded as an unsupported extension). Reviewed manually: brace
balance verified programmatically (807 open / 807 close across the whole
file, before and after the T077-09 addition), and every added block
(`AgentRunRowItem`/`AgentTurnRowItem` structs, root properties, the
"MedAgent" `NavItem`, the route-subtitle ternary branch, and the content
panel itself) was cross-checked line-by-line against the existing,
already-compiling Collaboration block's exact syntax patterns (same
component names, same `ui-action(string)` dispatch convention, same
`accessible-role`/`accessible-label` usage). No defect found; independent
confirmation came from `rust (windows-latest)`'s exact-head CI job, which
compiles this file through the real Slint compiler as part of the native
Desktop build and passed clean on the first push (`f1ba3d7`, run
35728751836).

## Verdicts

```text
AUTHORIZED_SCOPE_ONLY = true (only specs/077-medagent-workbench-owned
  paths + the one additive, promotion-authorized Spec 002 touch:
  ProducerKind::Agent(OpaqueId) in objects/authority_classes.rs)
UNEXPECTED_FILES = none (file list matches the union of every
  T077-0N_IMPLEMENTATION.md's own "Files changed" section)
NEW_DEPENDENCIES = none (no Cargo.toml/Cargo.lock delta in range;
  medscale-pack/OnnxTokenClassifierRuntime and rusqlite/serde_json were
  already qualified dependencies before this spec)
NETWORK_AUTHORITY_CHANGE = false (execute_agent_run's runtime is a local
  CPU ONNX inference call against a caller-supplied directory path;
  synthetic_only gate enforced; zero sockets/listeners/clients introduced)
REAL_PHI = false (synthetic fixtures/vaults/prompts throughout every
  T077-0N test; `grep -ilE "\bphi\b|patient.?name|real.?patient"` over
  every changed .rs file's *full contents* returns 7 matches, all
  inspected individually: pre-existing, unrelated lines this diff did
  not introduce (a fixture-UI PHI-boundary bail message in main.rs, a
  pre-existing set_patient_name Desktop call), and this spec's own
  gate-related comments/test names about the synthetic_only/
  ExternalGateRequired control itself (e.g. "real PHI through this
  runtime requires a later, explicit gate") -- no actual PHI literal or
  real-patient-data string anywhere in this diff)
MESC_CHANGE = false (no mesc paths touched)
NO_CALL_PATH_INTO_PROMOTE_AMEND_ACTIONS = true (grep for
  "authority::promote|authority::amend|contracts::actions|super::promote|
  super::amend|::actions::" in medagent.rs returns zero matches --
  independently re-verified in this review's second pass, not merely
  trusted from the implementation session's own claim)
CONTEXT_BOUNDARY_ENFORCED = true (require_artifact_in_context is the
  sole production path from invoke_tool's ReadContextArtifact branch to
  artifact content; independently re-verified in this review)
```

## Disposition

Both confirmed findings fixed before merge. Post-fix exact-head review
(`90589fc`) clean: 0 findings. Exact-head CI for `90589fc`: run
35732678400, conclusion `success`, 6/6 (supply-chain policy present,
cargo-deny, perf delivery-plan scale (windows), rust (ubuntu-latest),
rust (macos-latest), rust (windows-latest)).

`90589fc6e64833d05d0888007d099df4d14aaf32` is qualified for merge:
exact-head review clean, exact-head CI green, `reviewed_head ==
merge_candidate_head`.
