# MedScale Beta first-run qualification — engineering evidence

Date: 2026-10-23. Protected main base: `c7513ea2bf7b38d89f4104025036ca181dc927eb`. Synthetic-only.

## Existing downloadable engineering preview
- Public release tag `engineering-preview-19d419dd7b15`, source `19d419dd7b157b00630a51e84f4c7e58374128b0`; **different from current main**.
- Windows EXE size: 13,872,773 bytes, SHA-256 `89c6fd754d45bd0be7c9d71463828fe57756f933f49d6ff9348486095d64be6c`. SHA matches published `SHA256SUMS` and provenance `package-manifest.json`.
- Inspected public Windows provenance ZIP. `installer-qualification.json` from GitHub-hosted disposable runner: **INSTALL=PASS, LAUNCH=PASS, UNINSTALL=PASS**, 20 seconds process active, `msedgewebview2.exe` child and `MedScale` window detected. **SIGNING=NOT_GRANTED**.
- Program files removed on runner uninstall; user WebView profile remains by design and must not be misrepresented as wiped.
- The authorized Windows device had WebView2 Runtime 156 installed. This is a prerequisite check only, not a personal-PC install pass.
- No installer was run or removed on a founder workstation: `qualify-tauri-installer-windows.ps1` forbids non-disposable use.
- macOS DMG is published, but direct Gatekeeper / human first-use qualification was **not run** in this work. Remote Mac did not respond. Do not infer PASS.

## New first-run user journey (not included in the published EXE/DMG)
- Welcome page's **primary** action now invokes `workspace_open_synthetic` directly. Advanced encrypted-vault creation/unlock is secondary.
- First-run message explicitly says two fictional subjects, Beta/synthetic-only, and no account/passphrase/payment.
- Responsive layout covers compact widths.
- Local frontend `npm test`: **22/22 PASS**; `npm run build`: **PASS**.
- Headless Edge browser `npm run test:first-run`: **4/4 PASS** at 375/768/1280 widths plus vault opt-in. Tests verify one synthetic command, no vault creation, no external browser requests and no unhandled errors.
- **Important distinction:** that browser suite SIMULATES Tauri IPC and is not native installer qualification. Existing Rust `every_surface_runs_against_real_core` separately verifies offline synthetic ingest and two subjects on real Core.
- New GitHub Actions workflow reruns the lightweight first-run suite on hosted Windows, recording report artifacts without additional paid services.
- The first CI run identified new RustSec unsoundness advisory `RUSTSEC-2026-0386` in nested Tauri `tokio 1.53.1`. Fixed the single affected lock entry to `tokio 1.53.3`, with its registry checksum updated. Locally verified `cargo metadata --locked` and `cargo tree --locked` resolve `tokio v1.53.3`. The complete scoped `cargo-deny` job must still pass on the updated PR head.

## Release and privacy gates
- No Stable/v1.0, code signing, notarization, release publication or clinical readiness in this change.
- Do not claim the current public installer contains this new first-run UI. A future tagged Beta must be built and verified independently.
- Native user-interaction, OS protection dialogs, network privacy, screen reader and physical keyboard qualification remain external follow-up gates; **no real PHI** is authorized.
