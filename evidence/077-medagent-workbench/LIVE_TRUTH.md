# LIVE_TRUTH — Spec 077 T077-00

**Status:** `T077-00 BASELINE (promotion branch, pre-implementation)`

## Canonical SHAs

```text
ORIGIN_MAIN_AT_PROMOTION = 80aefaecc513b504b89a8a6cf06b2a0bbd5e49ea
PROMOTION_MERGE_PR132 = 80aefae (merge of PR #132)
PR132_HEAD = 1dac0600be35b2f4059953fc762f43f65cfe7b5c
BRANCH = spec/077-medagent-workbench
MERGE_BASE_WITH_MAIN = 80aefaecc513b504b89a8a6cf06b2a0bbd5e49ea
```

## Spec 076 closure re-verification (live, this session)

```text
PR131_STATE = MERGED, baseRefName=main
PR131_HEAD = 10c99f40527ade420fdb99f59dfb207e15ead0e8
PR131_MERGE_SHA = 594c309a034bac1f2ba24b4e9d830dd6fbdbd857
PR131_MERGED_AT = 2026-09-21T18:42:23Z
PR131_EXACT_HEAD_CI = run 35637687997, conclusion success, jobs 6/6
  (cargo-deny, perf delivery-plan scale (windows), supply-chain policy present,
   rust (macos-latest), rust (windows-latest), rust (ubuntu-latest))
POST_MERGE_MAIN_CI_131 = run 35640113781 on 594c309, conclusion success, 6/6

PR132_STATE = MERGED, baseRefName=main
PR132_HEAD = 1dac0600be35b2f4059953fc762f43f65cfe7b5c
PR132_MERGE_SHA = 80aefaecc513b504b89a8a6cf06b2a0bbd5e49ea
PR132_MERGED_AT = 2026-09-21T19:25:18Z
PR132_EXACT_HEAD_CI = run 35642851086, conclusion success, 6/6
POST_MERGE_MAIN_CI_132 = run 35644705497 on 80aefae, IN PROGRESS at the time
  this document was first written (docs-only change; PR131's implementation
  CI is the load-bearing proof that Spec 076's actual code is sound -- this
  trailing run only re-verifies that the closure-bookkeeping commit itself
  introduces no regression, which a docs-only diff cannot do to compiled
  code). Updated below once observed complete, not left silently stale.
```

All facts above were re-verified live via `gh pr view 131/132 --json ...`
and `gh run view <id> --json jobs` against `TheHalfMoon/MedScale` in this
session, matching the founder's primary-directive requirement to reverify
live truth before promoting the next unit. Spec 076 is confirmed
`CLOSED_CANONICAL`.

Honest residuals carried forward from Spec 076 (not hidden, not treated as
077 blockers -- 077 does not depend on rendered Desktop evidence, the
Notes/Approvals Desktop gap, or the T11 inspection-only proof style):

```text
RENDERED_DESKTOP_EVIDENCE = none exists for Spec 076 or earlier; no CI
  rendering step, no headless AppWindow test harness in this codebase
NOTES_APPROVALS_DESKTOP_GAP = Spec 076's Desktop panel covers Rooms/
  Threads/Messages/Tasks only; Notes/Approvals are CLI-only
T11_PROOF_STYLE = content-leakage / no-secret-logging proven by source-text
  inspection, not an automated log-capture test
```

## Predecessor closures (074, 075, for completeness)

```text
SPEC_074_STATE = CLOSED_CANONICAL
SPEC_074_MERGE = 3d59255d0f37800cdda85dd4a7f12238356b02ad (PR #122)
SPEC_074_CLOSURE_DOC = evidence/074-project-artifact-graph-foundation/CLOSURE.md

SPEC_075_STATE = CLOSED_CANONICAL
SPEC_075_MERGE = 89a88cfbbe7b67582886fb07fb6de4fd9ca28dff (PR #129)
SPEC_075_CLOSURE_DOC = evidence/075-data-source-fabric/CLOSURE.md
```

## Numbering proof (verified on canonical base)

```text
specs/077-* on origin/main = absent
docs/planning/SPEC_077_PROMOTION.md on origin/main = absent
Spec 077 implementation code on origin/main = absent
  (git ls-tree -r --name-only origin/main | grep -i 077 -> empty)
Open PRs matching "077" = none implementing 077 (gh pr list --search 077
  --state all -> #124/#121/#123, all pre-existing planning docs that
  happen to mention "077" in prose, none implementing it)
```

## Dependency-graph confirmation (live docs, this session)

