# EXACT_RANGE_REVIEW — Spec 076

## Canonical base and branch

```text
BASE = 6021ff9aad397a8488087cae01e56528370e8211 (origin/main)
BRANCH = spec/076-collaboration-substrate
HEAD (at review time) = f124973ff0e7344720d2a5225dd55e5ccbd5163
HEAD (after review fix) = 04fa4cbab26aa617fdd6f1d7537f8a0b56afebcf
```

## Authorized scope

`docs/planning/SPEC_076_PROMOTION.md` (only Spec 076: Collaboration
Substrate; 077+ explicitly not authorized).

## Changed files (`git diff --name-status origin/main...HEAD`)

Product code:

```text
crates/medscale-cli/src/collaboration.rs (new)
crates/medscale-cli/src/main.rs (wiring)
crates/medscale-contracts/src/collaboration.rs (new)
crates/medscale-contracts/src/envelopes/mod.rs (~28 capabilities, ~33 requests, ~19 responses)
crates/medscale-contracts/src/lib.rs (module registration)
crates/medscale-core/src/authority/collaboration.rs (new: authority paths)
crates/medscale-core/src/authority/facade.rs (collab() helper + dispatch arms)
crates/medscale-core/src/authority/mod.rs (module registration)
crates/medscale-core/src/cli_session.rs (~30 collab_* wrappers + holder_id() accessor)
crates/medscale-core/tests/collaboration_076.rs (new)
crates/medscale-desktop/src/collaboration_workspace.rs (new: view-models + tests)
crates/medscale-desktop/src/main.rs (Collaboration route wiring + actions)
crates/medscale-desktop/ui/app.slint (Collaboration structs/properties/nav/route)
crates/medscale-storage/src/backup.rs (restore_v5, manifest v5)
crates/medscale-storage/src/collaboration.rs (new: V5_DDL + full CRUD)
crates/medscale-storage/src/lib.rs (module registration)
crates/medscale-storage/src/sqlite_meta.rs (v5 migration + snapshot_bytes v5)
crates/medscale-storage/tests/collaboration_076.rs (new)
crates/medscale-storage/tests/data_sources_075.rs (forward-fix: finished_version 4 -> 5)
crates/medscale-storage/tests/project_graph_074.rs (forward-fix: finished_version 4 -> 5)
```

Planning/spec/evidence (authorized promotion + package + evidence):

```text
docs/planning/SPEC_076_PROMOTION.md (new)
docs/planning/BUILD_QUEUE.md (076 row + footer)
specs/076-collaboration-substrate/ (6 files)
evidence/076-collaboration-substrate/ (this packet)
```

## Unexpected files

```text
NONE. git diff --name-status origin/main...HEAD (recorded above) matches
the union of the two lists exactly.
```

## New dependencies

```text
PRODUCT DEPENDENCIES ADDED: NONE.
- No new [dependencies] in any crate Cargo.toml (verified: no Cargo.toml
  or Cargo.lock diff appears in the changed-file list above).
- 076 reuses sha2 (checkpoint_digest), rusqlite, serde_json -- all already
  qualified by prior specs. cargo-deny + supply-chain-policy CI jobs green
  on exact head (see EXACT_HEAD_QUALIFICATION.md).
```

## Secret scan

```text
- No credential/token/private-key literal in any new code.
- CLI human/JSON output prints only collaboration content the operator
  directly requested (room names, message bodies, etc.), never a secret --
  same established pattern as every other medscale-cli/src/*.rs printer.
- No log::/tracing:: call anywhere in the 076 code paths (T11, verified by
  inspection; see SECURITY_ADVERSARIAL.md).
- Test fixtures are synthetic only (no PHI, no licensed content).
```

## Review method (per explicit instruction: OpenCodeReview, no others)

Performed with the OpenCodeReview CLI (`ocr`, `@alibaba-group/open-code-review`,
already installed locally) in **delegation mode**: no separate LLM was
configured for `ocr` itself, so OCR's deterministic file-selection and
rule-resolution commands (`ocr delegate preview`, `ocr delegate rule`) were
run against the full `origin/main..HEAD` range, and the reviewing agent
(Claude, via a forked session with full implementation context) performed
the actual review reasoning against OCR's resolved rule set, per OCR's own
documented "Delegation Mode: your coding agent runs the review using its own
LLM; no OCR API key required" design.

