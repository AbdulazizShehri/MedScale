# MedScale intended use (draft)

**DRAFT FOR LEGAL REVIEW. NOT IN EFFECT. NOT LEGAL ADVICE.**

## What MedScale is

MedScale is free, open-source desktop software (Apache-2.0) that runs on the user's own computer. It lets a person organize health records in FHIR format, see which facts each record supports, conflicts or lacks, search their own documents with citations, and run local AI models whose output is recorded as a proposal for a person to review.

## Intended use today

- **Data:** synthetic records bundled with the app, synthetic records the user makes, and licensed recordings that contain no patient information. Real patient data is **not authorized** (`REAL_PHI_AUTHORIZATION = NOT_AUTHORIZED`).
- **Users:** clinicians, informaticists, researchers and developers evaluating the software.
- **Purpose:** evaluation, research on synthetic data, and software development.

## Not intended for

- Diagnosis, treatment, prevention or monitoring of any disease or condition.
- Making, or being the basis for, any clinical decision about a real person.
- Use with real patient records, until the founder issues a separate written authorization and the gates in `docs/beta/REAL_DATA_READINESS.md` are met.
- Emergency or time-critical use.

## How the software supports this

- **Proposals, not decisions:** summaries and agent runs are recorded as proposals until a person accepts them; nothing an AI model produces changes a record on its own.
- **Visible evidence:** each fact shows its source, and conflicts and gaps stay visible rather than being resolved automatically.
- **No scores or predictions:** coverage views count facts; they don't score patients or predict outcomes.
- **Unqualified models:** no model is qualified for any clinical task (`docs/qualification/EXTERNAL_GATES.md`). Appearing in the model catalog is not approval.

## Limits stated to users

- MedScale is not clinically validated.
- Installers are not code-signed by a trusted publisher; the operating system warns before running them.
- **[DECISION]** Final wording of the warning shown in the app and on the website, and where it appears (the founder has asked that the homepage stay free of status lines; a single footer line carries the scope today).
