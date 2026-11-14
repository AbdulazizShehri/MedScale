# CLOSURE — Spec 076 Collaboration Substrate

## Terminal truth

```text
SPEC_076_CLOSED_CANONICAL=true
MERGE_SHA=594c309a034bac1f2ba24b4e9d830dd6fbdbd857
POST_MERGE_MAIN_CI=35640113781 (6/6 success on the exact merge commit)
EXACT_HEAD_CI=35637687997 (6/6 success on head 10c99f4, PR #131)
MIGRATION_RECOVERY=PASS (storage suite incl. v4->v5 additive migration
  preserving pre-076 rows, repeated-open safety, backup/restore round-trip
  incl. activity hash-chain re-verification and hand-edited-backup
  rejection, half-committed-state atomicity; see collaboration_076.rs tests
  and migration.md)
CORE_CLI_DESKTOP_PARITY=PASS (one Core authority path; CLI/Desktop render
  typed Core results through CliSession only; dependency-direction gate
  holds; Desktop covers Rooms/Threads/Messages/Tasks, CLI covers all 8
  entity families including Notes/Approvals -- an honest, documented gap,
  not silently claimed)
SECURITY_ADVERSARIAL=PASS (T1..T13 mapped to tests in SECURITY_ADVERSARIAL.md,
  including one real finding the exact-range review surfaced and this
  closure fixed: restore_v5 never re-verified the activity hash chain after
  replay, so a hand-edited backup would have restored silently instead of
  failing closed, T10)
EXACT_RANGE_REVIEW=evidence/076-collaboration-substrate/EXACT_RANGE_REVIEW.md
  (OpenCodeReview delegation mode, per explicit instruction, no other review
  tool/method; 1 confirmed finding, fixed with a regression test in 04fa4cb)
DESKTOP_COVERAGE_RESIDUAL=no rendered PNG evidence exists (no local or CI
  rendering path); this closure's own in-module Core-session test
  (collab_workspace_flows_through_real_core_session) found and fixed a real
  functional bug before merge -- the Desktop operator was registered under
  the wrong holder id, so every real "Create Room" action would have failed
  closed with Unauthorized (see DESKTOP_QUALIFICATION.md)
SPEC_077_PLUS_IMPLEMENTATION_AUTHORIZED=false (no promotion exists;
  candidates 077+ remain planning-only pending fresh promotion)
RESEARCH_OS_COMPLETE=false
REAL_PHI_AUTHORIZED=false
RELEASE_READY=false (unchanged by this lane)
PRIVATE_DATA_READY=false (unchanged by this lane)
MULTI_CLIENT_RELEASE_READY=false (unchanged by this lane)
```

## What this closure establishes

MedScale has a durable, local-first Collaboration Substrate layered on top
of the existing encrypted vault: participant identities distinguishing
human/service/agent actors, Project-scoped Rooms with membership-gated
visibility independent of Core `Capability` checks, threads anchored to
exact artifact revisions with live (never write-time-cached)
`ReferenceResolution`, append-only messages with author-only edit/delete,
revision-guarded tasks, notes with an explicit conflict-copy path (never
silent overwrite or auto-merge) on concurrent stale writes, approval
requests with multi-assignee decisions and an optional blind-until-closed
filter, and a hash-chained activity log that detects tampering both on live
storage and after backup/restore. One authority path throughout: CLI and
Desktop reach every collaboration operation exclusively through Core, never
touching `collab_*` storage tables directly. No new dependency was
introduced. A full exact-range review using OpenCodeReview (delegation
mode, per explicit instruction) found and fixed a real gap in the restore
path's own hash-chain re-verification before merge; this spec's own new
tests separately found and fixed a real Desktop authorization bug and (at
implementation time) a backup/restore data-loss gap and an activity-log
atomicity gap -- see `EXACT_RANGE_REVIEW.md`, `DESKTOP_QUALIFICATION.md`,
and `tasks.md` for the complete finding-by-finding record.

## What this closure does NOT establish

