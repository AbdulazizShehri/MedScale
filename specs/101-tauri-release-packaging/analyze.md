# Consistency analysis: Spec 101

Spec 101 adds packaging only. It does not change Core semantics, Tauri commands, capabilities, CSP, plugins, network policy, vault ownership or the synthetic-only scope (FR-001, FR-005, Anti-scope). The bundle identifier is unchanged, so app-local data does not move (FR-007).

**Requirements to tasks:**
- FR-001: T101-01
- FR-002: T101-02
- FR-003: T101-02 and research
- FR-004: T101-04/05
- FR-005: T101-07/08/09
- FR-006: T101-10
- FR-007: T101-01 test
- FR-008: T101-03

**User stories to tasks:**
- US1: T101-01/02/04/05/07
- US2: T101-02/04/05/08
- US3: T101-02/04/05/09
- US4: T101-10/11/12

**Consistency checks:**
- The research table matches the locked CLI version (2.12.0) and the source tag it was read from.
- The plan's per-OS `--bundles` values match the spec's targets.
- The `webviewInstallMode=skip` decision appears identically in the spec, research, config and config test.
- Signing appears only as `NOT_GRANTED`.
- The F096-T01 and F096-T05 carry-forward matches `specs/096-tauri-foundation/findings.md`.

**Open items that block closure, not drafting:**
- Every unchecked checklist line.
- The weak-digest residual for the NSIS tools.
- The dependency on #177 and the scoped review gate on the Tauri chain.

## Findings from exact-head qualification

- **F101-01, Windows launch mechanism (resolved in the harness).** Run `37709847054` at `2cdac53`:
  - On a secondary hidden desktop, the installed app created its "MedScale" window and then exited after 2.2 s with `0x65` (Rust panic), with no WebView2 child.
  - In the same session on the runner's default desktop, it stayed alive for 20 s with `msedgewebview2.exe` and a "MedScale" window.
  - The hidden desktop is therefore not a valid WebView2 qualification environment on GitHub-hosted runners. The verdict now uses the disposable runner's default desktop (no input sent), and the hidden attempt stays recorded as a diagnostic.
- **F101-02, WebView initialization failure panics (open, low).** When WebView2 cannot initialize, the app panics (exit 101) instead of showing a controlled error. No user-facing message is shown. A graceful failure path is repository-owned follow-up work, not a packaging blocker.
- **F101-03, Windows uninstall keeps user app data (by design).**
  - After a silent uninstall, `%LOCALAPPDATA%\org.medscale.desktop.preview\EBWebView` (the WebView2 profile) remains. Program files are removed.
  - User app data is never deleted by the uninstaller, which also protects encrypted vaults.
  - The leftover WebView profile is part of the F096-T05 privacy residual.
- **F101-04, a `vssadmin.exe` child observed during launch (explained).** It comes from the Spec 032 privacy probe `crates/medscale-storage/src/privacy_probes.rs`, which runs `vssadmin list shadows` (read-only) to report whether volume shadow copies exist, as input to the snapshot privacy gate. It is local, makes no network call and changes nothing.
