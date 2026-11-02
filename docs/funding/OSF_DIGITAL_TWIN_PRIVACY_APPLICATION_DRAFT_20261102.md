# Open Science Foundation — Verifiable Privacy for Digital Health Twins

**Status:** DRAFT / NOT SUBMITTED, prepared 2026-11-02. Official browser form partially completed (name, email, project title, 520-word proposal, question and deciding test; 43% completion shown). The official confirmation and application reference were **not** reached. Do not represent this as submitted or funded.

**Official form:** https://funding.opening.science/health-twin-privacy-2026/
**Deadline:** 2026-12-15, 23:59:59 CET
**Applicant:** Abdulaziz M. Alshehri, MPH — independent Saudi open-source maintainer (no current Manchester affiliation)
**Title:** Person-Verifiable Privacy for Local FHIR Health AI Agents
**Proposed ask:** CHF 20,000 / 9 months, subject to official application rules.

## Proposal field (520 words; drafted and entered into official browser form)

MedScale's long-term mission is to make clinician-controlled, AI-assisted clinical documentation free and local-first, rather than dependent on commercial ambient-AI subscriptions. This application is deliberately narrower: research into person-verifiable privacy for a single synthetic FHIR health-data twin. The existing Apache-2.0 MedScale engineering beta provides local workspaces, provenance, policy surfaces and optional local Whisper transcription on nonpatient material. It is not clinically validated and is not authorized for real patient records. The proposed privacy verifier is NEW research, not an existing guarantee.

Problem and privacy claim: A health-data owner cannot currently verify whether an AI agent stayed within its approved tools, nor whether a presented audit trail is complete. We will test the conditional claim that, for all accesses made through an explicitly trusted local reference-monitor gateway, a person can independently check that each read or export matched a time-bound policy, that receipts form a non-reordered sequence, and that no mediated receipt was silently removed after the last witnessed checkpoint. The guarantee applies only to instrumented, mediated accesses. Kernel compromise, bypassing the gateway, dishonest hardware, malicious pre-instrumentation copies and remote recipients who retain exports are explicitly outside it; we will publish these limitations rather than suggest that encryption or a hash chain proves total confidentiality.

Method: We will implement a small reference monitor with capability-limited access to per-person synthetic FHIR R4 resources. Every approved or rejected access will generate a signed or MAC-authenticated, hash-linked receipt containing only necessary metadata: action, resource class, policy version, model/tool identity, monotonic sequence and a declared reason. Independently stored checkpoints on the person's device let an offline verifier detect altered, reordered or removed recorded events. This is a testable protocol-level design, not a claim of hardware attestation. An optional owner-controlled summary will aggregate observed export counts and policy exceptions across sessions; this measures mediated disclosures, not information leakage beyond the gateway.

Deciding experiment: Publish a synthetic, adversarial benchmark with authorized reads, rejected requests, policy changes, missing receipts, forked chains, replayed events, false attestations, and unauthorized export attempts. Blindly inject faults into scripted sessions. Measure detection rate and false-alarm rate by fault class, end-to-end verification latency and storage overhead, and the fraction of deliberately unmediated access attempts that the architecture cannot observe. Separately test usefulness trade-offs, including denied legitimate actions and delays to local audio-note workflows.

Outputs and schedule: Months 1-2: public threat model, protocol specification, pinned synthetic fixture generator and preregistered evaluation. Months 3-5: runnable monitor, offline verifier, negative tests and CI. Months 6-7: scripted red-team experiments and independent reproduction. Months 8-9: performance study, clearly scoped privacy claims, open report and archival release. Public code will be Apache-2.0; reports CC BY 4.0 and original synthetic benchmark data CC0 or CC BY 4.0 with SPDX tags. The key experiment will rerun with one documented command on ordinary hardware without paid models or proprietary services. We will seek a community reproducer and report failed tests as openly as successful ones.

Impact: An evidence-based, nonclinical research artifact that helps individuals and open-source health-tool developers check what an AI agent actually did with one person's medical data. It does not assert clinical safety, privacy for unmonitored activity or deployability with patients.

## Main research question (entered in draft form)

Under an explicit reference-monitor trust assumption, can a person independently detect dropped, altered, replayed, reordered or policy-inconsistent AI-agent access records for one synthetic FHIR health twin across repeated sessions, and quantify the verifier's overhead and blind spots for accesses that bypass mediation?

## Main claim and how to test (entered in draft form)

