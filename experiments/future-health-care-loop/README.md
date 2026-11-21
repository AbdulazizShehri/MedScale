# MedScale Care Loop — Future Health synthetic prototype

**Status: standalone, nonclinical research demonstration.** This experiment is NOT MedScale Core, not a connected hospital workflow, and not certified FHIR R4. No patient data, human subjects, clinical decisions, model inference or external network calls.

## Why this exists

The [Future Health Digital Innovation Challenge](https://hack2skill.com/event/futurehealthchallenge/) includes **[Population Health] Closing the Screening-to-Care Loop**. The user of this demonstration can inspect how a positive synthetic screening record is connected to an explicitly human-reviewed task, a referral order, an appointment, a finished encounter, a result and a final human closure task. Every stage requires a matching synthetic-patient reference and an associated recorded reviewer provenance. A missing, cancelled, duplicated, contradictory, or unreviewed stage never produces a false **closed** result.

The existing MedScale beta is a local-first Rust/Tauri health AI system. This Node.js experiment is a separate competition slice. Integrating it with MedScale's trusted Rust Core is **future work**, and would require design and security review.

## Run locally on Windows, macOS or Linux

Requirements: Node.js 20+ only. No `npm install`, API keys, cloud services, healthcare data, or download steps are required for this folder.

From the repository root:

```powershell
node --test experiments/future-health-care-loop/test/*.test.mjs
node experiments/future-health-care-loop/server.mjs
```

Open **http://127.0.0.1:8787** on the same device. The HTTP server binds to loopback only and only allows GET requests for the bundled HTML and two local JavaScript modules. It does not expose any upload, edit, webhook or external-fetch endpoint. Stop using Ctrl+C. JavaScript/CSS may be inspected directly; the demonstration runs in a web browser with no external services.

## Coverage and evidence

The synthetic bundle contains a marked **Patient**, **Observation**, **Task**, **ServiceRequest**, **Appointment**, **Encounter**, follow-up **Observation**, explicit closing **Task**, and seven **Provenance** resources. All identifiers and agent references are fabricated.

Scenarios:

- **Complete evidence**: all seven stages exist, belong to the same synthetic patient and have recorded reviewer provenance; only here is closure true.
- **Missing result**: the chain stops at *attended*.
- **Overdue appointment**: the simulated date passes without an encounter, returns *overdue* with no automatic patient contact.
- **Cancelled visit**: returns *blocked*.
- **Wrong-patient link**: returns *blocked*.
- **Missing provenance**: returns *blocked* even if all underlying records exist.

The Node test suite also covers duplicate IDs, multiple synthetic patients, ambiguous screens, missing review/referral/appointment, unfinished encounter, invalid timestamps and spoofed provenance. All states are deterministic.

## Important limitations

- The fixture is **FHIR R4-style** rather than a complete external FHIR validator or clinical conformance benchmark. Sample synthetic codes are in a project-specific code system, not an endorsed screening/medical terminology.
- Provenance is a **synthetic audit pointer**, not cryptographic proof or independent identity verification. The prototype does not verify signatures or trust levels of practitioners or patient consent, and **must not** be used to certify authentic medical records.
- Event timestamps are evaluated only for synthetic appointment overdue warnings. Comprehensive ordering, timezone semantics, FHIR profile validation, privacy guarantees, correct clinical pathways, and explainability have **not** been established.
- No GUI or model from the current shipping MedScale application is reused here. The standalone web viewer is intentionally isolated from any production patient context.
- The currently running prototype is an engineering scaffold for a competitive evaluation. To claim challenge submission, an actual video demo, independent review, end-to-end qualification, and official registration/submission receipts must be completed and verified.

## Milestones before the challenge prototype deadline (November 20, 2026)

1. Independent Rust Core architecture/security review of the FHIR-style constraints, data-flow and synthetic-only boundary.
2. Validate input profiles against a real, licensed FHIR R4 validator; add adversarial timestamp, linkage and patient-scope cases.
3. Document human factors and accessibility, with no clinical efficacy claims.
4. Run the browser UI and deterministic Node tests on Windows; capture an accurate nonclinical demo.
5. Submit the final working source, test evidence and limitations only after official Hack2skill registration and challenge portal confirmation.

**License:** Apache-2.0, consistent with the parent MedScale repository.

## Verification checkpoint — 2026-11-03

- **30/30 Node tests passed on the user's Windows PC** including 300 deterministic synthetic evidence mutations and three UI logic tests using a fake DOM. The fake DOM tests are NOT end-to-end browser tests.
- The selected FHIR R4-style fixture now contains `Task.intent`, `Encounter.class`, and paired `Appointment.start` / `Appointment.end` to address three mandatory format gaps identified from the official R4 resource specifications. This still does NOT certify full R4 conformance.
- Local loopback HTTP server smoke previously passed. Real-browser visual/click E2E remains **UNVERIFIED**.
- The dedicated GitHub Actions workflow currently runs the original care-loop test file only; newer adversarial and UI tests have been exercised on Windows but **are not yet included in that workflow**. GitHub's separate general Rust CI has an unrelated `async-executor 1.14.0` yanked-crate failure; do not relax the security gate.
- The official Hack2skill registration remains unsubmitted pending successful form persistence and personal acknowledgments. No prize awarded or competition prototype officially submitted.
