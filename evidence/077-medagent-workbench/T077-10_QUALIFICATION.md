# T077-10 Qualification and closure — evidence

## Per-task exact-head CI qualification (already established)

Every T077-03 through T077-09 commit reached full green CI
(`cargo fmt --check`, dependency-direction, Clippy `-D warnings`,
workspace tests including this spec's own contract/storage/Core/CLI/
Desktop tests, cargo-deny, supply-chain policy, and the Windows/macOS/
Ubuntu native Desktop builds) at its own exact head before the next task
began. See each `T077-0N_IMPLEMENTATION.md`'s "Exact-head CI
qualification" section for the individual run ids. This satisfies
T077-10's "run format, dependency-direction, focused contract/storage/
Core/CLI/Desktop tests... Clippy... workspace tests, cargo-deny/
supply-chain gates" bullets cumulatively; T077-10 itself re-confirms this
on the final exact head below rather than repeating each check narrative.

## Migration/reopen/recovery, malformed/corrupt, context-boundary-leakage

Already covered by dedicated tests written during implementation, not
retrofitted at closure:

- Migration + repeated-open safety: `migration_v5_to_v6_preserves_pre_077_rows_and_adds_medagent_tables`,
  `repeated_open_and_repeated_migration_is_safe`
  (`crates/medscale-storage/tests/medagent_077.rs`, T077-02).
- Backup/restore recovery, including a hand-edited-backup tamper test:
  `backup_restore_roundtrips_medagent_rows_and_reverifies_run_receipt_consistency`,
  `restore_rejects_hand_edited_backup_with_orphaned_run_receipt`
  (same file, T077-02).
- Malformed/corrupt state: `verify_run_receipt_consistency_detects_orphaned_receipt_and_missing_receipt`
  (direct DB tamper, T077-02); `tool_invocation_refused_with_malformed_arguments`
  (`crates/medscale-core/tests/medagent_077.rs`, T077-06).
- Context-boundary leakage (`security.md` T4): `require_artifact_in_context_allows_named_denies_everything_else`,
  `context_manifest_is_scope_isolated` (inline tests,
  `crates/medscale-core/src/authority/medagent.rs`, T077-04);
  `granted_tool_refuses_artifact_outside_bound_context`
  (`crates/medscale-core/tests/medagent_077.rs`, T077-06) -- a granted
  tool kind naming a real, existing, but out-of-context artifact is
  refused end to end through the facade, not merely at the unit level.
- Reopen durability: `agent_identity_survives_vault_reopen`
  (T077-03, uses the two-process `CloseVault` precedent from
  `durable_restart_016.rs`).
- Cancellation race (CAS safety under concurrent terminal transitions):
  `cancellation_race_only_one_request_wins` (T077-05).

## Credential/log secret and content-leakage scan

Source-text inspection (this repository's established `T11` proof style
-- `LIVE_TRUTH.md`: "T11_PROOF_STYLE = content-leakage / no-secret-logging
proven by source-text inspection, not an automated log-capture test"):

```text
grep -rn "log::\|tracing::\|println!\|eprintln!\|dbg!" \
  crates/medscale-core/src/authority/medagent.rs \
  crates/medscale-storage/src/medagent.rs \
  crates/medscale-contracts/src/medagent.rs
  -> no matches
```

Zero logging calls anywhere in Core/storage/contracts MedAgent code. The
only `println!` calls in this spec's diff are in
`crates/medscale-cli/src/medagent.rs` (34 occurrences) -- the CLI's own
human-readable printers, which only ever print content the operator
directly requested (their own agent identity/run/turn/tool-invocation
records), the same established pattern as every other
`medscale-cli/src/*.rs` printer. `crates/medscale-desktop/src/
medagent_workspace.rs` has zero logging calls.

## Rendered Desktop evidence

Honest residual recorded, not fabricated: see
`evidence/077-medagent-workbench/T077-09_IMPLEMENTATION.md`'s "Rendered
Desktop evidence" section. No local Slint toolchain, no CI rendering
step, no headless `AppWindow` harness in this codebase -- inherited from
Spec 076, not introduced by this spec.

## Alibaba Open Code Review — exact-range review (the only accepted
## semantic/code reviewer, per the founder's standing directive)

**Tool:** `ocr` (`@alibaba-group/open-code-review`), installed locally at
`/c/Users/Shehr/bin/ocr`, `open-code-review v1.12.7 (85cecfe)
windows/amd64`, matching `npm ls -g` (`@alibaba-group/open-code-review@1.12.8`).

**Mode:** delegation mode, per this repository's own established
convention (`evidence/076-collaboration-substrate/EXACT_RANGE_REVIEW.md`):
`ocr delegate preview`/`ocr delegate rule` perform deterministic
file-selection and rule-resolution only (no LLM call, no API key); the
actual review reasoning is performed by the reviewing agent (Claude)
against OCR's resolved rule set. Cubic and CodeRabbit automation posted
unsolicited status checks on PR #133 during this work (visible in its
`statusCheckRollup`); neither was consulted, and neither counts as review
evidence here, per the founder's explicit directive.

