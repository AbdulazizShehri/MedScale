# MedScale Beta 0.1.0 — safe first-run and synthetic QA guide

> **Engineering Beta only. Synthetic fixtures only.** MedScale is free and local-first, but is **not** a clinically validated medical device, approved for real patient information, or authorized for diagnostic or treatment decisions. Do not enter patient identifiers, protected health information, passwords, or other confidential content. This is not a public release announcement or a download link.

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
5. Open **Audio** and use **Import synthetic tone sample** for a basic import check. This does **not** validate microphone capture, transcription accuracy, multilingual ASR or recording of real speech.
6. Open **Exports** with a synthetic fixture and read the displayed FHIR conformance/authority disclosure. Review **Audit Trail** for the action. Do not transfer the export into a live healthcare system.
7. Visit **Settings**, **About**, and the command palette/theme switch. Close and reopen MedScale; confirm the synthetic project/session information expected by the UI is still available. A privacy or startup warning should not be dismissed as a successful test.

**Offline/local-first boundary:** The packaged desktop uses a local Core, bundled UI assets, a restricted window capability and default-deny network paths. This does **not** prove every integration, external model workflow, or OS diagnostic is permanently offline. Do not equate “local-first” with a blanket independent no-egress certification.

## Troubleshooting without risking data

| Symptom | Safe action |
|---|---|
| Windows launches no window / “could not start” | Check that WebView2 Evergreen is installed by Windows/Microsoft-supported means. Preserve the workspace; do not delete the profile or vault as an initial step. |
| An unexpected extra window, console flash, or disappearing dialog | Record the navigation route (for example Synthetic → Settings), OS version, Beta build/commit if known, and whether it reproduces. Avoid sharing private screen contents. |
| macOS says the app cannot be opened/verified | Confirm **ARM64**, the SHA-256 and the source first. Follow only system-supported *per-app* trust controls when allowed by local policy; do not disable Gatekeeper for the machine. |
| An encrypted workspace appears empty or locked | Stop; check you opened the expected workspace, and use your own securely stored unlock/recovery material. Do not re-create or reset a vault to “fix” it. |
| A model is listed but cannot run | A catalog row is **metadata**. Check whether a local Pack has been admitted; do not assume model weights were bundled or downloaded. |
| Audio imported but did not transcribe | Only synthetic-tone import was qualified by the Beta desktop E2E. Accuracy and microphone/ASR claims require separate tests. |
| Installation or export hash differs | Do not execute or distribute the artifact. Report expected/actual digests without attaching private data. |

## Evidence and issue reports

For reproducible issues, record: **Beta version**, exact **source SHA** if known, OS/architecture, the screen and synthetic-only sequence of clicks, expected behavior, actual behavior, repeat frequency, and a sanitized screenshot if safe. State whether you used a synthetic workspace; never upload vault databases, patient records, passphrases, tokens, or recovery codes. Use [GitHub Issues](https://github.com/AbdulazizShehri/MedScale/issues) for sanitized reports.

For the current engineering qualification, see [Beta installer staging #297](https://github.com/AbdulazizShehri/MedScale/issues/297), [Product QA matrix #302](https://github.com/AbdulazizShehri/MedScale/issues/302), and [external qualification gates](qualification/EXTERNAL_GATES.md). The 22 desktop journeys in [PR #301](https://github.com/AbdulazizShehri/MedScale/pull/301) establish only the specifically tested synthetic behaviors; they do not establish clinical safety, full accessibility, universal model execution, or general-audience distribution readiness.

**Release policy:** No public release, tag, download-link cutover, or Hugging Face distribution switch follows automatically from this guide. Those require a separate founder decision.
