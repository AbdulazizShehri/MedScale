# MedScale — OSF 2026/27 introduction-video provenance

Status: **video uploaded to an Unlisted MedScale YouTube channel, official grant application submitted 2026-11-02**. The watch link is stored with the official Foundation form and should **not** be placed in this public GitHub PR because Unlisted links are accessible to anyone who possesses the URL.

## Actual artefact

- Container: MP4, H.264/AAC, 1280x720, around 74 seconds (below 90-second call limit).
- Channel: MedScale, created under the authorized applicant Google account.
- Visuals: six original captioned slides created locally with Pillow; dark-teal backgrounds and no borrowed copyrighted illustrations.
- Narration: Apple macOS built-in `say` / Samantha voice. This is computer-generated TTS, **not the applicant's real recorded voice**, and was disclosed on the video, in its YouTube description, and in the grant AI-use field.
- Encoding: free local ffmpeg, `libx264`, 20 fps, AAC; no paid cloud inferencing or generated synthetic human video.
- Visibility: **Unlisted**, accessible to reviewers who know the link, not intentionally advertised on the public project site.
- Privacy: only synthetic FHIR discussion and public repo information; **no actual patient data or live clinical recording**.
- Video claim: the reference monitor, witnessed receipts, independent offline verifier, and adversarial experiments are planned **NEW research**, not functioning MedScale Beta features.
- Official YouTube Studio returned *Upload complete*, *Checks complete: No issues found*, and *All changes saved* after selecting Unlisted.

## On-screen slide sequence

1. **MedScale:** Open health AI, built locally; Open Science Foundation privacy research proposal.
2. **What exists today:** Rust Core, Tauri desktop, FHIR R4 workflows. Engineering beta on synthetic and licensed nonpatient material; no clinical validation.
3. **Research question:** Can one person verify which agent reads and exports were authorized? Falsifiable tests for a synthetic digital health twin.
4. **Proposed NEW research:** Policy gateway, witnessed receipts, offline verifier. Missing event, forged event, replay and bypass tests. These proofs do not exist yet.
5. **Nine months / CHF 20,000:** Investigator research and isolated compute. Public commits and reproducible tests; report failures openly.
6. **What it can and cannot prove:** Distinguish witnessed vs. unwitnessed events; no claim to perfect confidentiality or clinical readiness. Link to the public project source.

## Original synthetic narration script

> MedScale is an Apache licensed, local first health AI engineering beta. We already have a Rust core, a desktop app, and bounded F H I R interchange. Our current engineering tests use synthetic health records and licensed non-patient material. We do not claim clinical validation or patient deployment. I am proposing new, focused research into one question. Can a person independently verify whether an A I agent accessed or exported only the records it was authorised to use? The new work would implement a policy gateway, witnessed access receipts, an offline verifier, and adversarial tests for missing, replayed, or forged events. These features are a research proposal, not existing privacy proofs. Over nine months we would publish code and reproducible tests from the first commit, and report failures as clearly as successful cases. The funding request is twenty thousand Swiss francs for researcher effort and isolated compute tests. The goal is not to promise perfect privacy. It is to reveal which accesses were witnessed and which could have bypassed our monitor. The current source and engineering evidence are public on the MedScale GitHub repository.

## Reproduction notes

Use the original six slide texts above and only public repository evidence. Render each slide as a static PNG with local fonts, synthesize voiceover using the macOS `say` command (`say -v Samantha -r 165 -o narration.aiff`), and compose with `ffmpeg` to under 90 seconds, preserving the TTS disclosure on screen and in YouTube description. Use `ffprobe` to verify runtime and avoid clipping the narration. Public media must never imply the new privacy verifier is already built. All temporary local working files used for upload should be cleaned after the upload and external URL are independently confirmed.

## Foundation submission evidence

The official application form showed **APPLICATION RECEIVED**, reference **OP-DFF93223BF6B**, 2026-11-02T20:36:36.657Z. The provider's receipt email confirmed the same reference and project title. This is an application receipt, not an award.

Official call: https://opening.science/apply/health-twin-privacy-2026/terms/
