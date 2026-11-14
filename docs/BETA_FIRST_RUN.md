# MedScale Beta 0.1.0 — first-run guide

> **Free, local-first Beta engineering candidate.** Synthetic health workspaces and deliberately selected **nonpatient** audio are supported on qualified builds. MedScale has not been validated for diagnosis, treatment or real patient records. Do not enter patient identifiers, PHI, passwords or other confidential material. This document does not authorize public distribution or replace a signed release package.

## Before opening the application

1. Use only a package supplied through a **separately authorized** Beta distribution channel. A GitHub Actions build artifact is an expiring engineering artifact, not a public signed release. Do not use an unknown download link.
2. Confirm the package's platform and architecture: Windows **x64** or macOS **Apple Silicon / ARM64**. An Intel Mac installer, Linux desktop package, signed build, or notarized build has **not** been qualified by the current Beta evidence.
3. Compare the original installer file's SHA-256 against the `installer.sha256` in the matching `tauri-release/<platform>/package-manifest.json`, and confirm `source_sha` corresponds to the intended reviewed commit. The `SHA256SUMS` release set and the SBOM/NOTICE should accompany it. If any digest disagrees, **stop** and report the mismatch; do not open the file.
4. Windows requires the **Microsoft Edge WebView2 Evergreen Runtime**, installed through supported Microsoft/Windows mechanisms. MedScale does not silently download this dependency.
5. Both Beta packages are **unsigned**. SmartScreen or macOS Gatekeeper can block an untrusted build. Never disable SmartScreen, Gatekeeper, antivirus, or organization-wide policy. If your device is managed or you cannot establish the package's provenance, do not proceed. For an explicitly trusted, checksum-verified Beta, use only the OS-provided per-app review/approval UI where policy permits. No global bypass commands are required.

## Protect an existing workspace

- **Do not uninstall your current working Beta merely to follow this guide.** Uninstall behavior and data retention depend on the installer and OS.
- Before a separately approved upgrade, exit MedScale, arrange an independent protected backup of **all vault data, keys/recovery material and the existing installer**, and verify that the backup can be read. Keep the original backup unchanged.
- Never paste recovery codes, vault passphrases, private logs, or actual patient content into a support ticket. Do not delete a vault, WebView profile, or backup while investigating a Beta bug.
- A package manifest proves which software was packaged; it is **not** a backup of personal app data.

## First run: try synthetic data

1. Launch MedScale from its normal application entry. Choose **Try MedScale with sample data**. The first open seeds a synthetic workspace with **two invented subjects**; it must not contain real patient information.
2. Open **Patients** and inspect an invented subject, brief and timeline. Open **Documents**, **Insights**, and **Evidence** to follow what is displayed and where evidence came from. Evidence and proposals are not approved medical records.
3. Open **Projects**, create a test project, then open **Data**, add a **synthetic sample source** and import a snapshot. Use **Analytics** with the generated snapshot binding. Invalid bindings should explain the error instead of silently inventing results.
4. Open **Models** → **Catalog**. The catalog is a metadata index; a listed item **is not a downloaded, installed, approved, or runnable model**. Page forward and backward. On **Try a model**, only select an **admitted local synthetic evaluation Pack** already available to this Beta; its evaluation output remains evidence, not an automatically promoted patient record. Do not infer support for arbitrary external models.
5. Open **Audio** and use **Import synthetic tone sample** for a quick offline fixture check. To transcribe your own lawful **nonpatient** PCM WAV, follow the optional real-audio steps below. Synthetic tone import by itself does not measure microphone capture, real-speech accuracy or all languages.
6. Open **Exports** with a synthetic fixture and read the displayed FHIR conformance/authority disclosure. Review **Audit Trail** for the action. Do not transfer the export into a live healthcare system.
7. Visit **Settings**, **About**, and the command palette/theme switch. Close and reopen MedScale; confirm the synthetic project/session information expected by the UI is still available. A privacy or startup warning should not be dismissed as a successful test.

## Optional: transcribe a nonpatient WAV locally

