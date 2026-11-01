# Spec 107: Beta transcript hand-off and in-app defect reporting

**Status:** `RESOLVED` after Goldfish review (EGM design, zoom-out, codebase-details and implementation-readiness reviewers, fresh context, 2026-11-02).
**Depends on:** Spec 020/021 (export disclosure), Spec 081/106 (AudioFlow transcripts), Spec 096 (named commands, narrow capabilities, `navigation_policy.rs`).
**Issues:** #302 journeys 14 and 20, #317.

## Problem

- **Journey 14 ("view, copy and export the transcript").** Transcripts can be viewed, corrected and copied (Copy transcript, PR #399: `transcript-text.ts`, write-only Web Clipboard API on an explicit click). They cannot be saved to a file.
- **Journey 20 ("report defects").** `SUPPORT.md` and the issue templates exist, but the app offers no route to them:
  - `navigation_policy.rs:1-11` admits only the app origins.
  - `main.rs` registers only `tauri_plugin_dialog`.
  - The capability set grants only `dialog:allow-open` (`capabilities/default.json:83`, enforced by `ipc.test.ts:84-88`).

The desktop never writes a file the user chooses: `exports_subject` (`cmd_core.rs:264-281`) shows its result and does not write it to disk. The draft called the app "closed", which was wrong: it downloads models over HTTPS after explicit consent (`ureq`, `audio_model_manager.rs`).

## Decision D1: no new transcript export code in Beta

Copy transcript, as shipped, is the Beta hand-off. File export and a disclosure-recorded copy are deferred.

Why (review findings, all verified against source):

1. **Core has no general disclosure append.**
   - `CliSession` exposes only `export_subject_fhir` and `disclosures` (`cli_session.rs:466-500`).
   - `AppendDisclosure` is reachable only from `workflow.rs:497-510`, and it trusts the refs and digest the caller supplies (`facade.rs:2452-2491`).
   - An auditable copy would therefore need a new Core capability, request and response types, and a handler. That is a change to the authority layer, not an additive UI change.
2. **The record would be false for real recordings.** Both disclosure paths hard-code `synthetic_only: true` (`facade.rs:2393`, `facade.rs:2470`), while `audio_import_wav` accepts real licensed WAVs (`cmd_ops.rs:1139`).
3. **The record would not cover every copy path.**
   - The transcript text is already on screen and in the webview (`Operations.tsx:489-498`), so selecting it manually and pressing Ctrl+C bypasses any record.
   - A clipboard write can fail after an append-only record has already been written.
4. **"Accepted" is not a contract state.** `TranscriptState` is `complete | partial | untranscribed` (`audio.rs:307-319`). Blocking copy of "unaccepted" transcripts would block every copy.

**Deferred design if file export is later required (alternative B):**
- Core renders the canonical text and computes its digest in one request.
- `synthetic_only` is derived from the source's provenance.
- Rust owns the save dialog, so the webview never supplies a path.
- The write reuses the refuse-overwrite plus `fs::rename` pattern in `audio_model_manager.rs:592-612`.

**Product decision required.** Whether clipboard copy satisfies "export where supported" for Beta is a founder product decision. Until it is made, `docs/beta/QA_MATRIX.md` row 14 states "view + copy; file export not supported in Beta".

## Decision D2: copy-only "Report a problem" in About

The About page gets a new section containing:
- The issue-chooser URL as selectable text, with a **Copy link** button.
- A visible diagnostics block, with a **Copy diagnostics** button. It is shown before copying, so users see exactly what they would paste.
- The bolded data rule from `SUPPORT.md`, verbatim. A test keeps both the URL and the rule in sync with `SUPPORT.md`.

The diagnostics block is built in TypeScript (`support.ts`) only from fields `about_info` already returns: version, build profile, target, shell, local-only and network default-deny. No Rust change, dependency or capability is added. The network value reads "default deny (consented model downloads excepted)", because "default deny" alone would overclaim.

**Why copy-only:** opening the browser needs `tauri-plugin-opener` or `shell`. That is a new dependency, subject to the `tauri dependency policy` and cargo-deny gates, plus a new external-launch permission. A hard-coded opener URL remains a valid later option.

## Requirements

- **R107-01** `support.ts` exports `SUPPORT_ISSUES_URL`, `SUPPORT_DATA_RULE` and the pure function `supportDiagnostics(about)`.
  - The output has exactly these keys, one per line: `MedScale version`, `Build profile`, `Target`, `Shell`, `Processing`, `Network`.
  - Values are stripped of line breaks and capped at 120 characters.
- **R107-02** About renders the section. It adds no IPC command, capability, plugin, navigation rule or network request.
- **R107-03** `permission_tests.rs` refuses `plugin:opener|open_url`, `plugin:opener|open_path` and `plugin:shell|open`, in addition to the existing refusals.
- **R107-04** A copy failure shows "Copy failed. Select the text above and copy it manually." and changes nothing else.

## Tests

- **TypeScript** (`support.test.ts`):
  - The URL and the data rule are present in `SUPPORT.md`.
  - The diagnostic key set is exactly the allowlist.
  - For representative input, values contain no `/`, `\`, `@` or drive prefix.
  - Line-break injection cannot add keys.
- **Rust:** `generic_privilege_and_plugin_commands_are_refused` covers the opener and shell-open commands.
- **Native Windows E2E** (`about-doctor`):
  - The support address and the Copy diagnostics button are present.
  - Copy link is clicked, and the data rule is visible.
  - Exactly one WebView window remains, so nothing navigated or opened.
  - Clipboard contents are not read.

## Risks accepted for Beta (risk register)

| Risk | Evidence | Status |
|---|---|---|
| Copied transcripts may persist in Windows clipboard history or the cloud clipboard (both off by default). The Web Clipboard API cannot set exclusion formats. | Design review finding 4 | Accepted for Beta, documented |
| Clipboard **read** denial relies on WebView2/WKWebView defaults; MedScale installs no permission handler. | Implementation-readiness M7 | Open; not enforced by MedScale |
| A user could paste a transcript into a public issue. | Design review finding 16 | Mitigated by showing the `SUPPORT.md` data rule beside the copy actions |

## Rollback

D2 is additive: UI, one TypeScript module and test entries. Reverting restores today's behaviour. No stored data or capability changes.