### Invocation

```text
BASE = origin/main = 80aefaecc513b504b89a8a6cf06b2a0bbd5e49ea
FIRST REVIEWED HEAD = f1ba3d762c3049a34b8a2dce891d89e3d526c1e5 (T077-09's
  qualified head, before this review's own fixes)

ocr delegate preview --from origin/main --to HEAD
  -> 21 reviewable .rs files (.md/.slint excluded: unsupported_ext,
     OCR's own file-type gate -- matches Spec 076's precedent, where
     .slint was reviewed manually for the same reason)

ocr delegate rule <21 files>
  -> resolved rule groups: Obvious Typos, Ownership and Lifetime
     Correctness, Error Handling and Panics, Unsafe Code Boundaries,
     Concurrency and Shared State, Async and Cancellation Safety,
     Collections/Iterators/Performance, Type and API Design, Macros and
     Metaprogramming (n/a -- no macro_rules!/proc macro in this diff),
     Security-Sensitive Code
```

### Reviewed files (21, all `.rs`)

```text
crates/medscale-cli/src/main.rs
crates/medscale-cli/src/medagent.rs
crates/medscale-contracts/src/envelopes/mod.rs
crates/medscale-contracts/src/lib.rs
crates/medscale-contracts/src/medagent.rs
crates/medscale-contracts/src/objects/authority_classes.rs
crates/medscale-core/src/authority/facade.rs
crates/medscale-core/src/authority/medagent.rs
crates/medscale-core/src/authority/mod.rs
crates/medscale-core/src/cli_session.rs
crates/medscale-core/tests/medagent_077.rs
crates/medscale-desktop/src/main.rs
crates/medscale-desktop/src/medagent_workspace.rs
crates/medscale-storage/src/backup.rs
crates/medscale-storage/src/lib.rs
crates/medscale-storage/src/medagent.rs
crates/medscale-storage/src/sqlite_meta.rs
crates/medscale-storage/tests/collaboration_076.rs
crates/medscale-storage/tests/data_sources_075.rs
crates/medscale-storage/tests/medagent_077.rs
crates/medscale-storage/tests/project_graph_074.rs
```

### Findings (2, both disposed)