Hypothesis: for all accesses forced through a trusted local reference monitor, hash-linked receipts with independently witnessed checkpoints enable an offline person-controlled verifier to detect targeted tampering and policy violations with low false-positive rates and bounded runtime cost. We will preregister a synthetic benchmark and inject missing, forged, reordered and replayed events and unauthorized export attempts. Each result will be compared to an independent ground-truth oracle, reporting per-class sensitivity, false positives, latency and storage overhead. Bypass tests must demonstrate the architecture's stated limit: accesses outside the monitor are not provably observable. A failed experiment will refute the relevant part of the claim.

## Remaining requirements before real application

- Complete cost and duration fields with itemized CHF 20k budget that strictly fits personnel / compute / data-access rules; verify no overlapping grant costs.
- Add verified existing public GitHub artifact with reproducible runnable **demo** and explicit separate planned new privacy work.
- Complete synthetic-data ethics description and honest declaration of any AI drafting/research tools used.
- **Connect a verified LinkedIn, ORCID or Bluesky identity**. LinkedIn OAuth redirected to login on 2026-11-02; it was not linked. Do not enter credentials via an assistant tool or claim completed connection.
- **Record and upload a genuine <=90 second introduction or demo** to YouTube/Vimeo (unlisted allowed), and supply the accessible URL. Not yet uploaded.
- Read terms. Required reviewer commitment is up to four anonymous peer applications within 14 days, and the research question is temporarily public after the call closes. Only complete submission with official receipt counts.

No sensitive authentication or banking information is stored in this repo.

## Submission-ready remaining fields — prepared 2026-11-02, NOT entered into browser

**Important:** These are copy-ready OFFLINE answers, not proof that the browser saved them. Existing browser draft still reported 43% in the last visible checkpoint. Verify official form input limits and the local browser's saved state before entering. No profile was authenticated, no video uploaded and no application receipt obtained.

### Requested funding and duration

- Requested funding: **CHF 20000** (type integer `20000`).
- Duration: **9 months**, starting **2027-02-01** if selected.
- The project is NEW research on the verifiable privacy layer, not repayment for already developed MedScale features. Do not claim an award or any double-funded expense.

### Budget justification (under 100 words; verify terms before submission)

> CHF 18,000 supports 400 hours of investigator effort at CHF 45/hour over nine months: formal threat model and protocol design (90h), reference-monitor and verifier implementation (145h), adversarial tests and blind re-runs (110h), open documentation and reproduction (55h). CHF 2,000 supports isolated compute and cross-platform test runs for tampering and denial-of-service experiments, with no paid-model or proprietary-service requirement for third-party reproduction. Synthetic fixtures and open tools are free. This budget excludes existing MedScale development, hardware purchases, clinical work, and any costs covered by other grants.

**Check:** CHF 18,000 + CHF 2,000 = **CHF 20,000**, personnel plus compute only. Rate is an illustrative requested investigator rate, not a payroll/employment claim. Hardware request in other programmes is intentionally outside this application. Review final budget for tax and invoice rules with the funder; never use the same hour or invoice for two grants.

### Data, permissions and ethics (under 150 words)

> All evaluation will use reproducible synthetic FHIR R4 records and, where useful, explicitly licensed public nonpatient fixtures with documented provenance. No identifiable patient records, live clinical encounters, biometric identifiers or private data from other people will be collected, uploaded, or processed. We will publish synthetic generators, machine-readable fixture schemas and the exact adversarial mutation recipes. The experiment is software/security protocol research, not a medical intervention or human-subjects study; on this design, no patient consent or clinical ethics approval is claimed or required. We will reassess legal/ethics requirements before any material protocol change, user study, or use of real personal health information; any such activity is out of grant scope until appropriate approval. Results will report both detectability and demonstrable limitations of the reference-monitor trust assumption.

### AI-use disclosure (under 150 words)

> OpenAI ChatGPT assisted with opportunity research, proposal wording, budget structuring, question formulation and editing. The named applicant has to independently verify sources, code, protocol assumptions and every final claim before submission. Any subsequent coding, peer review or test-assistance model will be logged with its purpose in the public research methodology. No sensitive patient data are shared with AI systems. The foundation's official Theoria assessment is described in the Call Terms; only the application research question is sent to it by the foundation. We will not put other applicants' review material into unapproved AI tools.

### Existing public runnable work: truthful artifact boundary

