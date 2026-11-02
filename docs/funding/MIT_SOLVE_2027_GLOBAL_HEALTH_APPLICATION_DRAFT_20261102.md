# MIT Solve 2027 Global Health Challenge — MedScale individual prototype application packet

**2026-11-02 checkpoint: DRAFT / NOT SUBMITTED.** A help@solve.mit.edu inquiry received a general link to eligibility details and a quiz; this is not a review decision. Application must be made by an authenticated individual on https://solve.mit.edu/challenges/2027-global-health-challenge/start. Deadline **Friday 2026-11-06 12:00 EST (20:00 Asia/Riyadh)**, per the official call. Do not confuse the earlier email inquiry with formal submission.

## Applicant, project and truthful stage

- Lead: Abdulaziz M. Alshehri, MPH, University of Melbourne graduate, independent Saudi-based developer.
- Team: individual application; **no** company or nonprofit incorporation, current university affiliation, grant award, clinical users, financial traction or pilots claimed.
- Project: **MedScale — Open, Local-First Health AI and FHIR Interoperability**.
- Website: https://medscale.si/ ; source: https://github.com/AbdulazizShehri/MedScale ; license Apache-2.0.
- Choose **Prototype**, *not* Pilot/Growth. Existing public-source Rust/Tauri/Core engineering beta and synthetic QA exist. Current version is nonclinical and not authorized for patient data.
- Theme: digital tools supporting frontline healthcare workers, information portability and reduction of administrative burden, **conditional on later clinical research, field validation and governance**.
- The 2027 challenge accepts prototypes and individual applicants; selection is competitive and technical/human-centered impact evidence is weak until real field work occurs. No grant amount should be described as guaranteed.

## Application narrative — candidate answers for personal review

### One-line description

MedScale is an open-source, local-first toolkit researching how health workers can use accountable AI-assisted documentation and interoperable FHIR data without mandatory cloud subscriptions, starting with synthetic-only evaluation.

### The problem and the community we hope to serve

Frontline care settings often face fragmented information, complex documentation workflows and constraints around paid cloud software. As a consequence, products designed around subscriptions and centralized data handling may be difficult to adapt to low-resource, privacy-sensitive environments. **We have not measured these specific burdens with a MedScale pilot or carried out interviews that would substantiate a target-population prevalence claim.** Our first community research must test whether these assumptions hold, where and for whom.

### What exists now versus what the award would unlock

**Exists:** Apache-2.0 Rust Core, Tauri desktop, opt-in local Whisper transcription with qualified nonpatient audio, bounded FHIR R4 interchange, provenance/audit surfaces, encrypted local workspaces, CI and reproducible nonclinical synthetic fixtures. Platform is at engineering beta; no demonstrated patient benefit, clinical validation or live deployments.

**Proposed:** Co-design with actual care workers *before* shaping a clinical workflow, formally review governance, privacy, accessibility and safety risks, then run a consented and approved evaluation in a representative setting where allowable. Evaluate task completion, documentation time, error/correction rates, coverage, provenance and language accessibility. **No real-patient workflow until the appropriate medical, legal, security and ethics approvals exist.** A nonclinical user study may precede any clinical evaluation.

### What makes the approach different?

Local-first control and transparent, independently inspectable source code can make vendor dependence less central. MedScale intentionally exposes uncertainty, attribution and policy constraints, rather than claiming the model knows something it does not. The hypothesis that these engineering choices reduce health-worker burden while maintaining safety **has not yet been validated in practice**. Do not claim to have replaced Abridge, to have full EHR integrations, or to make commercial closed APIs freely available.

### Impact measurement / falsifiable milestones

1. **0–3 months:** independent technical reproducibility by at least two external testers; synthetic FHIR interchange/traceability benchmark with published failure cases; accessibility/privacy and stakeholder interview plan. Report actual denominators and missing data.
2. **3–6 months:** recruit real-world end-user advisers where feasible, with documented consent and no patient data in engineering recordings; document needs in multiple languages and tasks. Recruitment is planned, **not confirmed**.
3. **6–12 months:** only after partner approval and safety review, evaluate a narrowly scoped controlled workflow or simulation. Pre-register primary metrics: time per task, completeness, source-linked error rate, false assurance, user correction effort and workflow satisfaction. Report negative outcomes and uncertainty.
4. **Guardrails:** no claims of clinical utility from synthetic tests; explicit no-go for patient processing until qualifications and approvals; no identifiable medical records in an open repo.

### Sustainability and accessibility

The intended product remains free and Apache-2.0, with local use not requiring payment to proprietary cloud services. A community of independent testers, university partners and maintainers is the desired sustainability pathway; those relationships **must not be described as existing without evidence**. Grant funding would support open evidence, independent QA, community co-design and accessibility. It is not an investment in an already commercialized company.

### Team and why this applicant

The individual applicant holds an MPH from the University of Melbourne with a health policy/digital-health background and currently maintains the MedScale open-source repository. The technical repository is inspectable. Do not claim an active university appointment, medical license, hospital affiliation, patient adoption or multi-person founding team.

### Funding and support use (not a formal submitted budget)

If selected, request support for an external safety and reproducibility review, community recruitment/design, cross-platform testing and accessibly documented evidence. Provider states typical Solver teams receive **an average** of approximately USD 40,000, **not a guaranteed individual award**. No double-charge of OSF's distinct verifiable digital-twin privacy research or FUTO's possible equipment request.

## Mandatory authenticated actions that remain

1. Complete official eligibility quiz: https://3bw7v.share.hsforms.com/2GVVsqiiVT8aVh2gnGK6zCg ; record result, not assumed eligibility.
2. Login/create an MIT Solve individual account: https://solve.mit.edu/login ; use account owner identity, do not share credentials in repo.
3. Open challenge and **Start Solution**: https://solve.mit.edu/challenges/2027-global-health-challenge/start .
4. Use applicant's own answers and authentic team/community data for required fields; fill optional questions where evidence permits. Verify prototype status. Attach only media actually created.
5. Review and click **Submit** before the deadline; record confirmation email/reference. An emailed enquiry to Solve or an auto-saved profile **does not count**.
6. If selected, disclose any overlapping grants and confirm compliance before incurring costs.

## Evidence and official sources

- Challenge: https://solve.mit.edu/challenges/2027-global-health-challenge
- How to apply: https://solve.mit.edu/how-to-apply
- Applicant stage definitions and individual application support: both links above.
- Programme outcome status: **no submission, acceptance or award is known** as of this update.
