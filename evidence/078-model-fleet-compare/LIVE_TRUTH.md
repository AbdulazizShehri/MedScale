# LIVE_TRUTH — Spec 078 T078-00

**Status:** `T078-00 BASELINE (promotion branch, pre-implementation)`

## Canonical SHAs

```text
ORIGIN_MAIN_AT_PROMOTION = ff2e677294b1ea8704bbeefa605d129c1a99e8f2
PROMOTION_MERGE_PR134 = ff2e677 (merge of PR #134, Spec 077 closure bookkeeping)
BRANCH = spec/078-model-fleet-compare
MERGE_BASE_WITH_MAIN = ff2e677294b1ea8704bbeefa605d129c1a99e8f2
```

## Spec 077 closure re-verification (live, this session)

```text
PR133_STATE = MERGED, baseRefName=main
PR133_HEAD = 2ea886c71898890e152fc9d1924d41a10df8dd09
PR133_MERGE_SHA = ce8a40da88a78a0478039e5cfe8a0d4c5b058f34
PR133_EXACT_HEAD_CI = run 35735800476, conclusion success, 6/6
POST_MERGE_MAIN_CI_133 = run 35738904685, conclusion success, 6/6

PR134_STATE = MERGED, baseRefName=main
PR134_HEAD = 6517e3e0ddf885908f33ab3dbc57310f782a79ef
PR134_MERGE_SHA = ff2e677294b1ea8704bbeefa605d129c1a99e8f2
PR134_EXACT_HEAD_CI = run 35742088092, conclusion success, 6/6 (rust
  ubuntu-latest, rust windows-latest, rust macos-latest, perf
  delivery-plan scale (windows), cargo-deny, supply-chain policy present)
  -- re-verified live this session via
  `gh pr view 134 --repo TheHalfMoon/MedScale --json headRefOid,mergeCommit,statusCheckRollup`
  (an earlier restatement in this session's own prior continuation prompt
  cited a different, incorrect run id for this same PR; corrected here
  against the live source, not carried forward)
POST_MERGE_MAIN_CI_134 = run 35745220872, conclusion success, 6/6 --
  reconfirmed live in this session via
  `gh run view 35745220872 --repo TheHalfMoon/MedScale --json conclusion,headSha,jobs`
  immediately before this promotion began
```

All facts above were re-verified live via `gh pr view`/`gh run view`
against `TheHalfMoon/MedScale` in this session, matching the founder's
primary-directive requirement to reverify live truth before promoting the
next unit. Spec 077 is confirmed `CLOSED_CANONICAL`
(`evidence/077-medagent-workbench/CLOSURE.md`).

Honest residuals carried forward from Spec 077 (not hidden, not treated as
078 blockers -- 078 does not depend on rendered Desktop evidence or the
077 CLI-only identity/context-creation gap):

```text
RENDERED_DESKTOP_EVIDENCE = none exists for Spec 077 or earlier; no CI
  rendering step, no headless AppWindow test harness in this codebase
IDENTITY_CONTEXT_CREATION_CLI_ONLY = Spec 077's Desktop panel covers run
  list/detail/create+start/cancel/turns; agent identity registration and
  context-manifest creation remain CLI-only
T11_PROOF_STYLE = content-leakage / no-secret-logging proven by source-text
  inspection, not an automated log-capture test
```

## Numbering proof (verified on canonical base, this session)

```text
git ls-tree -r --name-only origin/main | grep -i 078 -> empty
specs/078-* on origin/main = absent
docs/planning/SPEC_078_PROMOTION.md on origin/main = absent
Spec 078 implementation code on origin/main = absent
gh pr list --repo TheHalfMoon/MedScale --search "078" --state all
  -> #124 (unrelated planning doc mentioning "078" in prose, open),
     #134 (Spec 077 closure PR, merged -- title does not mention 078,
     matched by body/commit text referencing "078+" deferred-work rows),
     #121/#123 (unrelated planning docs, merged)
  -> none implements Spec 078
```

## Dependency-graph confirmation (live docs, this session)