```text
RESEARCH_OS_EXECUTION_ROADMAP.md section 2 program dependency graph:
  "074 --> 077 MedAgent Workbench --> 078 Model Fleet + Compare". 077
  depends only on 074 (already PROMOTED separately and CLOSED_CANONICAL).
  075 and 076 are not hard dependencies of 077, though the roadmap notes
  "Integration with 076 participant/activity semantics should be bound
  before final closure if live contracts require it" -- 076 is now also
  CLOSED_CANONICAL, so this integration is available, not merely planned.

RESEARCH_OS_V2_SPEC_IMPLEMENTATION_CONTRACTS.md section "077 — MedAgent
  Workbench": "Hard dependency: 074." Required contracts list
  (AgentIdentity, AgentProfile, AgentCapabilityManifest, ContextManifest,
  AgentRun, AgentRunState, AgentTurn, ToolManifest, ToolInvocation,
  ToolReceipt, RunReceipt, AgentProposal) matches contracts.md.

BUILD_QUEUE.md "077+" row (pre-promotion state): listed MedAgent Workbench
  among DEFERRED_BY_CANONICAL_DESIGN candidates requiring fresh promotion.
  This packet performs that promotion and confirms 077 is the correct next
  number.

CONCLUSION: Spec 077 = MedAgent Workbench is confirmed the correct next
  dependency-ready Research OS candidate. No unclosed hard prerequisite
  blocks it.
```

## Live repository anchors inventoried (T077-00 inspection)

```text
contracts: crates/medscale-contracts/src/{lib,objects/{ids,authority_classes,
  source,scope},evidence,network,project_graph,envelopes,text,data_sources,
  collaboration,packs/mod}
core: crates/medscale-core/src/authority/{facade,project_graph,data_sources,
  data_acquire,collaboration,store}, crates/medscale-core/src/cli_session.rs,
  crates/medscale-core/src/process/session.rs
storage: crates/medscale-storage/src/{encrypted_vault,sqlite_meta,migrate,
  blob,sealed_blob,backup,project_graph,data_sources,collaboration}
storage schema version live = 5 (077 migration target: v5 -> v6)
pack runtime: crates/medscale-pack/src/{runtime,onnx_runtime,store,format,
  mesc_verify} -- PackRuntimeAdapter trait, FixtureRuntime (Spec 008),
  OnnxTokenClassifierRuntime (Spec 069, real tract_onnx-backed local token
  classifier, run(pack_dir, pack, input) -> OnnxRuntimeEvaluation), PackStore
  (admit/insert/list/get/current_pack_id/promote)
cli: crates/medscale-cli/src/{main,project,data_source,collaboration}
desktop: crates/medscale-desktop/src/{main,project_workspace,data_workbench,
  collaboration_workspace} + ui/{app,components,theme}.slint
dependency gate: scripts/check-dependency-direction.ps1 (CI runs before
  clippy/test)
CI: .github/workflows/ci.yml (rust x3, perf windows, cargo-deny,
  supply-chain policy)
reusable revision model: medscale_contracts::project_graph::{ProjectRevision,
  check_revision, initial_revision} (u64)
reusable anchor model: medscale_contracts::project_graph::{ArtifactDescriptor,
  ArtifactKind, ArtifactVersionBinding, ReferenceResolution}
reusable proposal model: medscale_contracts::objects::authority_classes::
  {Proposal, ProducerKind, ClinicalAssertion} -- ProducerKind currently
  {Human, Rule, WorkerStub, Other(String)}; 077 adds Agent(OpaqueId)
  additively (contracts.md section 7)
Spec 076 integration point: medscale_contracts::collaboration::
  {ParticipantKind::Agent, AgentParticipantIdentity.agent_profile_ref}
  (currently an unresolved Option<OpaqueId>; 077 provides a real
  AgentIdentity id it can point at)
session/actor identity: medscale_core::process::session::SessionRegistry
  (holder_id: OpaqueId is the existing audit/session actor identity);
  CliSession::holder_id() (added during Spec 076 T076-11) exposes it
migration pattern confirmed: crates/medscale-storage/src/sqlite_meta.rs
  fn migrate() -- sequential
  "if journal.finished_version < N { begin_migration(N);
  execute_batch(module::VN_DDL); finish_migration(N); }" blocks;
  v4->v5 (collaboration::V5_DDL) is the exact precedent 077 follows for
  v5->v6.
```

## Baseline gate state

```text
WORKSTATION_TOOLCHAIN = local Windows link.exe/cl.exe absent (no MSVC Build
  Tools installed; only Git's own non-MSVC link.exe on PATH). Local
  link/build impossible, matching the constraint Spec 075/076 recorded.
  cargo fmt --check and cargo metadata remain available locally
  (rustc 1.97.1, cargo 1.97.1). GitHub Actions CI is the authoritative
  qualification path.
BASELINE_PRE_EXISTING_FAILURES = none observed on main CI (last confirmed
  main CI: run 35640113781 on 594c309, success, 6/6; run 35644705497 on
  80aefae in progress at write time, docs-only); local build/test linkage
  unavailable (recorded, not suppressed).
```

## Worktrees

```text
C:/Users/Shehr/work/MedScale-074 = queue/spec-074-final-bookkeeping (stale; merged)
C:/Users/Shehr/work/MedScale-075 = queue/mark-075-closed-canonical (stale; merged)
C:/Users/Shehr/work/MedScale-076 = queue/mark-076-closed-canonical (stale; merged)
C:/Users/Shehr/work/MedScale-077 = spec/077-medagent-workbench (this work; clean at creation)
C:/Users/Shehr/IdeaProjects/MedScale = spec/073-final-ui-polish (unrelated,
  pre-existing uncommitted work from a different task; untouched by this session)
```
