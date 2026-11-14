# CLOSURE — Spec 083 Knowledge + Research Canvas

## Terminal truth

```text
SPEC_083_CLOSED_CANONICAL=true
MERGE_SHA=2892860e64ffed9ff9ae6387f681f1657730e19c (PR #145)
FINAL_HEAD=194840ce74bee64c033aa0da557e2086eab4b1a9
EXACT_HEAD_CI=36004914365 (6/6)
CODE_HEAD_CI=35997730588 (6/6 on 8a05ccf; ubuntu 832 passed / 0 failed /
  1 ignored, windows 829 passed / 0 failed / 1 ignored)
POST_MERGE_MAIN_CI=36014350193 (6/6 on 2892860)
BASE=f08f903 (PR #144 merge; post-main run 35997628344 6/6)
REVIEW_POLICY=FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22 (no external reviewer;
  deterministic scope record in EXACT_RANGE_REVIEW.md; security challenge in
  SECURITY.md)
RETRIEVAL=lexical-v1 only (integer scoring); VECTOR=NOT_ADMITTED (Q28/Q29)
DEPENDENCIES_ADDED=none
CONTRACTS=PASS (6)  STORAGE_MIGRATION_RECOVERY=PASS (8, v11->v12 additive;
  17 restore tamper cases)  CORE_AUTHORITY=PASS (6 + 3 unit)
CLI=PASS (1)  DESKTOP_VIEW_MODEL=PASS (1)
REAL_PHI_AUTHORIZED=false
RELEASE_READY=false
PRIVATE_DATA_READY=false
SPEC_084_IMPLEMENTATION_AUTHORIZED=false until its own promotion
```

## What this closure establishes

MedScale has one Core-owned, per-Project knowledge path:
- A rebuildable lexical index over the exact current revisions of Spec 075
  snapshots, Spec 080 web evidence, Spec 081 transcript revisions and Spec
  082 derived tables. Every chunk names its object, content digest, locator
  and character range; the chunk list is re-digested on every read.
- Freshness is computed on every request. Superseded sources are labelled
  and not served by default; archived sources are tombstoned and their text
  is never shown. There is no retrieval cache.
- Every retrieval that reaches a Project leaves a receipt, including
  refusals; "insufficient evidence" is a recorded outcome, not an empty
  list.
- A Research Canvas holds live references, never copies; the live view
  reports unsupported notes, contradictions, superseded and missing
  evidence; stale writers and forged spans are refused.
- The index and canvases are projections: nothing here writes or can
  cascade into a source object.

## Defects found and fixed during qualification

- A v12 restore silently skipped a family that was present but not an
  array, and accepted a v12 snapshot missing a knowledge family (`334cf6b`).
  Every family that is not an array is now refused, and v12 snapshots must
  carry all three knowledge families.
- A restored receipt could list hits that were not chunks of its index
  (`82869e4`, tests `64d0679`). Every hit must now be a chunk of its index,
  span for span.

## Honest residuals (non-blocking, recorded)

- Lexical only: no stemming, synonyms or semantic matching.
  "Insufficient evidence" means no lexical match among current spans.
- No literature library, PDF/OCR import or graph expansion (out of scope).
- Retrieval and index-build cost at the 100,000-chunk bound is unmeasured.
- Web evidence flagged as instruction-like is indexed as inert text; the
  hit does not repeat the flag.
- A corrupt Spec 082 derived table fails index build and status closed for
  the whole Project until repaired.
- Canvas editing is CLI-only; no rendered Desktop screenshot (no CI
  rendering step).
- Row integrity in unencrypted vaults is structural, not cryptographic.
- No local compile or test run completed on this workstation; GitHub
  Actions is the compiler of record.