1. In **Audio**, select a supported Whisper checkpoint. The English-only **Whisper Tiny English** model is a smaller first-run option; choose a **multilingual** model for Arabic. A model shown in the Models catalog is not necessarily installed or runnable.
2. Select **Check disk budget** and review the estimated download and staging space, pinned model source and revision. Only after reviewing these details should you check **Approve Whisper model download** and select **Download, verify and install**. Model weights are **not bundled** into the desktop installer. The explicit download requires network access; local transcription after installation does not.
3. For a verified model already on disk, use **Activate saved model**. An incomplete cache is not overwritten automatically; follow the on-screen error and avoid deleting unrelated workspaces or vaults.
4. Select **Browse WAV…** (or enter an absolute path) to import one **16 kHz, 16-bit PCM WAV** of at most **32 MiB**. Choose the transcription language appropriate to your model, then select **Transcribe** on the imported source. MP3/FLAC and live microphone capture must not be assumed supported by this import control.
5. Review the result locally. In **Stored data**, inspect recording/transcript retention before confirming removal from the **active vault**; existing backups or linked evidence may persist and require separate retention handling. Do not share recordings, transcripts or screenshots containing private content in bug reports.

**What is verified:** A real Windows desktop run completed **26/26** journeys, including user-approved model download, actual LibriSpeech voice transcription and restart ([source-bound run](https://github.com/AbdulazizShehri/MedScale/actions/runs/40575188135)). A separately bundled native macOS WKWebView app passed explicit disk preflight, disabled-until-consent model download, model activation, licensed human WAV import and visible genuine transcription ([source-bound run](https://github.com/AbdulazizShehri/MedScale/actions/runs/40687382970)). These are engineering tests on disposable hosted runners, not assurances about every customer device, a signed/notarized release, clinical accuracy or all Arabic dialects.

**Offline/local-first boundary:** The packaged desktop uses a local Core, bundled UI assets, a restricted window capability and default-deny network paths. Explicitly approved model downloads require network access. This does **not** prove every integration, external model workflow or OS diagnostic is permanently offline. Do not equate “local-first” with a blanket independent no-egress certification.

## Troubleshooting without risking data

| Symptom | Safe action |
|---|---|
| Windows launches no window / “could not start” | Check that WebView2 Evergreen is installed by Windows/Microsoft-supported means. Preserve the workspace; do not delete the profile or vault as an initial step. |
| An unexpected extra window, console flash, or disappearing dialog | Record the navigation route (for example Synthetic → Settings), OS version, Beta build/commit if known, and whether it reproduces. Avoid sharing private screen contents. |
| macOS says the app cannot be opened/verified | Confirm **ARM64**, the SHA-256 and the source first. Follow only system-supported *per-app* trust controls when allowed by local policy; do not disable Gatekeeper for the machine. |
| An encrypted workspace appears empty or locked | Stop; check you opened the expected workspace, and use your own securely stored unlock/recovery material. Do not re-create or reset a vault to “fix” it. |
| A model is listed but cannot run | A catalog row is **metadata**. Check whether a local Pack has been admitted; do not assume model weights were bundled or downloaded. |
| Audio imported but did not transcribe | In **Audio**, verify that the selected Whisper Pack was fully downloaded, verified and activated; that the file is 16 kHz / 16-bit PCM WAV and within the 32 MiB limit; and that the selected language matches the model. Follow the exact on-screen error. Actual native Windows and macOS nonpatient speech workflows have been tested, but microphone capture, clinical accuracy and every customer device have not. |
| Installation or export hash differs | Do not execute or distribute the artifact. Report expected/actual digests without attaching private data. |

## Evidence and issue reports

For reproducible issues, record: **Beta version**, exact **source SHA** if known, OS/architecture, the screen and synthetic-only sequence of clicks, expected behavior, actual behavior, repeat frequency, and a sanitized screenshot if safe. State whether you used a synthetic workspace; never upload vault databases, patient records, passphrases, tokens, or recovery codes. Use [GitHub Issues](https://github.com/AbdulazizShehri/MedScale/issues) for sanitized reports.

For the current engineering qualification, see [Beta installer staging #297](https://github.com/AbdulazizShehri/MedScale/issues/297), [Product QA matrix #302](https://github.com/AbdulazizShehri/MedScale/issues/302), and [external qualification gates](qualification/EXTERNAL_GATES.md). The historical 22 desktop journeys in [PR #301](https://github.com/AbdulazizShehri/MedScale/pull/301) establish their specific synthetic behaviors; newer real-voice Windows and macOS tests are linked above. None establishes clinical safety, full accessibility, universal model execution, signed installers or general-audience distribution readiness.

**Release policy:** No public release, tag, download-link cutover, or Hugging Face distribution switch follows automatically from this guide. Those require a separate founder decision.