Rendered Desktop visual evidence (no local or CI rendering path exists in
this repository). A Desktop UI surface for Notes or Approvals (CLI-only for
those two entity families in this closure). Hub network sync execution of
any kind (076 is explicitly local-first only; no sync/relay/transport code
was added). MedAgent Workbench, Model Fleet, Privacy Gate expansion,
Governed Browse, AudioFlow, Analytics/Cohort Builder, Clinical
Graph/Research Canvas, Hub, Compute, R Workspace, Community Extensions,
Research/Evidence Packs, Institutional Adapters, federation, real-PHI
readiness, product release readiness, or Research OS completion.

## Closure trail

```text
PR_131_MERGE=594c309a034bac1f2ba24b4e9d830dd6fbdbd857
POST_MERGE_MAIN_CI=35640113781 (6/6)
EXACT_HEAD_CI=35637687997 (6/6, head 10c99f4)
DESKTOP_SLINT_COMPILE_CI=35624477017 (6/6, head ebec119 -- the standalone
  Desktop-panel commit, proven independently before the rest of T076-11
  landed on top of it)
CI_ATTEMPT_1=35628104959 (FAILED on ubuntu/macos: clippy::doc_lazy_continuation
  -- a module doc-comment line began with "+ qualification sequence)",
  parsed as an unindented markdown list continuation)
FIX_1=61a3d72 (reworded the doc comment, no content change)
CI_ATTEMPT_2=35628606985 (FAILED on ubuntu/macos: E0499 double-mutable-borrow
  -- a nested h.source_record(...) call inside source_anchor(...) borrowed
  the Core test harness's &mut self while an outer h.call(...) already held it)
FIX_2=adead4e (bound the nested call to a local before use, both occurrences)
CI_ATTEMPT_3=35629013553 (FAILED on ubuntu/macos with a REAL functional bug,
  not a test-code defect: collab_workspace_flows_through_real_core_session
  failed "create room: Unauthorized" -- ensure_self_participant registered
  the Desktop operator under the hardcoded string "desktop-operator" instead
  of the session's actual bound holder id, so every real "Create Room"
  action would have failed the same way on a running Desktop build)
FIX_3=bf9907c (added CliSession::holder_id(), registered under it instead)
CI_ATTEMPT_4=35630430836 (PASSED 6/6, head bf9907c -- input to the
  OpenCodeReview exact-range review)
EXACT_RANGE_REVIEW=OpenCodeReview delegation mode (ocr delegate preview +
  ocr delegate rule against origin/main..HEAD, agent-executed reasoning
  against the resolved rule set, per explicit "no other review tool/method"
  instruction); found restore_v5 never re-verified the activity hash chain
  after replay (security.md T10 / migration.md section 11)
EXACT_RANGE_REVIEW_FIX=04fa4cb (verify_activity_chain now called for every
  restored room before restore_v5 returns Ok; regression test
  restore_rejects_hand_edited_backup_with_broken_activity_chain)
CI_ATTEMPT_5=35634828435 (PASSED 6/6, head 04fa4cb -- the reviewed head)
EVIDENCE_DOCS=10c99f4 (this evidence packet's own docs-only update,
  EXACT_RANGE_REVIEW.md + EXACT_HEAD_QUALIFICATION.md, qualified together
  on the exact head that was actually merged)
FINAL_EXACT_HEAD_CI=35637687997 (head 10c99f4, PASSED 6/6)
MERGE=594c309 (PR #131, merge commit, MERGEABLE/CLEAN)
FINAL_POST_MERGE_MAIN_CI=35640113781 (PASSED 6/6)
```

## Open external residuals (not repository-owned work)

- `LOCAL_WINDOWS_TOOLCHAIN_DESTRUCTION_2026_09_18` (`OPEN_WORKSTATION_ONLY`,
  `docs/planning/EXTERNAL_GATES.md`): local link/C builds remain impossible
  on this workstation. Every fix in this closure was verified by tracing
  exact call sites and error-mapping chains against the real code, then
  qualified for real on GitHub Actions CI -- never claimed as locally
  tested. CI remains the authoritative qualification path.
- No rendered Desktop screenshots and no headless `AppWindow` test harness
  exist in this codebase (confirmed at close, not merely assumed); tracked
  for whichever future spec adds Desktop UI-logic/rendering test
  infrastructure.
- Content-leakage / no-secret-logging (T11) is proven by source-text
  inspection, not a log-capture-based automated test.
- All other open gates are the standing ones in
  `docs/planning/EXTERNAL_GATES.md` (unchanged by this lane).
