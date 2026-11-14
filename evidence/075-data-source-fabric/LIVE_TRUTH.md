# LIVE_TRUTH — Spec 075 T075-00

**Status:** `T075-00 BASELINE (promotion branch, pre-implementation)`

## Canonical SHAs

```text
ORIGIN_MAIN_AT_PROMOTION = ae0441918296c2d1510a71061249c7e55e65d760
PROMOTION_MERGE_PR128 = ae04419 (merge of PR #128)
PR128_HEAD = 3deeb5aeaee4bf6a28186b8721a3d4582ec27f1b
PR128_BASE = 8db4ac089db2869da9ff6570ad7da91139987321
BRANCH = spec/075-data-source-fabric
MERGE_BASE_WITH_MAIN = ae0441918296c2d1510a71061249c7e55e65d760
```

## PR #128 qualification (pre-merge, verified live)

```text
PR128_STATE_AT_MERGE = OPEN, non-draft (marked ready for review), MERGEABLE, CLEAN
PR128_DIFF = 19 files, all under docs/planning/, 7559 insertions, 0 deletions
PR128_EXACT_HEAD_CI = run 35411823551, conclusion success (6/6 required jobs green)
PR128_REVIEWS = 0 reviews, 0 comments, 0 threads
MERGE_METHOD = normal merge commit (no squash, no rebase, no force-push)
```

## Post-merge main verification

```text
POST_MERGE_MAIN_CI = run 35413163411 on ae04419
POST_MERGE_MAIN_CI_RESULT = run 35413163411, conclusion success (6/6 required jobs green; verified 2026-09-19)
```

T075-00 gate: post-merge main CI verified green; promotion claims in `SPEC_075_PROMOTION.md` and `BUILD_QUEUE.md` are accurate as committed.

## Predecessor closure (Spec 074)

```text
SPEC_074_STATE = CLOSED_CANONICAL
SPEC_074_MERGE = 3d59255d0f37800cdda85dd4a7f12238356b02ad (PR #122)
SPEC_074_EXACT_HEAD_CI = run 35308381108 (6/6)
SPEC_074_POST_MERGE_MAIN_CI = run 35309949710 (6/6)
SPEC_074_CLOSURE_DOC = evidence/074-project-artifact-graph-foundation/CLOSURE.md
SPEC_075_IMPLEMENTATION_AUTHORIZED_BEFORE_THIS_DIRECTIVE = false
```

## Numbering proof (verified on canonical base)

```text
specs/075-* on origin/main = absent
docs/planning/SPEC_075_PROMOTION.md on origin/main = absent (only SPEC_075_PROMOTION_CANDIDATE_2026-09-19.md planning candidate)
Spec 075 implementation code on origin/main = absent (no data_sources modules; csv/parquet/arrow hits are `narrow` substrings only)
```

## Live repository anchors inventoried (T075-00 inspection)

```text
contracts: crates/medscale-contracts/src/{lib,objects/{ids,source,scope},evidence,network,project_graph,envelopes}
core: crates/medscale-core/src/authority/{facade,project_graph,source_ops,cli_session,store}
storage: crates/medscale-storage/src/{encrypted_vault,sqlite_meta,migrate,blob,sealed_blob,backup,project_graph}
storage schema version live = 3 (075 migration target: v3 -> v4)
network: crates/medscale-network/src/{allowlist,transport,adapters}
keys: crates/medscale-keys/src/{keystore,provider}
cli: crates/medscale-cli/src/{main,project}
desktop: crates/medscale-desktop/src/{main,project_workspace} + ui/{app,components,theme}.slint
dependency gate: scripts/check-dependency-direction.ps1 (CI runs before clippy/test)
CI: .github/workflows/ci.yml (rust x3, perf windows, cargo-deny, supply-chain policy)
credential reference type CredentialRef = absent (only authorization_token_id: Option<OpaqueId>)
```

## Baseline gate state

```text
MAIN_CI_AT_8db4ac0 = run 35323152103 success (pre-merge main, verified)
WORKSTATION_TOOLCHAIN = LOCAL_WINDOWS_TOOLCHAIN_DESTRUCTION_2026_09_18 OPEN_WORKSTATION_ONLY;
  local link.exe/C builds impossible; CI is the authoritative qualification path.
  Local cargo fmt/metadata checks available (cargo 1.97.1, rustfmt present).
BASELINE_PRE_EXISTING_FAILURES = none observed on main CI; local build/test linkage unavailable (recorded, not suppressed).
```

## Worktrees

```text
C:/Users/Shehr/work/MedScale-074 = queue/spec-074-final-bookkeeping (stale; content merged via PR #127)
C:/Users/Shehr/work/MedScale-075 = spec/075-data-source-fabric (this work; clean at creation)
C:/Users/Shehr/IdeaProjects/MedScale = spec/073-final-ui-polish (unrelated; untouched)
```
