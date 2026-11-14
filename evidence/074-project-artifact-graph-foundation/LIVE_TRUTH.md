# LIVE TRUTH — Spec 074 Project + Artifact Graph Foundation (T074-00)

## Reconciliation record

```text
MAIN_SHA=ee8daef3a2782bbdbcb3324766a5d6b95c09fa09
BRANCH=spec/074-project-artifact-graph-foundation
PRE_MERGE_BRANCH_HEAD=384979087af9503856fd1da3bb36f067ec7ad3ee
PRE_MERGE_MERGE_BASE_SHA=a80c33307afc4577790282652e5b20911beb4bbe
POST_MERGE_LOCAL_HEAD=81d6e681b797d6628e6ca3b0405c9c6456a17452
POST_MERGE_MERGE_BASE_SHA=ee8daef3a2782bbdbcb3324766a5d6b95c09fa09
PR=122 (OPEN, MERGEABLE, mergeStateStatus=CLEAN)
V2_AMENDMENT_PR=123 (MERGED at ee8daef3a2782bbdbcb3324766a5d6b95c09fa09, mergedAt=2026-09-17T11:01:28Z)
OPEN_PR_STATE=only PR #122 open; no competing Spec 074 PR
SPEC_074_PROMOTION_PRESENT=true (docs/planning/SPEC_074_PROMOTION.md, PROMOTED_IMPLEMENTATION_AUTHORIZED, 2026-09-17)
REAL_PHI_USED=false
```

## How reconciliation was performed

