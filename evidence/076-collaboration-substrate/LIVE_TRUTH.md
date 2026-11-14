# LIVE_TRUTH — Spec 076 T076-00

**Status:** `T076-00 BASELINE (promotion branch, pre-implementation)`

## Canonical SHAs

```text
ORIGIN_MAIN_AT_PROMOTION = 6021ff9aad397a8488087cae01e56528370e8211
PROMOTION_MERGE_PR130 = 6021ff9 (merge of PR #130)
PR130_HEAD = f0e3cd1f7b7dc9aaec0c6046a97e9517644db67b
BRANCH = spec/076-collaboration-substrate
MERGE_BASE_WITH_MAIN = 6021ff9aad397a8488087cae01e56528370e8211
```

## Spec 075 closure re-verification (live, this session)

```text
PR129_STATE = MERGED, baseRefName=main
PR129_HEAD = 9f84a6e74805d2c326cec40ab4703cc3fff0e069
PR129_MERGE_SHA = 89a88cfbbe7b67582886fb07fb6de4fd9ca28dff
PR129_MERGED_AT = 2026-09-21T09:18:41Z
PR129_EXACT_HEAD_CI = run 35580861670, conclusion success, jobs 6/6
  (cargo-deny, perf delivery-plan scale (windows), supply-chain policy present,
   rust (macos-latest), rust (windows-latest), rust (ubuntu-latest))
POST_MERGE_MAIN_CI_129 = run 35582548200 on 89a88cf, conclusion success, 6/6

PR130_STATE = MERGED, baseRefName=main
PR130_HEAD = f0e3cd1f7b7dc9aaec0c6046a97e9517644db67b
PR130_MERGE_SHA = 6021ff9aad397a8488087cae01e56528370e8211
PR130_MERGED_AT = 2026-09-21T10:13:11Z
PR130_EXACT_HEAD_CI = run 35585345326, conclusion success, 6/6
POST_MERGE_MAIN_CI_130 = run 35587568653 on 6021ff9, conclusion success, 6/6
```

All eight facts above were re-verified live via `gh pr view 129/130 --json ...` and `gh run view <id> --json jobs` against `TheHalfMoon/MedScale` in this session, matching the founder's directive exactly. Spec 075 is confirmed `CLOSED_CANONICAL`.

Honest residuals carried forward from Spec 075 (not hidden, not treated as 076 blockers — 076 does not depend on remote dataset live-fetch or the Desktop-rendering gap):

```text
REMOTE_DATASET_LIVE_FETCH = not exercised beyond fail-closed deny-before-socket
  (UreqTransport::send unconditionally returns ExternalGateRequired in this
  repository state; parsing/merge/hash-verification logic is unit-tested with
  synthetic bytes only)
RENDERED_DESKTOP_EVIDENCE = none exists for Spec 075 or earlier; no CI
  rendering step, no headless AppWindow test harness in this codebase
```

## Predecessor closure (Spec 074, for completeness)

```text
SPEC_074_STATE = CLOSED_CANONICAL
SPEC_074_MERGE = 3d59255d0f37800cdda85dd4a7f12238356b02ad (PR #122)
SPEC_074_EXACT_HEAD_CI = run 35308381108 (6/6)
SPEC_074_POST_MERGE_MAIN_CI = run 35309949710 (6/6)
SPEC_074_CLOSURE_DOC = evidence/074-project-artifact-graph-foundation/CLOSURE.md
```

## Numbering proof (verified on canonical base)

```text
specs/076-* on origin/main = absent
docs/planning/SPEC_076_PROMOTION.md on origin/main = absent
Spec 076 implementation code on origin/main = absent
  (git ls-tree -r --name-only origin/main | grep -i 076 -> empty)
Open PRs matching "076" = none (gh pr list --search 076 --state all -> only
  #130 "mark Spec 075 CLOSED_CANONICAL", #121, #123, none implementing 076)
```

## Dependency-graph confirmation (live docs, this session)

