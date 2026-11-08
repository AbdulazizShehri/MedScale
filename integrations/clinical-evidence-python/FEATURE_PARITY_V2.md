# OpenMed additional clinical safety parity — tranche 2

**Status:** Ported into the optional MedScale Python package; not integrated into
native Rust/Core/desktop/FHIR decision paths; synthetic qualification only.

## Pin and provenance

Upstream: [maziyarpanahi/openmed](https://github.com/maziyarpanahi/openmed).
Pinned commit: `3e2323288ec551157c0b943b331f399ef1434649`.
Upstream license: **Apache-2.0**, with original source LICENSE and NOTICE kept
at `third_party/openmed/`. Per-file adaptation headers and the exact source pin
identify the origin of these renamed files. No datasets, proprietary
terminologies, hosted APIs, medical model weights or original OpenMed runtime
are required.

## 26 additional modules copied and renamed

| Family | MedScale Python modules | What this adds |
|---|---|---|
| Evidence/citation | `citation_boundaries`, `evidence_coverage`, `evidence_table` | Citation offset and coverage checks, metadata-only review tables |
| Safe summaries | `summary_omission_budget`, `summary_length_budget`, `summary_empty_evidence`, `summary_demographic_minimizer`, `summary_output_normalizer`, `summary_retry_control`, `summary_profiles`, `summary_section_plan` | Omission and empty-evidence gates, length/section plans, bounded retries, demographic minimization |
| Review and disclosure | `review_queue_sla`, `review_state_machine`, `refusal_taxonomy`, `output_use_policy`, `uncertainty_disclosure`, `uncertainty_sources`, `decision_support` | Human-review transitions, refusal/use policies, explicit uncertainty and guarded CDS envelopes |
| Clinical metadata | `lab_reference_ranges` | Explicit lab reference-range provenance and comparison context |
| NLI guards | `nli_hypothesis_guard`, `nli_premise_window`, `nli_gate`, `nli_labels`, `nli_batch_plan` | Small premise windows, hypothesis limits, abstention and bounded local batching |
| Sensitive SDOH | `sdoh_completeness`, `sdoh_sensitive_use` | Explicit processing completeness and prohibited-use labels |

The 26 modules are in
`src/medscale_clinical/clinical/`, extending the eight feature families
introduced in [PR #477](https://github.com/AbdulazizShehri/MedScale/pull/477).
They are source ports, not newly invented lookalikes.

## Verification and test scope

- Local Windows Python 3.14: **435 pytest tests and 182 subtests passed**,
  covering tranche 1 and tranche 2, using a private isolated temporary
  pytest directory due to an unrelated Windows global-temp permissions issue.
- A fresh worktree based on the original PR #477 branch regenerated the
  tranche from pinned upstream source, then passed the same 435/182 suite.
- Existing CI `.github/workflows/clinical-evidence-python.yml` runs this
  isolated Python package on Ubuntu Python 3.11 and Windows Python 3.14 when
  this directory changes. CI is separate from Rust qualification.

**Focused exclusions, without disguising them as passing tests:**
- `sdoh_completeness`: first seven pure-audit tests retained; the original
  whole-stack SDOH extractor/section-parser tests are kept as
  `tests/upstream_reference/sdoh_completeness_full.py.txt`.
- `nli_labels`: excluded one upstream compatibility test importing the full
  `clinical.nli` verifier; full original kept as
  `tests/upstream_reference/nli_labels_full.py.txt`.
- `review_state_machine`: excluded one upstream compatibility test importing
  `review_transitions` (requires separate database/workflow imports);
  original kept as `tests/upstream_reference/review_state_machine_full.py.txt`.
- `summary_card` was examined but **not ported** because it imports the
  broader `processing.text` suppression stack. The previous tranche also
  left `summary_claim_segments` out because of the sentence/language
  segmentation dependencies. These remain explicit gaps.

## Safety/architecture boundaries

Metadata checks cannot validate the clinical truth of a diagnosis or treatment.
These modules are **not** clinically qualified, not available to real patient
workflows, and do not make autonomous clinical decisions. No production use
is authorized by this PR. Lab and policy rules require expert validation;
model NLI entailment accuracy is not established by structural gate tests.
The Rust authority/Core integration, FHIR binding and synthetic desktop E2E
qualification are separate future engineering gates.

## Reproduce the source import

From repository root, after authenticating `gh` for public source access:

```bash
python scripts/vendor_openmed_clinical_v2.py
```

The script pins exact upstream Git SHA and refuses overwriting already
imported files. For an offline source checkout at the exact pinned SHA:

```bash
python scripts/vendor_openmed_clinical_v2.py --source-root /path/to/openmed
```

To verify in a clean Python 3.11+ environment:

```bash
cd integrations/clinical-evidence-python
python -m pip install -e ".[test]"
python -m pytest tests -q
```