- `git fetch origin --prune` from live GitHub truth on 2026-09-17.
- Verified `origin/main` = `ee8daef3a2782bbdbcb3324766a5d6b95c09fa09` (merge of PR #123).
- Verified `origin/spec/074-project-artifact-graph-foundation` = `384979087af9503856fd1da3bb36f067ec7ad3ee`.
- Verified pre-merge merge-base = `a80c33307afc4577790282652e5b20911beb4bbe`.
- Branch-side files since base (9 commits, docs only): `docs/planning/SPEC_074_PROMOTION.md`,
  `specs/074-project-artifact-graph-foundation/{spec,plan,tasks,contracts,migration,security,MUSE_EXECUTION_PLAN}.md`,
  `evidence/074-project-artifact-graph-foundation/README.md`.
- Main-side files since base (V2 amendment, docs only): 16 files under `docs/planning/`
  including `RESEARCH_OS_PROGRAM_AMENDMENT_001_DATA_EXTENSIONS.md`,
  `DATA_SOURCE_FABRIC_PLAN.md`, `R_WORKSPACE_PRODUCT_PLAN.md`,
  `COMMUNITY_EXTENSIONS_PRODUCT_PLAN.md`, `RESEARCH_OS_EXECUTION_ROADMAP.md`,
  `RESEARCH_OS_V2_DECISION_REGISTER.md`,
  `RESEARCH_OS_V2_IMPLEMENTATION_CONTRACT_ADDENDUM.md`,
  `RESEARCH_OS_V2_SPEC_IMPLEMENTATION_CONTRACTS.md`,
  `RESEARCH_OS_V2_REPOSITORY_MAP_ADDENDUM.md`,
  `RESEARCH_OS_V2_VERIFICATION_ADDENDUM.md`,
  `RESEARCH_OS_V2_GAP_CLOSURE_REVIEW.md`, plus updates to
  `SOURCE_ADOPTION_MATRIX.md`, `RESEARCH_OS_PLAN_INDEX.md`,
  `RESEARCH_OS_PLAN_STATUS.md`, `RESEARCH_OS_PACKET_VERSION.md`,
  `RESEARCH_OS_FINAL_PLANNING_ASSERTIONS.md`.
- Zero overlapping files between the two sides; merge is docs-only with no code impact.
- Integrated via normal merge (NO rebase, NO force-push):
  `git merge origin/main -m "merge: integrate Research OS V2 planning amendment into Spec 074 lane"`.
- Result: `81d6e681b797d6628e6ca3b0405c9c6456a17452`.
- Work performed in isolated worktree `C:/Users/Shehr/work/MedScale-074` so the
  founder's dirty `spec/073-final-ui-polish` working tree in
  `C:/Users/Shehr/IdeaProjects/MedScale` was left untouched.

## CI state (live GitHub truth, 2026-09-17)

```text
MAIN_CI_RUN_ID=35213509633 (workflow=ci, head=ee8daef, status=completed, conclusion=success)
MAIN_CI_JOBS=6/6 success: cargo-deny, rust ubuntu/macos/windows (incl. fmt, dep-direction, clippy, test, portable-package qualification), supply-chain policy present, perf delivery-plan scale (windows)
BRANCH_PRE_MERGE_CI_RUN_ID=35205196612 (head=3849790, status=completed, conclusion=success, 7/7 PR checks pass incl. CodeRabbit skip-as-draft)
POST_MERGE_HEAD_CI=PENDING (to be recorded in EXACT_HEAD_QUALIFICATION.md after push; pending is NOT pass)
BRANCH_PROTECTION=none configured on main (API 404); PR #122 mergeable=CLEAN
```

PR #122 checks at pre-merge head (all pass):
`cargo-deny`, `perf delivery-plan scale (windows)`, `rust (macos/ubuntu/windows-latest)`,
`supply-chain policy present`, `CodeRabbit (skipped: draft)`.

## Baseline tests (local, post-merge head 81d6e68)

Host: Windows 11, worktree `C:/Users/Shehr/work/MedScale-074`.
Toolchain: repository-pinned Rust 1.97.1 (CI); local cargo present.

```text
cargo fmt --all -- --check => PASS (exit 0)
./scripts/check-dependency-direction.ps1 => PASS (cli/desktop -> core -> {storage,fhir,network,pack} -> {contracts,keys})
cargo clippy --workspace --all-targets --locked -- -D warnings => NOT_RUN (covered by exact-head CI; pending is not pass)
cargo test --workspace --locked => NOT_RUN locally (covered by exact-head CI; pending is not pass)
cargo deny check --all-features => NOT_RUN locally (covered by exact-head CI; pending is not pass)
PREEXISTING_FAILURES=none observed (fmt + dep-direction pass; docs-only merge cannot change code behavior)
```

Full baseline (clippy/test/deny/portable) is bound to the pushed exact-head CI run,
not claimed here.

## Authority state

```text
SPEC_074_PROMOTION=PROMOTED_IMPLEMENTATION_AUTHORIZED (docs/planning/SPEC_074_PROMOTION.md)
V2_AMENDMENT=merged to main (PR #123); planning-only, does NOT reinterpret Spec 074 scope
CANONICAL_SEQUENCE=074,075,076,077,078,079,080,081,082,083,084,085,086,087,088,089,090,091,092
SPEC_074_IMPLEMENTATION_AUTHORIZED=true
SPEC_075_PLUS_IMPLEMENTATION_AUTHORIZED=false
REAL_PHI_AUTHORIZED=false
MESC_MUTATION_AUTHORIZED=false
FORBIDDEN_IN_074=database connectors, Kaggle import, Hugging Face datasets, Data Source Fabric, R/RStudio/Posit, Community Extensions, plugin/WASM runtime, extension marketplace/registry, MedScale Hub, collaboration, MedAgent, Analytics, AudioFlow, Browse, RAG, Compute, second ID/provenance/audit/patient/document/clinical-truth models, canonical payload copies, new runtime network authority
```

V2 planning reconciliation: `SPEC_074_SCOPE_CHANGED=FALSE`
(`RESEARCH_OS_V2_GAP_CLOSURE_REVIEW.md` section 18/20). PR #122 remains the dedicated
Spec 074 lane. 075+ remain candidate-only without separate promotion.

## Working tree

```text
WORKTREE=C:/Users/Shehr/work/MedScale-074
WORKTREE_STATUS_AT_MERGE=clean
DIRTY_SPEC073_TREE=C:/Users/Shehr/IdeaProjects/MedScale (branch spec/073-final-ui-polish, untouched, out of scope)
```

## Next

Push `81d6e68` (+ this file) to `origin/spec/074-project-artifact-graph-foundation`
with a normal push (no force), then record the exact-head CI run ID and result in
`EXACT_HEAD_QUALIFICATION.md`. Pending CI is not PASS.