```text
RESEARCH_OS_EXECUTION_ROADMAP.md line 25 program dependency graph:
  "077 MedAgent Workbench --> 078 Model Fleet + Compare". Section "078 --
  Model Fleet + Compare" (lines 212-231): "## Depends on / 077." 078
  depends only on 077 (CLOSED_CANONICAL). Line 238 additionally confirms
  Spec 079 (Privacy Gate) depends on "074 + 077. 078 is not a hard
  prerequisite" -- so 079 is also technically dependency-ready, but 078 is
  the natural next-in-sequence candidate per this repository's established
  numeric-order promotion convention (074->075->076->077 in strict order).

RESEARCH_OS_V2_SPEC_IMPLEMENTATION_CONTRACTS.md line 292 "# 078 -- Model
  Fleet + Compare": "## Hard dependency / 077." Contracts list (AgentLane,
  LanePolicy, LaneTransform, FleetRun, FleetRunState, LaneRunRef,
  ComparisonRequest, ComparisonObservation, ComparisonReport) matches
  contracts.md. Closure gate: "At least two distinct admitted lanes run
  the same task, partial failure is explicit, and comparison preserves
  evidence/unknown semantics."

BUILD_QUEUE.md "078+" row (pre-promotion state): listed Model Fleet +
  Compare among DEFERRED_BY_CANONICAL_DESIGN candidates requiring fresh
  promotion. This packet performs that promotion and confirms 078 is the
  correct next number.

CONCLUSION: Spec 078 = Model Fleet + Compare is confirmed the correct next
  dependency-ready Research OS candidate by strict numeric-order
  convention. No unclosed hard prerequisite blocks it.
```

## Lane-plurality live inventory (T078-00, this session)

```text
grep -rln "PackRuntimeAdapter\|impl.*Runtime.*for" crates/medscale-pack/src
  -> runtime.rs (FixtureRuntime, Spec 008, stub, no real inference),
     onnx_runtime.rs (OnnxTokenClassifierRuntime, Spec 069, real
     tract_onnx-backed CPU inference)

find . -iname "*.onnx" (excluding target/)
  -> exactly one: evidence/069-real-local-model-runtime-hf-pack-path/
     fixtures/pack-tiny-token-classifier-v0/model.onnx

find . -iname "*manifest*.json" | grep -i pack (excluding target/)
  -> evidence/008-local-ai-capability-fabric/fixtures/pack-fixture-ner-v0/
     pack.manifest.json (FixtureRuntime-only, no ONNX artifact)
  -> evidence/026-pack-signer-os-sandbox/fixtures/pack-fixture-signed-v1/
     pack.manifest.json (signing fixture, not a real inference Pack)
  -> evidence/069-real-local-model-runtime-hf-pack-path/fixtures/
     pack-tiny-token-classifier-v0/pack.manifest.json (the one real,
     ONNX-backed, genuinely-inferential admitted Pack fixture in this
     repository)

CONCLUSION: exactly one real, qualified local model runtime/Pack exists in
  this repository as of this promotion. See
  docs/planning/SPEC_078_PROMOTION.md's "Lane-plurality scope decision"
  for how this promotion resolves the closure gate's "two distinct
  admitted lanes" requirement against this live fact.
```

## Live repository anchors inventoried (T078-00 inspection)

