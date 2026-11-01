# Hacker Initiative 2026 — MedScale application form working answers

**Status:** PREPARED, NOT SUBMITTED. As of 2026-11-02; use only after the maintainer checks every response against the live application and accepts the reporting terms.

**Official form:** https://hackerinitiative.org/apply-now/ — **official preview:** https://hackerinitiative.org/wp-content/uploads/2026/09/The-Hacker-Initiative-Grant-Application-2026.pdf

**Deadline:** November 9, 2026 at 11:59 PM PST, as written on the 2026 application preview. The programme accepts online form submissions ONLY, not emailed proposals. The grant is USD $5,000–$25,000, with an award potentially smaller than the request.

## Required fields that cannot be completed from public project evidence

| Form questions | Private applicant action |
|---|---|
| 1 Email | Verify official MedScale sender or preferred personal correspondence email |
| 2–3 First and last name | Enter legal applicant name; cannot use public GitHub identity as proof of legal name |
| 4–9 Title and contact address | Enter true physical street address, city, state/province, country and postal code; no P.O. box |
| 10–11 Organization and TIN/EIN | Truthfully select individual or legally registered entity; do not invent company registration or tax ID |
| 13 Official relationship | Enter verified maintainer/founder status and whether the applicant is applying personally |
| 14 Alternative email and phone | Provide applicant-controlled addresses, direct telephone and responsible parties |
| 15 Payee | Specify exactly who would receive a USD check and be responsible for applicable taxes |
| 16 Physical mailing address | Real postal street delivery address, not P.O. box |
| 21 Other funding for the same project | Provide actual awards and pending requests; do not claim 'none' if a separate application is pending |
| 25–26 Reporting and non-discrimination acknowledgements | Read and personally accept or decline the funder's legally relevant terms |
| 27 Source of referral | Respond truthfully; 'independent online search' only if accurate |

**Security:** These fields are deliberately omitted from the public GitHub repository. Never place tax IDs, bank details, full home address or phone number in a public PR.

## Application answers drafted from public MedScale evidence

### 12. Project website

https://github.com/AbdulazizShehri/MedScale

Secondary public site, if still accurate at submission: https://medscale.si/ — independently verify current ownership, contents and availability before adding it.

### 17. Project name

**MedScale — Open-source Provenance and Integrity Testing Toolkit**

### 18. Brief project description (150–300 words)

MedScale is a free, Apache-2.0-licensed, local-first open-source health intelligence project. It combines a trusted Rust core with desktop interfaces to support auditable health-information workflows, transparent evidence provenance, constrained FHIR R4 interchange, and human oversight. It currently operates as an engineering Beta, using synthetic health records and appropriately licensed public nonpatient recordings. It is not clinically validated and is not approved to process real patient data.

We seek support for an openly reusable provenance and integrity-testing toolkit built around MedScale's existing validation architecture. The toolkit would help developers distinguish supported facts from missing, altered, contradictory, or outdated source material before an AI-assisted system presents a convincing but unsupported conclusion.

The proposed project will create synthetic adversarial scenarios, deterministic fixture generation, verification checks, and open documentation explaining each failure mode. Tests will cover missing source identifiers, altered content hashes, conflicting records, stale evidence, unsupported inferences, and incomplete audit trails. Results, including failures and limitations, will be published with reproducible instructions.

The outcome is not a diagnostic tool or an attempt to certify AI for clinical care. It is an educational and security-focused resource that independent developers and researchers can adapt when testing software that makes claims about information sources. The project will remain publicly inspectable, with no real patient records, private credentials, or confidential healthcare data involved.

### 19. Project goals and success measurement

Over a proposed six-month project:

- Publish a threat model and provenance-integrity test protocol describing at least six source-failure classes and clear nonclinical scope limits.
- Produce at least 50 synthetic test scenarios with human-reviewed expected outcomes; test altered/missing/conflicting/stale content and unsupported attribution.
- Add deterministic CI regression tests and publish source-bound receipts so maintainers can reproduce results.
- Publish an open report showing passing, failing, and inconclusive tests and model/tool limitations.
- Share concise developer-facing guidance to help other independent security researchers adapt the tests outside MedScale.

