# Windows native package inspection (PR #177 head `5b5643d`)

## Identity

- Source head: `5b5643dd7b3505cf99f0c63977ac11d4c651812a`
- Source tree: `78b58ab5e3af9a62a9166af67bf5f1db49e30a15`
- Root CI: [37545340950](https://github.com/TheHalfMoon/MedScale/actions/runs/37545340950). All six required jobs passed: Rust on Ubuntu, macOS and Windows; perf delivery-plan scale; cargo-deny; supply-chain policy.
- Tauri preview workflow: [37545340868](https://github.com/TheHalfMoon/MedScale/actions/runs/37545340868).
  - **Native build, test and package: PASS** on Ubuntu, macOS and Windows.
  - Windows attempt 1 hit the workflow's 35-minute `timeout-minutes` during "Build native preview" (started 23:15:00Z, stopped 23:50:07Z). The same job was rerun unchanged as attempt 2 and succeeded. The timeout was not a source failure. Attempt 2 took 33.9 minutes (Clippy and tests 13.5, native release build 16.6), leaving about one minute of headroom, so the commit that adds this record raises the native job's `timeout-minutes` from 35 to 60. No other workflow behavior changes.
  - **Dependency policy: FAILED**, only on `advisories` (`glib` 0.18.5 / RUSTSEC-2024-0429, F096-T01). `bans`, `licenses` and `sources` are ok.
- Artifact: `tauri-preview-windows-latest` (id 11455323554, 20,534,808 bytes), downloaded with `gh run download`.
- `build-receipt.json` in this folder is the CI receipt byte-for-byte (CRLF, SHA-256 `19f0b490d0db90f0a662ad2bfd9b8f0eca76037619444efed581b36f4a5824cb`). Its `core_connected: false` field was a stale constant in `scripts/package-tauri-preview.ps1` at this head; the build does connect to the in-process Core. The script is corrected in the commit that adds this record, and the original receipt is kept unedited here.

## Checks executed (Windows 11, 2026-10-07)

| Check | Result |
| --- | --- |
| Package inventory: all `file-inventory.json` entries re-hashed (SHA-256) and size-checked | 1,545 entries, 0 mismatches |
| Executable identity matches the receipt | `medscale-desktop-tauri.exe`, 51,474,944 bytes, SHA-256 `6c600c3c50d1e0e55f857c8f4a68f0732765f2ed92b2902fae69cf466081a35a` |
| Version resource | ProductName `MedScale`, FileDescription `MedScale`, ProductVersion/FileVersion `0.1.0`, CompanyName `medscale` |
| Authenticode signature | `NotSigned`: unsigned engineering package, no signing claimed |
| Launch from the verified package | Main window title `MedScale`; process responding; one host process plus six `msedgewebview2` processes |
| App-local data location | `%LOCALAPPDATA%\org.medscale.desktop.preview` (bundle identifier unchanged by the product rename) |
| Runtime network observation, about 8 s after launch | No connection from the MedScale host process. The WebView2 **browser** process (no `--type`) held 2 established TCP connections to `[2603:1046:c0b:81a::2]:443` (Microsoft address space), plus 1 UDP endpoint |

## Findings

- **WebView2 runtime background traffic.** The application CSP has no remote origin and the frontend makes no remote requests, but the Edge WebView2 runtime process opened outbound HTTPS connections on its own. This is runtime-level traffic outside MedScale's IPC/CSP boundary. It is recorded under F096-T05 (privacy containment is not established). "No runtime network activity" is **not** claimed. Identifying the runtime feature responsible, and whether it can be disabled through supported WebView2 browser arguments, is open work.
- **Interactive testing was interrupted.** While the launched window had focus, keyboard input not issued by this inspection reached it (an encrypted vault was created at 05:08:52 local time from unknown input). Interactive route, keyboard and encrypted-lifecycle checks from this session are therefore **not** recorded as evidence. They must be repeated in a controlled session (dedicated desktop or VM, or with the founder's explicit go-ahead).
- **Test artifact disposition.** The encrypted vault created by that unknown input was not qualification evidence. With the founder's explicit authorization, only that folder was deleted on 2026-10-07:
  - Path: `%LOCALAPPDATA%org.medscale.desktop.previewencrypted-workspace`, created 2026-10-07T02:08:52Z.
  - Its contents were not opened; only entry names were listed.
  - `TEST_ARTIFACT_DISPOSITION=DELETED_NOT_QUALIFICATION_EVIDENCE`.
  - The sibling `encrypted-workspace.desktop-seed.json` manifest was outside the authorization and was left in place. With the vault removed, a later unlock attempt reports an error instead of "no vault".
- **Runtime egress follow-up.** A controlled follow-up on a hidden desktop classified the egress as `MEDSCALE_APP_EGRESS`: no remote application-originated request observed in the performed measurement, and `EMBEDDED_PLATFORM_RUNTIME_EGRESS`: observed. See the F096-T05 investigation in `specs/096-tauri-foundation/findings.md`.
- No native screenshots from this session are committed. The single capture included unrelated desktop content and showed the externally typed input.

## Not established

- Offline startup was not tested: disabling network adapters or firewall rules is a system-settings change outside this inspection's authority.
- Not established: physical keyboard, screen reader, focus behavior, all-route native visual acceptance, macOS or Linux native visual inspection, signing, installer, upgrade or uninstall.
- `RELEASE_READY=false`, `PRIVATE_DATA_READY=false`, `PLATFORM_QUALIFIED=false`.
