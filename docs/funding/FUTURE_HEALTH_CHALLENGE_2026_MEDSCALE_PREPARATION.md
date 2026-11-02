# Future Health 2026 — MedScale population-health screening-to-care challenge

**Actual status, 2026-11-03 Asia/Riyadh:** **REGISTRATION PREPARED IN OFFICIAL HACK2SKILL BROWSER; NOT YET SUBMITTED.** No registration number or confirmation. Prize money is contingent on independent competition results, not current funding.

## Official opportunity, eligibility and dates

- Organiser: Future Health — A Global Initiative by Abu Dhabi, delivered with Hack2skill, co-created by Microsoft, Inception and MBZUAI.
- Official registration: https://hack2skill.com/event/futurehealthchallenge/ (not a third-party lookalike). 10 challenge statements in fertility, population health, wellbeing.
- Registration deadline: **2026-11-10**. Working challenge-specific prototype with technical documentation, assumptions and safety limitations due **2026-11-20**. Top teams may present at GITEX 2026 Dec 7–11.
- Prizes: **USD 50,000, USD 35,000, USD 20,000** first/second/third, payable only if selected and eligibility verified. No payment or prize guaranteed.
- Solo individual participation from Saudi Arabia with an existing open-source technical prototype is eligible in principle **if significantly extended for a selected challenge statement**. Official Hack2skill support explicitly confirmed these conditions by email on 2026-11-02, including that the product must address a real challenge and a new working deliverable is required. No company registration necessary for solo participation.

## Chosen track — proposed, not built

**[Population Health] Closing the Screening-to-Care Loop**.

MedScale today is a public Apache-2.0 Rust/Tauri local-first engineering beta with bounded FHIR R4 workflows, synthetic fixtures, evidence attribution/policy and cross-platform QA. It **does not** already provide a clinically validated screening/referral closure intervention, population-health programme, clinic referral integration or real-user rollout.

**New proposed entry concept: MedScale Care Loop — a synthetic, fail-closed FHIR-based referral-follow-up simulator and independent evidence explorer.**

Workflow for a synthetic screened person: screening record -> clinician-reviewed referral recommendation -> booked follow-up status -> attended/not attended -> result documented -> loop closed, with evidence lineage and a non-actionable overdue queue for hypothetical missed follow-ups. No new clinical AI diagnosis, predictive risk model, or autonomous patient communication. Every suggested step has manual human confirmation and a provenance pointer; missing referral/result evidence is explicitly flagged as **unknown** rather than automatically complete.

Potential FHIR R4 objects: Person/Patient (synthetic), Observation (simulated screening), ServiceRequest, Appointment, Encounter, Task, Provenance and AuditEvent. Check public FHIR R4 definitions and MedScale's current supported subset before implementing; do not assert full FHIR conformance. All identifiers synthetic. Prefer a deterministic CLI state machine and auditable transitions in Core first, then a compact Windows desktop UI.

## Build slices and evidence gates, due before 20 November

1. **Scope and contracts** — select one prevention/screening example defined wholly by synthetic cases; derive a strict minimal state machine `screened -> reviewed -> referred -> appointment_known -> attended -> result_received -> closed` with `blocked/unknown/overdue` states. Document each event's permitted actor and required provenance.
2. **Generator and negative fixtures** — produce reproducible FHIR-like synthetic bundles for normal completed care loops, missing referral, duplicated events, cancelled appointments, outdated encounters, contradictory evidence, missing patient consent and delayed follow-up. No real persons or hospital credentials.
3. **Core implementation** — parse fixtures, reject invalid cross-patient linkage, forbid state escalation without required evidence, preserve evidence IDs, and never send reminders/communications automatically.
4. **Desktop demonstration** — read-only synthetic case overview, per-person timeline, evidence/source drill-down, safe human approval for a *simulated* referral action, export a redacted demonstration report. Clear labels: **Research/Simulation only — not for patient care**.
5. **Qualification** — deterministic tests on every legal/illegal transition; document false-closure and missed-overdue cases; run cross-platform CI and scripted end-to-end desktop demo; independent code/security reviewer; only call PASS when checks have actually executed.
6. **Submission packet** — source ref to actual running code, reproducible build/run command, at least 90-second nonclinical demo if programme allows, architecture diagram, threat model, sample synthetic fixtures, measured test results, risk register and 2-minute oral explanation. If any work is unfinished, say so. Submit via official challenge dashboard by Nov 20 and preserve confirmed acceptance proof.

## Registration state and confidentiality

- The official Hack2skill Vivaldi session was already authenticated to the applicant account. The registration page was opened at `/event/futurehealthchallenge/registration` and the form fields for account name, LinkedIn, owner role, independent affiliation, correct country (Saudi Arabia), state (Asir), city (Abha), age eligibility data and the challenge statement were filled/selected in the browser.
- **A legitimate phone-field validation obstacle remained**: the international telephone widget initially defaulted to India and was corrected to the Saudi country-code selector, but the mobile entry did not verify. **No Register Now submission was confirmed**.
- The two required agreement/communications consent checkboxes and final submission need verification; do not make a false application receipt claim. Browser field state may disappear across reloads; never assume persistence.
- **Privacy:** Do not commit the applicant's mobile number, date of birth, OAuth tokens, cookies, authentication data, or any ID/document images into the repo or PR. Use the official website only for those fields.
- For support, the official organisers' public email is `support+fhdic@hack2skill.com`. The user should not be asked to repeat identity/contact data already provided in prior conversations.

## Required distinction

A submitted **registration** would not be the final **prototype submission**, selection, a prize or a health-system pilot. The technical prototype, evidence and final portal receipt must all be independently verified. No patient or clinical claims without explicit independent evidence.
