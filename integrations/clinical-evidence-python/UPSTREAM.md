# OpenMed-derived clinical evidence modules

- Upstream: https://github.com/maziyarpanahi/openmed
- Exact source commit: `3e2323288ec551157c0b943b331f399ef1434649` (branch `master` at acquisition)
- Original source license: Apache-2.0 (see third_party/openmed/LICENSE).
- Original upstream NOTICE preserved in third_party/openmed/NOTICE.
- Changes: modules copied into the `medscale_clinical` namespace,
  product identifiers replaced, selected imports adjusted, and minimal
  package initializers supplied. All eight corresponding upstream test
  modules were ported to the renamed namespace. No weights, datasets,
  cloud adapters, or background network services were copied.
- Integration: an isolated, optional Python source package. Importing it
  does not automatically activate any clinical authority in MedScale's Rust
  desktop or FHIR flows. Clinical outputs require qualification and review.
- The copy script fetches from GitHub when explicitly run by a developer;
  importing and executing the copied library does not fetch source code.

## Tranche 2 — additional source-derived clinical safety modules

- Exact upstream commit unchanged: `3e2323288ec551157c0b943b331f399ef1434649`.
- 26 additional modules and their original test files were copied and renamed
  with the reproducible `scripts/vendor_openmed_clinical_v2.py` importer.
- Some source tests exercise broader upstream modules that are deliberately
  **not** shipped: SDOH extractor/sections, NLI inference, and review storage
  transitions. The three complete upstream reference tests are preserved in
  `tests/upstream_reference/*.py.txt` and the independent pure tests are
  run. These are documented as exclusions, not as passing tests.
- The original Apache-2.0 license/NOTICE are preserved under `third_party/`;
  each adapted Python implementation has an explicit derivative header.
- For an inventory and validation evidence, see `FEATURE_PARITY_V2.md`.
- Neither tranche grants real-PHI processing authorization or release readiness.

## File inventory

Every file derived from OpenMed at `3e2323288ec551157c0b943b331f399ef1434649`. "Upstream blob" is the Git
blob id at that commit; "Local SHA-256" is the first 16 hex digits of this
repository's file. MedScale-written files (package initializers, the
conformance generator and its test) are not listed.