```text
RESEARCH_OS_EXECUTION_ROADMAP.md section 2 program dependency graph:
  "076 Collaboration Substrate" depends only on "074" (already PROMOTED
  separately and CLOSED_CANONICAL). 075 is not a hard dependency of 076.
  Roadmap states default preference: "prefer sequential promotion through 079."

RESEARCH_OS_V2_SPEC_IMPLEMENTATION_CONTRACTS.md section "076 — Collaboration
  Substrate": "Hard dependency: 074." Required contracts list matches
  contracts.md exactly (CollabEventEnvelope, Room, RoomMembership, ThreadRef,
  Message, MessageEdit, Task, TaskStatus, NoteDocument, NoteRevision,
  ApprovalRequest, ApprovalDecision, PresenceEvent, ParticipantIdentity,
  AgentParticipantIdentity, ActivityRecord, SyncCursor).

BUILD_QUEUE.md "076+" row: lists Collaboration Substrate among
  DEFERRED_BY_CANONICAL_DESIGN candidates requiring fresh promotion; states
  numbering "not assumed to be 076 by number" without verification -- this
  packet performs that verification and confirms 076 is correct.

Stale doc flagged (non-blocking): docs/planning/RESEARCH_OS_SPEC_IMPLEMENTATION_CONTRACTS.md
  (V1, pre-Amendment-001) numbers Collaboration Substrate as "075" and
  MedAgent Workbench as "076" -- superseded by Amendment 001 / V2 docs / live
  BUILD_QUEUE.md, which all agree on 076. Not corrected by this promotion;
  flagged so no future reader is misled.

CONCLUSION: Spec 076 = Collaboration Substrate is confirmed the correct next
  dependency-ready Research OS candidate. No unclosed hard prerequisite
  blocks it.
```

## Live repository anchors inventoried (T076-00 inspection)

```text
contracts: crates/medscale-contracts/src/{lib,objects/{ids,source,scope},
  evidence,network,project_graph,envelopes,text,data_sources}
core: crates/medscale-core/src/authority/{facade,project_graph,data_sources,
  data_acquire,store}, crates/medscale-core/src/process/session.rs
storage: crates/medscale-storage/src/{encrypted_vault,sqlite_meta,migrate,
  blob,sealed_blob,backup,project_graph,data_sources}
storage schema version live = 4 (076 migration target: v4 -> v5)
cli: crates/medscale-cli/src/{main,project}
desktop: crates/medscale-desktop/src/{main,project_workspace}
  + ui/{app,components,theme}.slint
dependency gate: scripts/check-dependency-direction.ps1 (CI runs before
  clippy/test)
CI: .github/workflows/ci.yml (rust x3, perf windows, cargo-deny,
  supply-chain policy)
reusable revision model: medscale_contracts::project_graph::{ProjectRevision,
  check_revision, initial_revision} (u64)
reusable anchor model: medscale_contracts::project_graph::{ArtifactDescriptor,
  ArtifactKind, ArtifactVersionBinding, ReferenceResolution}
reusable span model: medscale_contracts::text::{TextSpan, TextRepresentation,
  CoordinateSystem}
session/actor identity: medscale_core::process::session::SessionRegistry
  (holder_id: OpaqueId is the existing audit/session actor identity)
migration pattern confirmed: crates/medscale-storage/src/sqlite_meta.rs
  fn migrate() -- sequential
  "if journal.finished_version < N { begin_migration(N);
  execute_batch(module::VN_DDL); finish_migration(N); }" blocks;
  v3->v4 (data_sources::V4_DDL) is the exact precedent 076 follows for v4->v5.
```

## Baseline gate state

```text
WORKSTATION_TOOLCHAIN = local Windows link.exe/cl.exe absent (no MSVC Build
  Tools installed; only Git's own non-MSVC link.exe on PATH; no vswhere.exe).
  Local link/build impossible, matching the constraint Spec 075 recorded.
  cargo fmt --check and cargo metadata remain available locally
  (rustc 1.97.1, cargo 1.97.1). GitHub Actions CI is the authoritative
  qualification path.
BASELINE_PRE_EXISTING_FAILURES = none observed on main CI (last main CI:
  run 35587568653, success, 6/6); local build/test linkage unavailable
  (recorded, not suppressed).
```

## Worktrees

```text
C:/Users/Shehr/work/MedScale-074 = queue/spec-074-final-bookkeeping (stale; merged)
C:/Users/Shehr/work/MedScale-075 = queue/mark-075-closed-canonical (stale; merged)
C:/Users/Shehr/work/MedScale-076 = spec/076-collaboration-substrate (this work; clean at creation)
C:/Users/Shehr/IdeaProjects/MedScale = spec/073-final-ui-polish (unrelated,
  pre-existing uncommitted work from a different task; untouched by this session)
```
