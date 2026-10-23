# MedScale Beta: first run (synthetic only)

MedScale's local desktop Beta is free to use. It is **not qualified for patient data, diagnosis, treatment, or clinical decisions**. Use only fictional fixtures.

## Download
Use https://medscale.si/download/ to get the Windows EXE or macOS DMG and verify its SHA-256 against the accompanying GitHub Release `SHA256SUMS`.

- **Windows:** Windows 10/11 requires the Microsoft Edge WebView2 Evergreen Runtime. The installer does not download WebView2. The unsigned Beta may encounter SmartScreen or organization policy restrictions.
- **macOS:** DMG is unsigned/not notarized and may be blocked by Gatekeeper. Do not globally disable OS security.
- **Linux:** Desktop packages are not offered; only Core and CLI are supported.

If installation is blocked, report the observed error or test in a disposable device. Do not weaken organizational security.

## Open your first sample workspace
Launch MedScale from the desktop icon—no terminal or cloud account required.

- **Current published preview** `engineering-preview-19d419dd7b15`: choose **Open synthetic workspace** from the Welcome screen (currently the secondary button).
- **A future Beta containing the first-run improvement:** the primary button becomes **Try MedScale with sample data**, which opens the same example workspace in one click. The encrypted vault remains an advanced opt-in.

Core should import its offline synthetic fixtures, producing **two fictional subjects**. Open **Patients** to see them, then examine **Documents** and **Evidence** to inspect provenance. No passphrase, MedScale account, cloud API, or real medical data is needed.

## Remaining limits
The published artifacts are unqualified engineering builds. CI demonstrates installer install/launch/uninstall on disposable runners, but this is not a human-operated native first-use qualification, OS reputation validation, physical accessibility test, or privacy certification. The browser first-run regression uses simulated IPC while a separate Rust test exercises the real Core and synthetic ingest. The new onboarding change is **not** in current public downloads until a separately verified Beta artifact is published.