| Local path | Upstream path | Upstream blob | Local SHA-256 | Change |
|---|---|---|---|---|
| `src/medscale_clinical/clinical/citation_boundaries.py` | `openmed/clinical/citation_boundaries.py` | `6ead30545564` | `cc4a86a2591ed52d` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/citation_minimality.py` | `openmed/clinical/citation_minimality.py` | `6da08fba89c7` | `f31a6ea9efaf3048` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/citation_ordering.py` | `openmed/clinical/citation_ordering.py` | `64a8cc790a55` | `003b547bc5049ed7` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/claim_conflicts.py` | `openmed/clinical/claim_conflicts.py` | `3a7aeea3a4ef` | `66b4332f103c9155` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/claim_integrity.py` | `openmed/clinical/claim_integrity.py` | `eb8a5f62298e` | `669b43cb9dd15b1d` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/decision_support.py` | `openmed/clinical/decision_support.py` | `a17469ac6a1f` | `1ccb0a0986b2a019` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/evidence_coverage.py` | `openmed/clinical/evidence_coverage.py` | `e54c2726b46f` | `ccccc71c8ebe2036` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/evidence_recency.py` | `openmed/clinical/evidence_recency.py` | `c18426f3f602` | `58b9ff2ca9de4d27` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/evidence_span_audit.py` | `openmed/clinical/evidence_span_audit.py` | `79a13c445b42` | `4d835a0b1f8f580e` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/evidence_table.py` | `openmed/clinical/evidence_table.py` | `14b6d14b29ea` | `aea06b67f060020c` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/lab_reference_ranges.py` | `openmed/clinical/lab_reference_ranges.py` | `94b1fa0f2329` | `be048c0b13a5b39f` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/lexicons/__init__.py` | `openmed/clinical/lexicons/__init__.py` | `989ac4f96872` | `9b026fe58c8ebf61` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/lexicons/clinical_norm.py` | `openmed/clinical/lexicons/clinical_norm.py` | `47d6d04d2867` | `e64594eb6ea013dc` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/nli_batch_plan.py` | `openmed/clinical/nli_batch_plan.py` | `ca8a65db4e08` | `e1b715aebce6986a` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/nli_gate.py` | `openmed/clinical/nli_gate.py` | `635719e7bb39` | `221ec1c48be579ab` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/nli_hypothesis_guard.py` | `openmed/clinical/nli_hypothesis_guard.py` | `8502f6d484c5` | `5df3279b19aace90` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/nli_labels.py` | `openmed/clinical/nli_labels.py` | `aae25390cc5a` | `136f4c4f1f3fdb59` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/nli_numeric_precheck.py` | `openmed/clinical/nli_numeric_precheck.py` | `2adeddec7376` | `0dae5bb0bfa2bd7e` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/nli_premise_window.py` | `openmed/clinical/nli_premise_window.py` | `e815a562f121` | `3c8bd1ef4c7e40d7` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/output_use_policy.py` | `openmed/clinical/output_use_policy.py` | `f26e2bb6a853` | `b4a3f068b08f9718` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/refusal_taxonomy.py` | `openmed/clinical/refusal_taxonomy.py` | `3acec51493d4` | `b79cc8846d85a378` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/review_queue_sla.py` | `openmed/clinical/review_queue_sla.py` | `3f56e1c48925` | `b7de95b5e9794176` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/review_state_machine.py` | `openmed/clinical/review_state_machine.py` | `27f42b1c1ecf` | `be6d0b6324953d1a` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/sdoh_completeness.py` | `openmed/clinical/sdoh_completeness.py` | `020b1448d39f` | `2b325ca5a7d6a7c9` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/sdoh_sensitive_use.py` | `openmed/clinical/sdoh_sensitive_use.py` | `53a579df0346` | `e0be43796efb12fb` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/summary_citations.py` | `openmed/clinical/summary_citations.py` | `99043fe7f2ca` | `38e58a91cdd9d0b6` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/summary_demographic_minimizer.py` | `openmed/clinical/summary_demographic_minimizer.py` | `c17b7a882c89` | `f526f3b1b6c66de4` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/summary_empty_evidence.py` | `openmed/clinical/summary_empty_evidence.py` | `33ce405ea871` | `ea98acf344cba66e` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/summary_length_budget.py` | `openmed/clinical/summary_length_budget.py` | `f788aedd2e08` | `56a4e2e1eccf587b` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/summary_omission_budget.py` | `openmed/clinical/summary_omission_budget.py` | `d11b0c8b83f0` | `a3af2eb402149d21` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/summary_output_normalizer.py` | `openmed/clinical/summary_output_normalizer.py` | `120abe68f53f` | `063a5b8adb08d205` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/summary_profiles.py` | `openmed/clinical/summary_profiles.py` | `2328e13737be` | `5ac1cc810b80b2b6` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/summary_retry_control.py` | `openmed/clinical/summary_retry_control.py` | `6bfc4f956de3` | `307dc5adc71b87aa` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/summary_section_plan.py` | `openmed/clinical/summary_section_plan.py` | `788ee3537578` | `b3b3a376edfa4b77` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/uncertainty_disclosure.py` | `openmed/clinical/uncertainty_disclosure.py` | `94761164faa1` | `963bf4dd19d43b44` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/uncertainty_sources.py` | `openmed/clinical/uncertainty_sources.py` | `51e8c8b1fa07` | `ded39840a58e73b2` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/units/__init__.py` | `openmed/clinical/units/__init__.py` | `2ecae7fdd849` | `81cbde2f09051515` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/units/convert.py` | `openmed/clinical/units/convert.py` | `34d1dc2a75a3` | `e5336dcd9cfc4e21` | namespace and product-name substitution |
| `src/medscale_clinical/clinical/units/ucum.py` | `openmed/clinical/units/ucum.py` | `d8d8b280b36c` | `20c6d92c528d0f0b` | namespace and product-name substitution |
| `src/medscale_clinical/core/audit.py` | `openmed/core/audit.py` | `bf58fdf6b777` | `47780037128bf18e` | namespace and product-name substitution |
| `src/medscale_clinical/core/iso_temporal.py` | `openmed/core/iso_temporal.py` | `30fd0c7ac188` | `5c73ab715e64627b` | namespace and product-name substitution |
| `tests/test_citation_boundaries.py` | `tests/unit/clinical/test_citation_boundaries.py` | `871d5f135bd8` | `0c0a35874476009c` | namespace and product-name substitution |
| `tests/test_citation_minimality.py` | `tests/unit/clinical/test_citation_minimality.py` | `077eebf24f31` | `9a9deae6ac468822` | namespace and product-name substitution |
| `tests/test_citation_ordering.py` | `tests/unit/clinical/test_citation_ordering.py` | `72c26898e066` | `c188fa3dfbc6e355` | namespace and product-name substitution |
| `tests/test_claim_conflicts.py` | `tests/unit/clinical/test_claim_conflicts.py` | `5422470c5672` | `0ec2c6555a586aaa` | namespace and product-name substitution |
| `tests/test_claim_integrity.py` | `tests/unit/clinical/test_claim_integrity.py` | `0f1419bfebc2` | `1d35ad88cb4e9c1f` | namespace and product-name substitution |
| `tests/test_decision_support.py` | `tests/unit/clinical/test_decision_support.py` | `cf0552ccfca8` | `9237c032b3880bed` | namespace and product-name substitution |
| `tests/test_evidence_coverage.py` | `tests/unit/clinical/test_evidence_coverage.py` | `d15ef4fda3e8` | `4e0f642938e39539` | namespace and product-name substitution |
| `tests/test_evidence_recency.py` | `tests/unit/clinical/test_evidence_recency.py` | `3b5e45b6e7b2` | `f0a2f90a29627e8c` | namespace and product-name substitution |
| `tests/test_evidence_span_audit.py` | `tests/unit/clinical/test_evidence_span_audit.py` | `b6088d5490c6` | `3a657540df46a799` | namespace and product-name substitution |
| `tests/test_evidence_table.py` | `tests/unit/clinical/test_evidence_table.py` | `7be3684da003` | `33d350090bb649d1` | namespace and product-name substitution |
| `tests/test_lab_reference_ranges.py` | `tests/unit/clinical/test_lab_reference_ranges.py` | `a92c1f1c6531` | `9e5aa1192cd71492` | namespace and product-name substitution |
| `tests/test_nli_batch_plan.py` | `tests/unit/clinical/test_nli_batch_plan.py` | `5c2335498c13` | `83e336dfb940cb8a` | namespace and product-name substitution |
| `tests/test_nli_gate.py` | `tests/unit/clinical/test_nli_gate.py` | `a4f49fc1b49e` | `63ebae70a863c48e` | namespace and product-name substitution |
| `tests/test_nli_hypothesis_guard.py` | `tests/unit/clinical/test_nli_hypothesis_guard.py` | `278a73cd5b12` | `29c62e49fa331600` | namespace and product-name substitution |
| `tests/test_nli_labels.py` | `tests/unit/clinical/test_nli_labels.py` | `4353e6e5545e` | `7c38ca227a8b2d07` | namespace and product-name substitution; reduced to self-contained tests |
| `tests/test_nli_numeric_precheck.py` | `tests/unit/clinical/test_nli_numeric_precheck.py` | `0a6eea7a8ecc` | `cb2b896c55d283e6` | namespace and product-name substitution |
| `tests/test_nli_premise_window.py` | `tests/unit/clinical/test_nli_premise_window.py` | `5fe18c03c189` | `69d5e5f5545e3bf6` | namespace and product-name substitution |
| `tests/test_output_use_policy.py` | `tests/unit/clinical/test_output_use_policy.py` | `642b3d9bb404` | `9b277a64ea40fc37` | namespace and product-name substitution |
| `tests/test_refusal_taxonomy.py` | `tests/unit/clinical/test_refusal_taxonomy.py` | `2a8a26906890` | `5033c1f07a178142` | namespace and product-name substitution |
| `tests/test_review_queue_sla.py` | `tests/unit/clinical/test_review_queue_sla.py` | `d2113e186d0c` | `97cf10ba27f216f7` | namespace and product-name substitution |
| `tests/test_review_state_machine.py` | `tests/unit/clinical/test_review_state_machine.py` | `cc1e6321c1a0` | `50025bd2abb30430` | namespace and product-name substitution; reduced to self-contained tests |
| `tests/test_sdoh_completeness.py` | `tests/unit/clinical/test_sdoh_completeness.py` | `a83dd989fb94` | `0ec63fb6eaa6c7c6` | namespace and product-name substitution; reduced to self-contained tests |
| `tests/test_sdoh_sensitive_use.py` | `tests/unit/clinical/test_sdoh_sensitive_use.py` | `9fdfead7084d` | `1f08a7b9e0048eb4` | namespace and product-name substitution |
| `tests/test_summary_citations.py` | `tests/unit/clinical/test_summary_citations.py` | `ef2b3d5aef6c` | `c71d389881b6fad8` | namespace and product-name substitution |
| `tests/test_summary_demographic_minimizer.py` | `tests/unit/clinical/test_summary_demographic_minimizer.py` | `a31d9a90377b` | `a9b6fc461442768b` | namespace and product-name substitution |
| `tests/test_summary_empty_evidence.py` | `tests/unit/clinical/test_summary_empty_evidence.py` | `3e46d8346060` | `32e878562031e158` | namespace and product-name substitution |
| `tests/test_summary_length_budget.py` | `tests/unit/clinical/test_summary_length_budget.py` | `8d1002163599` | `0d66cd016e3c7c2f` | namespace and product-name substitution |
| `tests/test_summary_omission_budget.py` | `tests/unit/clinical/test_summary_omission_budget.py` | `3ba4822dbf17` | `b400663263791ea2` | namespace and product-name substitution |
| `tests/test_summary_output_normalizer.py` | `tests/unit/clinical/test_summary_output_normalizer.py` | `57a8b96a957e` | `e865c68dccc6b0cb` | namespace and product-name substitution |
| `tests/test_summary_profiles.py` | `tests/unit/clinical/test_summary_profiles.py` | `ba2aa380c584` | `5b51c168d502e60f` | namespace and product-name substitution |
| `tests/test_summary_retry_control.py` | `tests/unit/clinical/test_summary_retry_control.py` | `b56240db0d18` | `5ec03d1f8da0f661` | namespace and product-name substitution |
| `tests/test_summary_section_plan.py` | `tests/unit/clinical/test_summary_section_plan.py` | `3305193193b1` | `a55a73dc994638ba` | namespace and product-name substitution |
| `tests/test_uncertainty_disclosure.py` | `tests/unit/clinical/test_uncertainty_disclosure.py` | `a36b95135191` | `27e52f6ea89d9263` | namespace and product-name substitution |
| `tests/test_uncertainty_sources.py` | `tests/unit/clinical/test_uncertainty_sources.py` | `82f466eea8d5` | `9b3c12fca0c27bf5` | namespace and product-name substitution |
| `tests/upstream_reference/nli_labels_full.py.txt` | `tests/unit/clinical/test_nli_labels.py` | `4353e6e5545e` | `a366fe100acbaf99` | namespace and product-name substitution; NOT RUN (reference only) |
| `tests/upstream_reference/review_state_machine_full.py.txt` | `tests/unit/clinical/test_review_state_machine.py` | `cc1e6321c1a0` | `8da51634cd1ddffc` | namespace and product-name substitution; NOT RUN (reference only) |
| `tests/upstream_reference/sdoh_completeness_full.py.txt` | `tests/unit/clinical/test_sdoh_completeness.py` | `a83dd989fb94` | `2dfafb580bab76cc` | namespace and product-name substitution; NOT RUN (reference only) |
| `third_party/openmed/LICENSE` | `LICENSE` | `261eeb9e9f8b` | `c71d239df91726fc` | verbatim |
| `third_party/openmed/NOTICE` | `NOTICE` | `3641fd35cc82` | `c89b3eeffaf8456d` | verbatim |

## Revisions after import

- `src/medscale_clinical/clinical/__init__.py` exports names explicitly,
  following upstream `openmed/clinical/__init__.py` at the pinned commit
  (aliases such as `CoverageReviewState` included). The wildcard imports
  written by the importers let `ReviewState`, `REVIEW_STATES`, `Citation`
  and `EvidenceSpan` resolve to the wrong module; a test now fails on any
  duplicate export.
- Change notices name each upstream path (this inventory).
- `decision_support.py`: the rename had turned an upstream statement into a
  claim about MedScale and FDA guidance; the sentence now describes design
  intent only.
- `core/audit.py`: `manifest_hash()` and `AuditReport.attest()` depend on
  upstream files that are not vendored; they now raise instead of returning
  a misleading hash or failing on import.
- The importer scripts record the original import; rerunning them on this
  tree is refused, and their output is superseded by these revisions.