```text
contracts: crates/medscale-contracts/src/{lib,objects/{ids,authority_classes,
  source,scope},evidence,network,project_graph,envelopes,text,data_sources,
  collaboration,medagent,packs/mod}
core: crates/medscale-core/src/authority/{facade,project_graph,data_sources,
  data_acquire,collaboration,medagent,store}, crates/medscale-core/src/
  cli_session.rs, crates/medscale-core/src/process/session.rs
storage: crates/medscale-storage/src/{encrypted_vault,sqlite_meta,migrate,
  blob,sealed_blob,backup,project_graph,data_sources,collaboration,medagent}
storage schema version live = 6 (078 migration target: v6 -> v7)
pack runtime: crates/medscale-pack/src/{runtime,onnx_runtime,store,format,
  mesc_verify} -- PackRuntimeAdapter trait, FixtureRuntime (Spec 008),
  OnnxTokenClassifierRuntime (Spec 069, real tract_onnx-backed local token
  classifier), PackStore (admit/insert/list/get/current_pack_id/promote) --
  reused unmodified by 078, never called directly (only through Spec 077's
  AgentIdentity/AgentRun path)
medagent (Spec 077, reused unmodified): crates/medscale-contracts/src/
  medagent.rs (AgentIdentity, AgentCapabilityManifest, ContextManifest,
  AgentRun/AgentRunState/AgentTurn, ToolInvocation/ToolReceipt, RunReceipt,
  AgentProposal); crates/medscale-core/src/authority/medagent.rs (MedAgent
  struct: register_agent_identity, create_context_manifest,
  create_agent_run, start_agent_run, execute_agent_run, cancel_agent_run,
  complete_agent_run, fail_agent_run, get/list variants)
cli: crates/medscale-cli/src/{main,project,data_source,collaboration,medagent}
desktop: crates/medscale-desktop/src/{main,project_workspace,data_workbench,
  collaboration_workspace,medagent_workspace} + ui/{app,components,theme}.slint
dependency gate: scripts/check-dependency-direction.ps1 (CI runs before
  clippy/test)
CI: .github/workflows/ci.yml (rust x3, perf windows, cargo-deny,
  supply-chain policy)
reusable revision model: medscale_contracts::project_graph::{ProjectRevision,
  check_revision, initial_revision} (u64)
migration pattern confirmed: crates/medscale-storage/src/sqlite_meta.rs
  fn migrate() -- sequential
  "if journal.finished_version < N { begin_migration(N);
  execute_batch(module::VN_DDL); finish_migration(N); }" blocks;
  v5->v6 (medagent::V6_DDL) is the exact precedent 078 follows for v6->v7.
```

## Baseline gate state

```text
WORKSTATION_TOOLCHAIN = local Windows link.exe/cl.exe absent (no MSVC Build
  Tools installed; only Git's own non-MSVC link.exe on PATH). Local
  link/build impossible, matching the constraint Spec 075/076/077
  recorded. cargo fmt --check and cargo metadata remain available locally
  (rustc 1.97.1, cargo 1.97.1). GitHub Actions CI is the authoritative
  qualification path.
BASELINE_PRE_EXISTING_FAILURES = none observed on main CI (last confirmed
  main CI: run 35745220872 on ff2e677, success, 6/6); local build/test
  linkage unavailable (recorded, not suppressed).
```

## Promotion-commit exact-head CI qualification (T078-00 checkpoint)

```text
PROMOTION_COMMIT = 58c4cd8fa71741e32eb1197b8387692fd6946769
  ("docs: promote Spec 078 Model Fleet + Compare", PR #135, docs-only)
PROMOTION_COMMIT_CI = run 35750689882, conclusion success, 6/6
  (cargo-deny, supply-chain policy present, perf delivery-plan scale
  (windows), rust (ubuntu-latest), rust (macos-latest),
  rust (windows-latest)) -- confirmed live via
  `gh run view 35750689882 --json conclusion,headSha,jobs`, matching the
  head SHA of the promotion commit exactly.
```

Per this promotion's own "Promotion-mechanics precedent correction," T078-01
implementation now proceeds on this same branch/PR (`spec/078-model-fleet-
compare`, PR #135), continuing from this verified-green checkpoint rather
than merging a standalone promotion-only PR (which has no precedent in this
repository's actual history).

## Worktrees

```text
C:/Users/Shehr/work/MedScale-074 = queue/spec-074-final-bookkeeping (stale; merged)
C:/Users/Shehr/work/MedScale-075 = queue/mark-075-closed-canonical (stale; merged)
C:/Users/Shehr/work/MedScale-076 = queue/mark-076-closed-canonical (stale; merged)
C:/Users/Shehr/work/MedScale-077 = spec/078-model-fleet-compare (this work;
  same worktree directory reused across Spec 077's closure and Spec 078's
  promotion/implementation -- branch switched in place via
  `git checkout -b spec/078-model-fleet-compare origin/main`)
C:/Users/Shehr/IdeaProjects/MedScale = spec/073-final-ui-polish (unrelated,
  pre-existing uncommitted work from a different task; untouched by this
  session)
```
