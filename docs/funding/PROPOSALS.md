# MedScale — tailored funding proposal drafts (not submitted)

_Last reviewed 2026-11-02. Do not send these as a universal mass application. Adjust to each provider's current form and word limit. All claims must be independently checked._

## Common one-paragraph project overview

MedScale is a free Apache-2.0, local-first health intelligence project built with Rust and a Tauri/React desktop. Its design emphasizes explicit provenance, uncertainty, human oversight, privacy-preserving local workspaces, and bounded FHIR R4 interoperability. It also supports user-approved local Whisper transcription of appropriately licensed **nonpatient** audio. Current development is an engineering Beta using synthetic health-record fixtures and public/licensed nonpatient benchmarks. It is **not** clinically validated, a medical device, or authorized for processing real patient records. Repository: https://github.com/AbdulazizShehri/MedScale . The project is seeking independent support to develop reproducible safety/evidence tests and improve open-source maintenance capacity.

## Proposal A — Hacker Initiative (2026 Signal/Noise)

**Working title:** Open Source Provenance and Integrity Tests for Evidence-Grounded Health Software

**Request:** USD $15,000 cash grant for **eligible non-salary project costs** over a proposed six-month period, subject to programme approval.

**Problem:** AI-assisted health-information tools can present convincing outputs without showing which source supports which assertion. Open-source maintainers need practical, reproducible ways to detect absent, contradictory, changed, or unverifiable evidence before an application displays such outputs. This is a source-integrity and developer-education problem, not a request to automate diagnosis.

**Proposed approach:** Build reusable tests and developer guides around the existing open MedScale provenance/validation architecture. Create synthetic, intentionally conflicting evidence packages; tampered and missing source records; deterministic expected outcomes; and public integrity-check artifacts. The work will not ingest PHI or claim clinical validity.

**Proposed outputs and milestones:**
- Month 1: threat model and public requirements for provenance integrity and failure modes; synthetic test-plan specification.
- Months 2–3: publicly licensed synthetic corpus and fixture generators covering missing source, disagreement, stale source, altered source hash, and unsupported claims.
- Months 3–4: reproducible source-integrity regression tests and automated CI evidence; record false-positive/false-negative limitations.
- Months 5–6: independent test reproducibility instructions, open developer workshop materials (no paid staff training), public results, failure analysis and issues for follow-on work.

**Public benefit:** A reusable methodology for developers to test whether a tool accurately represents what its sources do—and do not—support, with a focus on privacy-preserving local software and no need for clinical data.

**Proposed expense request:** See [BUDGET.csv](BUDGET.csv). Categories are prospective and subject to funder confirmation. **No salaries, wages, staff training, deficits, websites, or marketing costs are requested.**

**Evaluation targets (proposed, not achieved):**
- At least 50 synthetic provenance scenarios with declared expected outcomes, including negative/tampering cases.
- Deterministic reproducibility instructions and public CI checks for all published fixtures.
- Open-source documentation mapping each failure class to a test and limitation.
- A public report with outcomes and unsuccessful or inconclusive tests.

**Risks:** Clinical misinterpretation, dataset licensing errors, and overclaiming generality. All artifacts will be explicitly research/developer-facing, synthetic or public licensed, and non-clinical; a failed check must not be hidden.

**Application:** https://hackerinitiative.org/apply-now/ . Deadline Nov 9, 2026. Actual form requires the applicant's own contact, disclosure and agreement. Do not submit without those details and review.

## Proposal B — Anthropic AI for Science

**Working title:** Reproducible Evaluation of Evidence Attribution, Uncertainty and Abstention in Local-First Health AI

**Request:** Up to USD $10,000 of **Claude API credits**, not cash; adjust requested amount to validated workload estimates and applicable form limits.

**Research question:** Can a transparent evaluation framework measure whether AI-generated summaries and responses (1) cite only available evidence, (2) distinguish conflicts and uncertainty, and (3) abstain rather than fill gaps when context is insufficient?