`ocr delegate preview --from origin/main --to HEAD` selected 19 of 30
changed files as reviewable (`.rs` files); `.md` and `.slint` files were
excluded as unsupported extensions by OCR's own file-type gate.
`crates/medscale-desktop/ui/app.slint` was reviewed manually for the same
reason (brace balance, property/callback wiring against
`crates/medscale-desktop/src/main.rs`'s `ui-action` dispatch) since OCR
cannot select it; no defect was found there beyond what exact-head CI
(`rust (windows-latest)` Slint compile) already proves.

`ocr delegate rule` resolved the standard Rust rule set (ownership/lifetime
correctness, error handling/panics, unsafe boundaries, concurrency/shared
state, collections/performance, type/API design, and — most relevant here —
Security-Sensitive Code: validate input before use, no string-built SQL, no
secret logging, integer/byte-slicing overflow checks, sound crypto/auth).
The reviewer applied these against every file's actual diff
(`git diff origin/main...HEAD -- <file>`), cross-checked against 076's own
frozen contracts (`security.md` T1-T13, `migration.md` section 5's
transaction-boundary requirements).

## Finding (1 confirmed, fixed before merge)

**`restore_v5` never re-verified the activity hash chain after replay
(security.md T10 / migration.md section 11).** `crates/medscale-storage/src/backup.rs`'s
`restore_v5` restored every `collab_activity_records` row via
`restore_activity_record_row` (which preserves `checkpoint_digest` verbatim,
never recomputing it) but never called `verify_activity_chain` afterward. A
hand-edited backup — one whose editor also recomputes the outer
`metadata_snapshot_digest` to match their edit, exactly the T10 threat
(`security.md`: "restore of a hand-edited backup"), not mere random
corruption the outer digest alone would already catch — would restore
silently instead of failing closed, contradicting `migration.md` section
11's explicit requirement that "a restore that produces a broken hash chain
is a corruption signal, not a silently accepted state."

Fixed in commit `04fa4cb`: `restore_v5` now collects every distinct
`room_id` seen while restoring activity records and calls
`verify_activity_chain` on each before returning `Ok`. Regression:
`restore_rejects_hand_edited_backup_with_broken_activity_chain`
(`crates/medscale-storage/tests/collaboration_076.rs`) — backs up a real
vault, hand-edits one activity record's `target_object_id` in the raw
snapshot file, recomputes the outer manifest digest to match (simulating a
real hand editor), and asserts `restore_vault` now fails closed.

No other confirmed findings survived verification. The reviewer's stated
confidence: high for logic/authority/transaction-boundary correctness
(every file read in full or in its security-relevant sections), moderate
for exhaustive line-by-line Rust-idiom nitpicks across the ~2,874-line
storage file, since delegation mode's value is OCR's deterministic file
selection/rule resolution while reasoning depth is bounded by what one
reviewer read in one pass.

## Prior self-caught findings (before this review, not hidden)

Two issues were found and fixed by this spec's own test suite before the
OpenCodeReview pass, recorded here for completeness (both already detailed
in `tasks.md`/`DESKTOP_QUALIFICATION.md`):

1. **Desktop operator registered under the wrong holder id.**
   `ensure_self_participant` registered under the hardcoded string
   `"desktop-operator"` instead of the session's real bound actor id
   (`"cli-holder"` for the production `CliSession::connect("desktop-projects")`),
   so every real "Create Room" action would have failed closed with
   `Unauthorized`. Caught by the new in-module test
   `collab_workspace_flows_through_real_core_session` on its first run.
   Fixed by adding `CliSession::holder_id()`.
2. **Backup/restore and activity-atomicity gaps found proactively during
   implementation** (T076-02, T076-03) — the synthetic-vault backup
   mechanism would have silently dropped all 076 data on restore, and the
   primary-row-write/`ActivityRecord`-append pair was originally two
   separate transactions. Both fixed within their originating commits,
   before any test or review caught them; documented in `tasks.md`.

### Local verification constraint (unchanged from prior specs)

This machine's MSVC toolchain is incomplete, so nothing in this PR has been
compiled or run locally end-to-end. `cargo fmt --check` (which works
locally without linking) is clean at every commit. Real, multi-platform
verification is exact-head GitHub Actions CI — see
`EXACT_HEAD_QUALIFICATION.md`.