**Finding 1 -- CONFIRMED, fixed.** `crates/medscale-core/src/authority/
medagent.rs`: the `commit_terminal_run` doc comment (introduced in the
T077-08 refactor that extracted it from the original `cancel_agent_run`)
retained the *old* `cancel_agent_run` doc paragraph immediately above the
new, correct one, with no blank line separating them -- rustdoc/rustfmt
merge adjacent `///` blocks into one comment, so the stale paragraph
("No tool invocations exist yet at T077-05, so `tool_invocation_ids` is
always empty here...") rendered directly above the correct text, with
nothing marking it as dead. Factually wrong as of T077-08: the function
body threads real tool-invocation ids for all three terminal states.
Fixed by deleting the stale paragraph.

**Finding 2 -- PLAUSIBLE, fixed.** `crates/medscale-core/src/authority/
medagent.rs`, `execute_search_context_artifacts`: allocated a full
lowercase copy of every context artifact's raw bytes
(`String::from_utf8_lossy(&bytes).to_lowercase()`) before checking
`SEARCH_MAX_MATCHES` -- only the *output* was bounded (match count,
snippet radius), never the input work. `SourceRecord`/
`DerivedSourceArtifact.bytes` carries no size cap of its own (Spec 002/074;
out of this spec's authority to add one there), and a `ContextManifest`
may name up to `CONTEXT_MANIFEST_MAX_ARTIFACTS` (64) artifacts, so an
agent run granted `SearchContextArtifacts` could trigger unbounded
per-call scan work, repeatable every turn. Marked PLAUSIBLE (not
CONFIRMED) by the reviewer pending confirmation that no upstream
ingestion-time size cap exists elsewhere in the codebase -- verified
during fix: `crates/medscale-core/src/authority/source_ops.rs`'s
`create_source_record` takes an unbounded `Vec<u8>`, confirming the gap
is real. Fixed by adding `SEARCH_ARTIFACT_SCAN_MAX_BYTES` (256 KiB) and
truncating each artifact's raw bytes to that cap before the lossy-decode/
lowercase pass (mirrors `execute_read_context_artifact`'s existing
`READ_CONTENT_MAX_BYTES` truncation pattern exactly); a new regression
test (`search_context_artifacts_does_not_scan_past_the_size_cap`) proves
content placed past the cap is genuinely not found, not merely that the
constant exists.

**UTF-8 char-boundary fix (from this session's own earlier work, not a
new finding) -- independently re-verified sound** by the reviewer:
`text_lower.find`/slicing is performed entirely against `text_lower`
(never the original `text`), and `floor_char_boundary`/
`ceil_char_boundary` correctly bound the snippet-radius arithmetic to a
valid char boundary before slicing.

**Structural claims re-verified independently** (not merely re-cited from
earlier evidence):
1. `require_artifact_in_context` has exactly one production caller
   (`execute_read_context_artifact`) -- confirmed by the reviewer via
   direct source inspection.
2. `grep -n "authority::promote\|authority::amend\|contracts::actions\|super::promote\|super::amend\|::actions::" crates/medscale-core/src/authority/medagent.rs`
   returns zero matches -- confirmed by the reviewer re-running the exact
   command.

### Post-fix exact-head re-qualification

Both fixes landed in a single commit,
`90589fc6e64833d05d0888007d099df4d14aaf32` ("fix(077): apply Alibaba Open
Code Review findings from T077-10 exact-range review"), which changed
only `crates/medscale-core/src/authority/medagent.rs` (21 lines) and
`crates/medscale-core/tests/medagent_077.rs` (+47 lines, one new test) --
no other file in the 21-file reviewed set changed. Per the exact-head
rule, the prior review is stale the moment code moves. The reviewing
agent re-verified the live file state at this exact head directly (not
just the diff) and called `ReportFindings` again with an **empty array**:
Finding 1's stale paragraph confirmed fully removed; Finding 2's fix
confirmed sound (`bytes[..scan_len]` is a plain byte-slice truncation via
`.min()`, always in-bounds, no UTF-8-boundary concern since it operates
before `from_utf8_lossy`); the new regression test confirmed
non-tautological (a marker placed past the 262,144-byte cap would be
found without the fix and is not found with it). This session
additionally verified `cargo fmt --check` clean across the whole
workspace at this head.

```text
FIX HEAD = 90589fc6e64833d05d0888007d099df4d14aaf32
POST-FIX REVIEW RESULT = 0 findings (ReportFindings, empty array)
```

```text
POST-FIX EXACT-HEAD CI = run 35732678400, conclusion success, 6/6
  (supply-chain policy present, cargo-deny, perf delivery-plan scale
  (windows), rust (ubuntu-latest), rust (macos-latest),
  rust (windows-latest))
```

`90589fc6e64833d05d0888007d099df4d14aaf32` is qualified for merge:
`reviewed_head == merge_candidate_head`, review clean, CI green.
