# Spec 071 Exact-Range Review

## Reviewed range

- Base: `129b63d0d93e8fc14fa0dcd786d95b1ac4697e68`
- Reviewed implementation head: `9f63fcc6da69b3ce063035c88687c8cb241af7bc`
- OpenCodeReview: `v1.12.3`
- Delegate preview: 4 reviewable files / 17 total files, 961 insertions / 54 deletions.
- Full OCR LLM review was attempted but no LLM endpoint is configured on the authorized host. No AI-review result is claimed. Delegate rules plus complete manual review were used instead.

## Coverage

Manual review covered all Rust, Slint, JSON, planning, specification, and evidence changes in the exact range. The Rust review used the resolved OpenCodeReview ownership/lifetime, error-handling, concurrency, security, API-design, and performance rules. Unsupported Slint/Markdown files were reviewed directly.

The 39-row `CLAIM_LEDGER.json` was compared programmatically with `docs/matrices/openmed-parity-matrix-v2.2.0.json`: row count, unique capability IDs, capability set, and all classifications match exactly. No ledger row has a parity claim, surpass claim, or BenchmarkManifest-present claim. Waiver presence is consistent with `WAIVED` state.

The OpenMed comparator was independently reverified live at tag `v2.2.0`, commit `59d9cb0a2e0ccbba8fa3d891a66d83ffaf45e837`, tree `1c949e35b2b8f2ea69da4284b370074fc4bf84ab`. `models.jsonl` contains 2266 rows at that pin, and all six comparator document SHA-256 values match `BASELINE_VERIFICATION.md`.

## Material findings and convergence

1. Row-count-only claim-ledger validation could accept 39 duplicated capability rows. The implementation now requires non-empty unique capability IDs, exact top-level baseline commit/tree, exact row-level baseline commit, allowed claim states, and false parity/surpass/BenchmarkManifest flags. A duplicate-capability regression proves fail-closed behavior.
2. The historical Spec 068 product-truth regression still asserted that source text contained `PROVEN ADVANTAGE` and `OPENMED AHEAD`. After Spec 071 those assertions could pass accidentally because the forbidden strings existed inside negative tests. The guard now requires the new `UNMEASURED`, `STRUCTURAL ONLY`, and `ANTI-METRIC` verdict assignments and explicitly rejects the old verdict assignments. Focused Spec 068 tests passed 5/5 and focused Clippy passed after the fix.

No further material finding remains after convergence.

## Scope and honesty checks

- No forbidden production verdict assignment (`PROVEN ADVANTAGE`, `OPENMED AHEAD`, `PARITY`, or `SURPASS`) remains on the product surface.
- No OpenMed/Python trusted-runtime dependency, external-weight vendoring, or product online-acquisition dependency was introduced.
- No real-PHI authority or MESC mutation was introduced.
- OpenMed model count remains contextual anti-metric data, not a parity metric.
- The runtime comparison remains `NO_RUNTIME_WINNER` because there is no same-model/same-corpus/same-hardware BenchmarkManifest.
- Release readiness, WCAG conformance, production clinical-model promotion, private-data readiness, and multi-client release readiness remain unclaimed.

## Verdict

`NO_MATERIAL_FINDINGS_REMAIN`

This review establishes review convergence for Spec 071 only. Exact-head required CI, protected merge, and post-main verification remain required before canonical closure.
