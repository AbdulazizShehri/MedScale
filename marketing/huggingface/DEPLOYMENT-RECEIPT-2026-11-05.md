# Hugging Face marketing publication receipt — 2026-11-05

**Status:** LIVE for the two static showcase Spaces. Product release qualification is unchanged; this record is marketing-only.

## Deployment

- Authenticated publisher: Hugging Face user **MedScale**, via the owner's existing local Hugging Face CLI OAuth session on their connected Mac. No TinyFish/browser-session automation was used.
- Main product showcase: https://huggingface.co/spaces/MedScale/MedScale
- Static origin: https://medscale-medscale.static.hf.space/index.html
- Commit: https://huggingface.co/spaces/MedScale/MedScale/commit/2eababbb1aaa6e9475196746fab51a0d0a39a0a9
- Organization card source Space: https://huggingface.co/spaces/MedScaleAI/README
- Static origin: https://medscaleai-readme.static.hf.space/index.html
- Commit: https://huggingface.co/spaces/MedScaleAI/README/commit/33e5bc57fff43d2f5cf9a8c99a26ad966270baec
- Organization home: https://huggingface.co/MedScaleAI
- Both Spaces use `sdk: static`, require no paid GPU hardware, are public, and report `runtime.stage=RUNNING` in the Hub API.

## Actual verification

- Hub connector independently confirms both Space repositories are static and updated 2026-11-05.
- Public Hugging Face pages return HTTP 200.
- Both public static HTML origins return HTTP 200; main page includes MedScale's hero headline, MedScaleAI research links, SafeOCR and synthetic-only disclosure.
- Windows Edge headless actual deployed page: desktop 1440×900 and mobile 390×844 return HTTP 200 with no horizontal overflow, 22 links, and zero uncaught JS errors.
- Main screenshot and logo assets loaded. The secondary Patients screenshot uses lazy loading; after scrolling into view it resolved successfully, natural width 1028 px.
- The existing `MedScaleAI/SafeOCR` and `MedScaleAI/SafeOCR-LabGold` assets were not edited by this deployment.

## Remaining optional profile details

Hugging Face's account and organization header descriptions, avatar, optional homepage/social links remain a separate profile-settings operation, described in `PROFILE-FIELDS.md`. Do not claim those are complete merely because the Space and organization card published.

## Constraints

- No application binaries or real patient records uploaded. No clinical, efficacy, regulatory or independent model qualification claims.
- MedScale remains an unsigned synthetic-data-only engineering beta. Publication of this marketing Space is not authorization to move MedScale's product release gates.
- No affiliation/endorsement from OpenMed or Abridge.
- GitHub PR #439 stays a draft for source review; this deployment is a separate explicit authorized marketing publication, not evidence that the PR was merged.
