# MedScale Beta: first-run status

This historical path points to the [maintained MedScale Beta first-run guide](../BETA_FIRST_RUN.md). Follow that guide for platform requirements, verifying a source-bound installer artifact, protecting existing vaults, opening a sample workspace, and optional local AudioFlow transcription. The [external qualification gates](../qualification/EXTERNAL_GATES.md) are the authoritative release boundary.

**No newly signed or notarized public Beta installer has been authorized or published.** The [download page](https://medscale.si/download/) describes current availability; it is **not** an instruction to download an EXE/DMG that has not been released. GitHub Actions artifacts are unsigned, temporary engineering candidates, not equivalent to a public GitHub Release. Do not install an unverified binary, disable operating-system security, or replace an existing installation or vault without an authorized device-specific qualification process.

MedScale can open **synthetic health workspaces** and, on qualified engineering builds, locally transcribe **deliberately chosen, lawfully sourced nonpatient WAV recordings** using explicitly consented and verified Whisper models. No model weights are bundled in the app. Patient records, PHI, clinical decisions and production use remain unauthorized. Never upload raw recordings, transcripts, identifiable information or private vault content to public issue reports.

The first-run sample workspace is separate from the optional Audio workflow. For current instructions to check model disk budget, approve its exact source, import PCM WAV, and inspect retention, see the [desktop guide](../../apps/desktop-tauri/README.md). No new public distribution or patient-data permission is implied by these instructions.
