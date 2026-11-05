# Hugging Face marketing source

This folder is the version-controlled source of the MedScale Hugging Face product presentation.

- `space/`: Static landing page and metadata for `https://huggingface.co/spaces/MedScale/MedScale`
- `organization/`: Public organization card for `https://huggingface.co/MedScaleAI` through the `MedScaleAI/README` Space
- `PROFILE-FIELDS.md`: Suggested UI profile fields and official logo, not changed by repository publication
- `QA.md`: Privacy, accuracy and release qualification checklist
- `publish.py`: Explicit authenticated publisher for the two Spaces, not a CI auto-deployment

Preview and validate locally before publishing. No paid compute or third-party JavaScript is used. The design takes broad inspiration from effective open-source research hubs and healthcare product websites, but uses original MedScale language and assets. There is no affiliation with OpenMed or Abridge.

**Never confuse a static marketing Space with the real desktop application or a clinical deployment.** Synthetic-only beta status is prominent and every capability should continue to be checked against MedScale qualification documentation.

Run:

```bash
python marketing/huggingface/publish.py --dry-run
# After explicitly authenticating the official @MedScale Hugging Face account with write access:
python marketing/huggingface/publish.py
```

Published URLs (after write authorization):

- https://huggingface.co/spaces/MedScale/MedScale
- https://huggingface.co/MedScaleAI

Do not change the SafeOCR demo, dataset, other published research, or MedScale installer artifacts during this marketing publication.