**Method:** Use entirely synthetic health-record scenarios and suitably licensed public nonpatient text. Create predefined tasks with blinded evidence conditions: fully supported, partially supported, contradictory, outdated, and missing. Compare candidate model configurations to a common human-authored rubric and quantify attribution precision, unsupported assertion rate, abstention sensitivity, audit-record completeness and regressions across model updates. Keep all test identifiers synthetic and disclose limitations on generalizability.

**Claude's proposed role:** Controlled evaluation assistance, evidence-extraction experiments and reproducible analysis scripting. Do not upload actual patient data or confidential partner materials; do not treat model outputs as medical advice.

**Deliverables:** Public protocol and nonclinical fixture taxonomy; runnable benchmark harness; versioned evaluation report; protocol limitations and reproducibility notes; open-source test improvements where license-compatible.

**Current status/risks:** Project is not clinically validated; no claimed institutional research affiliation, clinical cohort, or published study result. Budget, quota and API availability require verification after acceptance. Official program: https://www.anthropic.com/news/expanding-support-for-scientists .

## Proposal C — fal Research Grant (compute in kind)

**Working title:** Open-source multilingual nonclinical audio and evidence QA benchmarks for MedScale

**Request:** An initial compute-credit allocation subject to fal's discretion, focused on bounded GPU evaluation rather than recurring hosting. Do not characterize the website's `$50k average grant check` as a guaranteed cash award; its public offer to OSS researchers describes free compute.

**Work:** Evaluate local/offline Whisper runtimes against licensed public read-speech cohorts; characterize English/Arabic WER/CER and failure cases with clear dialect and domain caveats. Extend synthetic evidence-attribution and model-admission robustness tests. Reuse existing source-pinned model evaluation artifacts where relevant; isolate network execution from any private data.

**Measured baseline:** MedScale already documents a limited 100-excerpt Arabic FLEURS `ar_eg` read-speech benchmark, but it is **not clinical speech** and cannot substantiate medical transcription accuracy; see [qualification records](../qualification/EXTERNAL_GATES.md).

**Outputs:** Reproducible scripts, cost/compute receipts, failure cases, and public documentation of limitations.

**Application channel:** grants@fal.ai, https://fal.ai/grants .

## Proposal D — AWS Cloud Credits for Open Source

**Request:** AWS promotional credits scoped to reproducible open-source test infrastructure; seek programme-defined allocation rather than claim a preset cash amount.

**Justification:** Provide ephemeral isolated Linux test workers for non-PHI Rust Core/FHIR fixture checks, artifact and benchmark retention, controlled dependency/security scans, and synthetic source-integrity regression. MedScale remains local-first; the application does not propose cloud-hosting patient records. Its public license and maintainer activity are verifiable; community adoption is not yet demonstrated.

**Constraints:** AWS requires completion of its own application form, an AWS account subject to its terms, OSI-approved licensing, and evaluation of ecosystem relevance. Use the **newer** programme contact specified in its March 2025 announcement: awsopensourcecredits@amazon.com . See https://aws.amazon.com/blogs/opensource/aws-cloud-credits-for-open-source-projects-affirming-our-commitment/ .

## Proposal E — OpenAI Codex for OSS and GitHub security fund

For OpenAI: Request maintenance automation and code-review support for active Rust/TypeScript OSS; show concrete issue/PR triage, CI integrity and evidence review workflows, and acknowledge limited external adoption. Apply through https://openai.com/form/codex-for-oss/ .

For GitHub Secure OSS: Propose enhancements to supply-chain provenance, dependency admission, synthetic FHIR data isolation, threat-model testing, and contributor security reporting. However, **do not claim significant community adoption**; GitHub explicitly requires traction, so this application should be deferred until usage evidence exists or a programme reviewer accepts an ecosystem-critical exception. Apply at https://github.com/open-source/github-secure-open-source-fund .

## What is not to be claimed

- No real-patient-data or clinical deployment clearance.
- No guaranteed adoption, downloads, users, partnerships, sponsors, research institution or corporate entity.
- No automatic dollar allocation from a cloud programme.
- No use of restricted/patient data in hosted AI evaluations.
- No existing application submission, approval or award unless a receipt is recorded.
