# MedScale — Open-Source Sponsorship Brief

**Status:** Funding proposal (2 November 2026). **Not** a grant award, sponsor commitment, partnership, or clinical qualification.

- Project: [MedScale](https://github.com/AbdulazizShehri/MedScale)
- Public website: [medscale.si](https://medscale.si/)
- Maintainer: Abdulaziz M. Alshehri, MPH, University of Melbourne graduate; independent open-source maintainer based in Saudi Arabia
- License: Apache-2.0 for first-party source; bundled third-party assets retain their own terms
- Product stage: synthetic/nonpatient engineering beta, **not** clinically validated or authorized for real patient records
- Delivery: local-first Rust Core and Tauri desktop; bounded FHIR R4 interchange; provenance, policy checks and explicit AI abstention
- Current source-backed qualification and release limits: [External gates](../qualification/EXTERNAL_GATES.md), [README](../../README.md), [governance](../../GOVERNANCE.md), [security policy](../../SECURITY.md)

## Why sponsorship matters

MedScale works toward inspectable, reproducible digital-health software without mandatory cloud accounts. Its trusted Core records source provenance, validation decisions and policy gates, while desktop interfaces display those results. The project is building public, synthetic-only test cases for interoperability, auditability and fail-closed handling.

Continued development requires physical cross-platform devices, appropriate local compute for nonclinical AI tests, reliable storage and reproducible evidence. Sponsor funds would support these **engineering resources** and narrowly scoped open, verifiable deliverables—not medical deployment, private patient-data acquisition, clinical efficacy claims, commercial exclusivity, or preferential security treatment.

## Proposed sponsorship request

- **Primary request:** **US$15,000 one-time**, or equivalent in-kind local workstation/test equipment or restricted grant funding.
- **Smaller support:** US$5,000 staged contribution for a documented subset of the test and infrastructure milestones.
- **Expanded proposal:** up to US$25,000 where a funder explicitly supports additional security and reproducibility work, with a revised itemized proposal agreed first.
- **Nature:** Request only. No program approval, credit award, clinical partner, fiscal host, or tax-deductible status is represented.

### Illustrative US$15,000 equipment and project budget

| Category | Proposed USD | Intended purpose |
| --- | ---: | --- |
| Local AI / Rust development workstation | 9,500 | High-memory CPU/GPU engineering, local synthetic benchmark runs and reproducible builds |
| Windows/macOS native QA equipment and peripherals | 2,500 | Independent OS/device verification, peripherals and accessibility testing |
| Encrypted storage, backup and power protection | 1,000 | Reproducibility artifacts, backups and dependable test infrastructure |
| Targeted independent security/accessibility test support | 1,000 | Source-backed negative tests, review and accessible UX checks |
| Public engineering documentation and benchmark packaging | 1,000 | Test descriptions, fixture packaging, run instructions, limitations and maintenance |
| **Total** | **15,000** | **Planning estimate, not a supplier quote** |

**Budget controls:** No purchases or financial commitments are authorized by this document. Actual supplier quotes, eligible spending, tax treatment, shipping costs, and any funder restrictions must be checked before accepting funds or making purchases. Cost allocations may be revised with the sponsor's written agreement; restricted grants will not be repurposed.

## Six-month, testable deliverables (proposed)

1. **Months 1–2 — baselines and procurement:** publish a dated, synthetic-only benchmark manifest, a validated hardware bill of materials/quotes, and reproducible build/test instructions with OS coverage stated exactly.
2. **Months 2–4 — evidence and failure modes:** add at least 50 new or substantially expanded **synthetic** provenance-integrity adversarial test cases (missing, stale, conflicting, partial and tampered source attribution). Publish the test source, coverage scope, and pass/fail results; do not imply clinical accuracy.
3. **Months 4–6 — portable qualification:** publish independently rerunnable Windows/macOS Rust/Tauri/FHIR QA artifacts with a plain-language limitation register, accessibility checks, SBOM/dependency-security findings and remediation evidence. Linux desktop coverage will be reported only if actually qualified.
4. **Every milestone:** public report of executed versus planned work, source/commit identity, limitations, spending receipts at an appropriate privacy level, and next-step risks. No milestone automatically authorizes a release.

These are **targets contingent on funding, procurement and engineering verification**, not claims that they have already been completed.

## Sponsor packages and suitable pathways

- **Corporate sponsor:** one-time US$15,000 for a documented hardware/reproducibility work package, or cash/in-kind support of a smaller approved subset.
- **Research / open-source foundation:** milestone-restricted funding for synthetic FHIR interoperability, security, local-first development and reproducible scientific software.
- **Developer tooling / hardware partner:** equipment loan/donation, warranty-supported workstation, or explicitly free credits where those services complement—not displace—MedScale's local-first goals.
- **Recurring public support:** GitHub Sponsors **only after** the maintainer's personal sponsor profile is approved and payouts are activated. Do not add a nonfunctional funding button or infer that a public sponsor page is live.

A sponsor may receive appropriately labeled public acknowledgment by agreement. No sponsor receives clinical endorsements, real patient data, privileged source access, control over scientific findings, or exclusive ownership of first-party project code.

## Applicant and engineering facts to validate before each application

- Check current [README](../../README.md) and [external qualification gates](../qualification/EXTERNAL_GATES.md) immediately before sending a pitch; build state and test counts change.
- Verify eligibility by program and jurisdiction. Do **not** claim to be a current University of Melbourne student, a faculty PI, a company, a nonprofit, a registered charity, or an accredited clinical provider.
- Distinguish inquiries and completed submissions; use official portals where a company requires one.
- Provide independent adoption and community traction metrics only when verifiable. Do not invent external users, testimonials, stars, customers, publications or partner approvals.
- Do not share real patient records or private test fixtures in applications or public reports.

## Technical collaboration opportunities

Potential **proposals**, not integrations already built: synthetic FHIR R4 evaluation fixtures compatible with Open Health Stack ecosystems; cross-platform health-data provenance integrity and privacy/security negative tests; offline model admission/abstention auditability; nonpatient Arabic/English accessibility and model limitation reporting.

## Official maintainer contact

For a technical discussion or a scoped sponsorship proposal, use the project [GitHub Discussions](https://github.com/AbdulazizShehri/MedScale/discussions) or the contact channel already listed on the project website. Sensitive financial and identity documents should only be shared through the sponsor's approved private process.

**Last verified source checkpoint for the brief:** main `3338375a4e83ad3b008f78e8b8f612737a75aec1` (2026-11-02); this brief does not change runtime code, beta QA status, licensing, or release authorization.
