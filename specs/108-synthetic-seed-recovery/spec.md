# Spec 108: Prevent and recover from an incomplete fictional sample workspace

**Status:** `RESOLVED` into an ordered plan after Goldfish review (EGM design, codebase-details and zoom-out reviewers, fresh context, 2026-11-02). Step 1 is implemented with this spec. Steps 2-5 are open.

**Depends on:**
- PR #394 / #398: fail-closed seed preflight and imported-row attestation in `apps/desktop-tauri/src-tauri/src/host.rs`.
- Spec 096: named commands and capabilities.

**Issues:** #302 Beta journeys 15, 16 and 19; context for #386.

## Problem

`open_synthetic` (`host.rs`) runs `preflight_synthetic_seed` first. The preflight correctly refuses to reopen a sample whose seed state is inconsistent. There are three `corrupt` outcomes:

1. Entries but no manifest: "synthetic seed manifest missing".
2. Unparseable manifest: "synthetic seed manifest is invalid".
3. Manifest or durable Core ledger incomplete, or a manifest with no vault: "synthetic seed state is incomplete".

The refusal itself is right; it prevents replayed or duplicated proposals. What is missing is any way out. The Welcome and Access screens (`Gate.tsx`) show only the raw message. A user who closes MedScale during the first seed, or crashes during a sample-workspace FHIR import, cannot open the fictional sample again without editing app data by hand. #386 recorded a CI run in state 1; its root cause in CI is still unproven.

## What review changed

The draft proposed one manual "set aside and rebuild" command that renamed the `synthetic-workspace` folder. All three reviewers rejected it as written. Their findings, verified in source:

- **B1 — the manifest is not in the folder.** `manifest_path`, `pack_dirs_path` and `asr_pack_path` use `with_file_name`, so the seed manifest and the `.desktop-packs.json` / `.desktop-asr.json` records live **next to** the folder.
  - Renaming only the folder recovers state 1 and leaves states 2 and 3 stuck in a loop.
  - It also lets `restore_asr` reattach the old Audio Pack record to the rebuilt sample.
- **B2 — no single-instance guard.** `main.rs` registers no single-instance plugin, and the preflight never checks Core's `writer.lock.sqlite3`.
  - A second MedScale process can see a live first seed as "manifest missing".
  - On macOS a rename can succeed under the first process's open handles. That process would then write a 12/12 manifest for a vault that has moved.
- **M1 — read failures look like corruption.** `complete_synthetic_core` returns `false` for any read failure, including SQLITE_BUSY, so a busy but healthy workspace is reported as `corrupt`.
  - `corrupt` can also come from Core (`AuthorityError::Corrupt`, `DigestMismatch`) after the preflight has passed.
  - Keying recovery on the error kind is therefore unsound.
- **M3 — the manifest is written in place.** `save_manifest` used `std::fs::write`, which truncates the file and then writes it. A crash during `import_fhir`, after Core has promoted the import, leaves an unparseable manifest (state 2) in a workspace that may hold user imports.
- **M2 — prevention beats recovery for the first seed.** An interrupted first seed never reaches the UI (`state.workspace` is set only after `seed_synthetic` returns), so it contains no user data and can be cleaned automatically.
  - The zoom-out reviewer showed that a pure staging-then-rename seed is not a single atomic step today: completeness spans the folder and its sibling manifest, and Windows SQLite handles must be closed before the rename.
  - So prevention needs the reason codes (step 3) to classify leftovers safely.
- **M4 / M2 (zoom-out) — no cap, one slot.** A set-aside copy the app never shows, with a cap of 3 and no delete, recreates the dead end and keeps possibly user-imported data hidden on disk.

## Ordered plan

1. **Atomic sidecar writes — implemented with this spec.**
   - `write_atomic` creates a unique `<file>.<pid>.<counter>.tmp` with exclusive `create_new`, calls `sync_all`, renames it over the target and, on Unix, syncs the directory. This prevents temporary-file clobbering but is **not** a replacement for the single-writer guard in step 3.
   - `save_manifest`, `save_pack_dirs` and `save_asr_pack` all use it.
   - Tests: `sidecar_records_are_replaced_atomically_without_leftover_temp`, `atomic_write_does_not_clobber_an_existing_legacy_temp_file`, and `failed_atomic_replace_preserves_destination_and_removes_its_temp` (directory target forces a failed rename; the existing sentinel survives and the newly created temporary file is cleaned).
   - This removes the M3 source of state 2.
2. **Reason codes, and read failure ≠ corrupt.**
   - `complete_synthetic_core` returns `Result<bool, _>`, so "could not read" maps to `unavailable`.
   - `CmdError` carries a stable reason (`seed_manifest_missing`, `seed_manifest_invalid`, `seed_incomplete`).
   - The UI and every recovery command key on the reason, not on the kind.
3. **Single instance plus a writer-lock check.**
   - Enforce one running MedScale per user.
   - Any recovery command first tries Core's `WriterLock` and refuses on `WriterHeld`, releasing the lock before it touches files.
4. **Automatic cleanup of a never-opened first seed.** For reason `seed_manifest_missing` with no sibling records, it is safe to move the folder aside automatically. Such a folder cannot contain user data, because the workspace was never handed to the UI.
5. **Manual set-aside for committed workspaces.**
   - Move the sibling records first, then the folder last, into one fixed app-owned parent (`synthetic-set-aside/<stamp>-<n>/`).
   - Retry briefly when Windows reports sharing violations.
   - Keep **one** slot, and offer an in-app confirmed delete of the previous set-aside.
   - Apply this on both the Welcome and Access screens.
   - Before release QA exercises recovery, teach `release-webview-postsample-audit.mjs` and `webview-disk-audit.mjs` that `synthetic-set-aside/` (and the existing `fixture-staging/`) are named scopes.

## Non-goals

- Proving the CI root cause of #386.
- Any change to the encrypted vault.

## Risks

- **Stale `.tmp` files.** A crash between the temp write and the rename leaves a stale uniquely named `.tmp`. Readers ignore it; a later write uses a different exclusive filename rather than overwriting the stale file. Cleanup/audit handling remains an open qualification gap.
- **Synthetic markers in `.tmp` files.** A stale manifest `.tmp` holds synthetic markers that the post-sample auditor would not attribute. This only happens after a crash during the write itself.

## Rollback

Step 1: revert to `std::fs::write`. The on-disk format is unchanged.
