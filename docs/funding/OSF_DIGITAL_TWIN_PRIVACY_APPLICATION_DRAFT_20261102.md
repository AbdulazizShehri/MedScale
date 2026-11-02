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
