# OpenMed clinical modules in MedScale

Status record for the OpenMed-derived clinical evidence checks. Each column is a
separate completion level; one never implies the next.

- **Copied**: source vendored from `maziyarpanahi/openmed@3e232328` with change notices (`integrations/clinical-evidence-python/UPSTREAM.md`).
- **Unit tested**: the module's original upstream tests pass in the Python package (see the exclusions below).
- **Rust port**: reimplemented in `crates/medscale-clinical`, with reports identical to the Python reference on the conformance vectors.
- **In Core**: called by MedScale Core on a real workflow path.
- **Desktop verified**: exercised by the installed desktop app's qualification.
- **Release ready**: qualified and approved for release.

## Architecture

MedScale's Core can't run Python, R or shell code (founder decision 2026-09-23), and the provenance template forbids OpenMed Python as trusted runtime. So the Python package is a **reference implementation and test oracle only**. Each check Core needs is ported to Rust, and `integrations/clinical-evidence-python/conformance/generate_vectors.py` runs the Python reference on fixed synthetic inputs to produce vectors the Rust tests must match byte for byte, fingerprints included. A Python test fails when the vectors go stale.

## Modules

| Module | PR | Copied | Unit tested | Rust port | In Core | Desktop verified | Release ready |
|---|---|---|---|---|---|---|---|
| `citation_boundaries` | #478 | yes | yes | no | no | no | no |
| `citation_minimality` | #477 | yes | yes | no | no | no | no |
| `citation_ordering` | #477 | yes | yes | no | no | no | no |
| `claim_conflicts` | #477 | yes | yes | no | no | no | no |
| `claim_integrity` | #477 | yes | yes | no | no | no | no |
| `decision_support` | #478 | yes | yes | no | no | no | no |
| `evidence_coverage` | #478 | yes | yes | no | no | no | no |
| `evidence_recency` | #477 | yes | yes | no | no | no | no |
| `evidence_span_audit` | #477 | yes | yes | yes (#479) | yes: Research Canvas view (#480) | no | no |
| `evidence_table` | #478 | yes | yes | no | no | no | no |
| `lab_reference_ranges` | #478 | yes | yes | no | no | no | no |
| `nli_batch_plan` | #478 | yes | yes | no | no | no | no |
| `nli_gate` | #478 | yes | yes | no | no | no | no |
| `nli_hypothesis_guard` | #478 | yes | yes | no | no | no | no |
| `nli_labels` | #478 | yes | yes | no | no | no | no |
| `nli_numeric_precheck` | #477 | yes | yes | no | no | no | no |
| `nli_premise_window` | #478 | yes | yes | no | no | no | no |
| `output_use_policy` | #478 | yes | yes | no | no | no | no |
| `refusal_taxonomy` | #478 | yes | yes | no | no | no | no |
| `review_queue_sla` | #478 | yes | yes | no | no | no | no |
| `review_state_machine` | #478 | yes | yes | no | no | no | no |
| `sdoh_completeness` | #478 | yes | yes | no | no | no | no |
| `sdoh_sensitive_use` | #478 | yes | yes | no | no | no | no |
| `summary_citations` | #477 | yes | yes | no | no | no | no |
| `summary_demographic_minimizer` | #478 | yes | yes | no | no | no | no |
| `summary_empty_evidence` | #478 | yes | yes | yes (#482) | yes: MedAgent run execution (#482) | no | no |
| `summary_length_budget` | #478 | yes | yes | no | no | no | no |
| `summary_omission_budget` | #478 | yes | yes | no | no | no | no |
| `summary_output_normalizer` | #478 | yes | yes | no | no | no | no |
| `summary_profiles` | #478 | yes | yes | no | no | no | no |
| `summary_retry_control` | #478 | yes | yes | no | no | no | no |
| `summary_section_plan` | #478 | yes | yes | no | no | no | no |
| `uncertainty_disclosure` | #478 | yes | yes | no | no | no | no |
| `uncertainty_sources` | #478 | yes | yes | no | no | no | no |

Supporting files `core/audit.py`, `core/iso_temporal.py`, `clinical/units/*` and `clinical/lexicons/clinical_norm.py` are copied and exercised through the modules above.

## Test exclusions

Three upstream test files exercise modules not shipped here (the SDOH extractor and section parser, the NLI verifier, persistence-linked review transitions). Their full originals are kept in `integrations/clinical-evidence-python/tests/upstream_reference/` and are **not run**. Only the self-contained tests from those files run.

## Not imported

`summary_claim_segments`, `summary_card`, the full NLI verifier and inference pipeline, full SDOH extraction and section parsing, and persistence-linked review transitions. Each depends on wider upstream systems.

## Pull requests

| PR | Branch | Content |
|---|---|---|
| [#477](https://github.com/AbdulazizShehri/MedScale/pull/477) | `feat/openmed-clinical-evidence-v1` | 8 evidence modules (Python) |
| [#478](https://github.com/AbdulazizShehri/MedScale/pull/478) | `feat/openmed-clinical-safety-v2` | 26 safety, review and NLI modules (Python), plus review fixes; contains #477 |
| [#479](https://github.com/AbdulazizShehri/MedScale/pull/479) | `feat/clinical-rust-span-audit-20261108` | `medscale-clinical` crate: evidence-span audit port and conformance vectors |
| #480 | `feat/clinical-span-audit-core` | Core: the Research Canvas view lists evidence nodes that cite overlapping text (`CanvasView.overlapping_evidence`); the CLI prints it |

## Where each port runs

- **Evidence-span audit:** `CanvasView` (Core) inspects each Canvas revision for evidence nodes whose spans overlap within one source location (exact, nested or partial). Cross-location pairs are dropped because their offsets count different text. The CLI `knowledge canvas-show` prints the pairs. The desktop app lists canvases but does not show canvas views yet, so this is not desktop-verified.

- **Empty-evidence gate:** `execute_agent_run` (Core) re-resolves the run's context before the model runs. Only artifacts that are current now are cited on the proposal; missing, stale, out-of-scope, unsupported or corrupt ones are left out, and with none left the run is refused (`StaleReference`, code `empty_approved_evidence`) without generating anything. A fleet lane closes as failed with reason `no_current_evidence`. Desktop runs use the same path, but no desktop journey exercises the refusal yet.

## Qualification status

Nothing here is clinically qualified, integrated into a clinical workflow, or authorized for real patient data (`REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`). All tests use synthetic inputs.