**Success** means independently runnable fixtures, clearly versioned scripts and public evidence of limitations—not unsupported claims of medical accuracy or security certification. Targets are proposed, not yet achieved.

### 20. Community benefit

Independent open-source developers, security researchers, students of computer science, and developers building evidence-oriented tools will benefit from a small, reproducible, privacy-first integrity-testing toolkit. Source provenance and auditability are useful in many fields beyond healthcare, including academic research, civic information and open-data systems. Unlike demonstrations based on confidential datasets or unverifiable benchmarks, the proposed fixtures will be synthetic and openly auditable. The public test protocol will explain how to identify unsupported statements and preserve negative findings. Developers can reproduce the same source-conflict and tampering scenarios without needing cloud services or patient information. Community value will be measured by the availability of maintainable public artifacts and independent feedback, not by invented user statistics.

### 21. Amount of funding requested

**USD $15,000** (proposed; funder may award less). The application must disclose any real, overlapping granted or expected funding at the moment of submission. No funds have been confirmed in this workspace.

### 22. Intended use and proposed budget

This is a six-month, developer-research infrastructure budget, not a request for salary, staff training, marketing, production websites, clinical trials or previously incurred debt.

| Expense category | Proposed USD | Use |
|---|---:|---|
| Reproducible benchmark compute and licensed nonpatient fixture processing | 4,500 | Run bounded test matrices and validate deterministic source-integrity cases |
| Isolated security/integrity test infrastructure | 3,000 | Repeatability, test isolation, failure injection and verification |
| Windows/macOS test hardware and measurement equipment | 3,500 | Verify system-level protections without accessing patient data |
| Data-provenance fixtures and versioned research storage | 2,000 | Maintain nonclinical test artifacts with licensing and hash receipts |
| Accessibility and independent reproducibility measurement equipment | 2,000 | Test tool usage and document repeatable observations |
| **Total** | **15,000** | **Prospective; none purchased, invoiced, or approved** |

**Important:** Eligible expense classification has not been pre-approved by Hacker Initiative. Recheck whether equipment, cloud infrastructure and storage are eligible before submission. If not, revise the request and total; do not reclassify prohibited salary costs as expenses.

**Location:** The project is independently maintained in Saudi Arabia, subject to applicant verification. The deliverables are publicly accessible on GitHub.

**Existing evidence:** https://github.com/AbdulazizShehri/MedScale and https://github.com/AbdulazizShehri/MedScale/blob/main/docs/qualification/EXTERNAL_GATES.md

### 23. Category (maximum two)

- **Research Project**
- **Software/Tool Development**

### 24. Other category

Leave blank unless the official form explicitly requires it and the applicant selects Other.

### 25–26. Acceptance declarations

**Not answered on behalf of the applicant.** The form asks for personal acceptance of reporting deadlines and anti-discrimination terms. The applicant must read and select the actual answers.

### 28. Comments (optional)

This is a privacy-preserving, open-source software research project. It will use synthetic or appropriately licensed nonpatient data only. It is not a medical device, clinical validation effort or request for payroll support.

## Submission and reporting checklist

- [ ] Confirm the legal recipient/payee can lawfully receive a USD check from a US charity in Saudi Arabia.
- [ ] Confirm applicant's correct legal name and private mailing address/phone.
- [ ] Read the latest FAQ and all reporting and anti-discrimination statements.
- [ ] Reconcile actual duplicate/other funding applications and any awards.
- [ ] Review and validate each budget line's permissibility.
- [ ] Submit the official Google Form personally or through an authorized account after all required data/agreements are provided.
- [ ] Store only the **nonsecret** application ID/date/URL in the funding tracker.
- [ ] If approved, provide required reports by the funder's deadlines; retain private financial documents off the public repository.

_No application or grant receipt has been obtained at creation time._