- Public existing code: https://github.com/AbdulazizShehri/MedScale (Apache-2.0).
- Documented existing CLI smoke demonstration on a machine with pinned repository Rust toolchain and platform dependencies:
  ```sh
  git clone https://github.com/AbdulazizShehri/MedScale.git
  cd MedScale
  cargo run -p medscale-cli --locked -- --version
  cargo run -p medscale-cli --locked -- doctor
  ```
- Separate existing native audio integration evidence is linked in the repository README, but it does **not** show the proposed privacy monitor functioning.
- The proposed privacy receipts and offline independent verifier do **not** exist as a grant-ready working demo yet. This distinction must be made clear when describing the existing artifact.
- **Not yet verified:** fresh clean-room run of these commands on a pinned public commit, live screen capture of the documented synthetic workflow, funder's interpretation of a sufficient working demo, and public reproducible privacy research example. An existing CLI doctor's successful exit alone should not be represented as the new privacy-verification demonstration.

### Personal introduction video — script and filming checklist, NOT recorded

A simple screen recording or personal introduction **no longer than 90 seconds** is required on YouTube or Vimeo, public/unlisted. This is a proposed ~75-second script; use your own voice:

> Hello. I'm an independent open-source developer with a Master of Public Health from the University of Melbourne. I'm building MedScale, a free, local-first desktop project for health information, transparency and FHIR interoperability. Today's project is an engineering beta tested on synthetic and licensed nonpatient data, not a clinically validated system.
>
> My research question is narrow: can a person actually verify what an AI agent was allowed to read and export from one synthetic digital health twin? I propose an independently checkable policy gateway, witnessed audit receipts, and a public set of adversarial tests for missing, forged and replayed records.
>
> The hard part is being honest about what cannot be seen: accesses that bypass the gateway are not covered. Over nine months, I would publish the protocol, test harness, results, failures and reproduction instructions under open licences. Thank you for considering the proposal.

**Required person-side actions:** create real recording showing you or your authentic project demonstration; upload to your YouTube/Vimeo account; verify that the URL is playable without authentication and its length is at most 90 seconds. Do not invent or generate a claimed first-person recording. **Then** connect genuine LinkedIn/ORCID/Bluesky through the official grant site, review consent for temporarily publishing the research question and the obligation to review up to four applications, and only submit after the official receipt/reference is visible.

### On-screen form fields not yet completed

| Official field | Prepared material | Submission status |
|---|---|---|
| Requested funding | 20000 | OFFLINE READY, NOT ENTERED |
| Duration | 9 months | OFFLINE READY, NOT ENTERED |
| Budget justification | 85-word budget above | OFFLINE READY, NOT ENTERED |
| Existing runnable repository | MedScale public URL, with limitations above | NEEDS VERIFIED DEMO |
| Data and ethics | text above | OFFLINE READY, NOT ENTERED |
| AI disclosure | text above | OFFLINE READY, NOT ENTERED |
| Linked ORCID/LinkedIn/Bluesky | real applicant OAuth required | BLOCKED |
| YouTube/Vimeo video | script drafted, actual clip/URL required | BLOCKED |
| Final policy/peer-review consent and submission | applicant acceptance of binding terms, then official receipt | NOT SUBMITTED |

**Official source:** https://opening.science/apply/health-twin-privacy-2026/ ; form https://funding.opening.science/health-twin-privacy-2026/ ; deadline **2026-12-15 23:59:59 CET**.

## Live connected Mac browser checkpoint — 2026-11-02

**Saved draft 86% complete, NOT SUBMITTED**, on the original 7-question foundation form in Vivaldi. Official site persisted local draft under its own browser storage. Name, email, title, proposal, question, claim/measurement, CHF 20,000 request, 9-month duration, budget, public MedScale GitHub repository, synthetic data ethics and AI-use disclosure were entered and storage confirmed. Do not assume this local draft exists in another browser or that it was submitted to the foundation.

Clicking `Connect LinkedIn` opened LinkedIn's **sign-in page**, not an authorized identity callback, so there is **no verified connected profile**. YouTube Studio in the existing Google account prompted creation of a new channel, so no YouTube account/channel was created and **no video was uploaded**. The final form requires a real linked ORCID/LinkedIn/Bluesky identity, public/unlisted YouTube/Vimeo authentic clip <=90 seconds, meaningful runnable artifact evidence and applicant consent to research-question publication, Call Terms and reciprocal peer review. The 86% reflects completed content fields, **not** submission readiness.

The deadline remains **2026-12-15 23:59:59 CET**. A provider-issued submission reference is required before status `submitted` may be used.
