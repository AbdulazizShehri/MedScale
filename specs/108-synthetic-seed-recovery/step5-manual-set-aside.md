# Spec 108 step 5: manual set-aside of a sample workspace that cannot be opened

**Status:** `v2`, implemented as 5a (hot-journal settle on the committed path), 5b (Rust set-aside and delete) and 5c (UI and native E2E). Deviations:
- `webview-audit-compare.mjs` hard-codes three scope labels and cross-checks their sums, so `synthetic-set-aside/` and the `synthetic-workspace.*` siblings are classified as `synthetic_workspace` rather than a new scope.
- The offer is shown on Welcome. Access keeps its existing error handling, because its error state is shared with encrypted unlock (Minor 4).
- The release post-sample audit is unchanged, because release QA never exercises set-aside. A Goldfish review (2026-11-03) found 1 blocker and 7 major issues. The amendments below resolve them and take precedence over the draft. The work ships as three PRs: 5a, 5b and 5c.

## v2 amendments

- **5a — settle on the Complete path (M2).** When a *committed* sample's preflight finds a hot journal, the open takes `VaultWriterGuard` and calls `settle_interrupted_write`, exactly as Core's next open would, then runs the preflight again. The cause is a crash during any later write, such as an import. `seed_ledger_interrupted` (and the set-aside offer) remains only if the journal is still present after settling (the cold, zero-header case).
- **Move set equals scan set (B1).**
  - One shared predicate, `is_synthetic_sibling(name)`, returns true when the name contains `synthetic-workspace` (ASCII case-insensitive). `scan_siblings` and set-aside both use it.
  - Set-aside moves **every** matching `data_dir` entry, including temp files, case variants, a file in place of the folder, and links. A link is moved as an entry: `rename` acts on the link, never its target.
  - The guard is taken only when `synthetic-workspace` is a real directory.
  - Test: for every refused state, set-aside followed by `scan_siblings(None)` returns Ok.
- **Partial slot (M1).**
  - The slot is created with `create_dir` (not `create_dir_all`) as `synthetic-set-aside/<stamp>.partial/`, and renamed to `<stamp>` only after the vault has moved.
  - A later set-aside **resumes** into an existing `.partial` slot, checking each target with `ensure_absent`.
  - Delete is offered only for a completed slot.
  - After a full rollback, the empty slot is removed with `remove_dir`.
- **Move order and durability (Minor 1).**
  1. The manifest and its temp file move first, so a partial state never shows step 4 a pending manifest.
  2. Then the remaining siblings.
  3. Then the vault, last.
  - On Unix, `sync_parent_dir` runs for both `data_dir` and the slot after each rename.
- **Rollback (Minor 2).** It moves each item back with `ensure_absent` on the original path, in reverse order. Nothing is deleted on any failure path.
- **Backend re-check (M3).** Inside the mutex, set-aside runs `preflight_synthetic_seed` again. It refuses when the result is `Complete`, or `Fresh` with no matching siblings. The UI is never the only gate.
- **Reasons (M4).**
  - New: `set_aside_slot_occupied`, `set_aside_incomplete` ("Some sample files were moved to the set-aside folder; try again to finish") and `set_aside_refused`.
  - Each is added to Rust `seed_reason` and TS `SEED_REASONS`.
  - `VaultWriterGuardError` maps as: `Held` → `writer_active`, `Refused` → `set_aside_refused`, `Io` → `workspace_io_failure`. The last applies only when nothing was moved; otherwise the result is `set_aside_incomplete`.
  - The stale `seed_manifest_missing` phrase about "another MedScale window" is dropped (Minor 6).
- **Disclosure (M5).**
  - `workspace_status` reports `set_aside_exists`.
  - Welcome and About show a persistent notice with **Delete set-aside sample**.
  - The set-aside dialog states that the copy is **not encrypted**, **cannot be opened in MedScale**, may include imported files, and is kept until deleted. It also notes that Audio Pack settings must be set up again; the model files stay.
  - The delete confirmation says the deletion is permanent.
- **Delete safety (M6).**
  - `synthetic-set-aside` must be a real directory (`symlink_metadata`).
  - The delete removes only that folder, and `remove_dir_all` does not traverse links.
  - Tests: a junction or symlink inside the slot, and a junction *at* `synthetic-set-aside`, each with an outside sentinel that must survive. Symlink cases are gated to Unix; Windows uses junctions.
- **Tests (M7).**
  - A `#[cfg(test)]` fault hook fails rename N.
  - Crash-child boundaries at each move check three things: no automatic deletion on the next launch, a reason that still offers set-aside, and the partial slot resumed.
  - Hash comparisons exclude `writer.lock.sqlite3*`.
- **UI details (Minors 4, 5, 7).**
  - The offer appears only for a failure of `workspace_open_synthetic`, using the Access screen's separate error origin.
  - After a successful set-aside, the UI opens the sample automatically; a failure there shows the normal guidance.
  - The dialog traps focus and returns it to the trigger.
- **Audit scripts (Minor 8).**
  - `webview-disk-audit.mjs` gets a `synthetic_set_aside` scope.
  - `release-webview-postsample-audit.mjs` gets a separate `set_aside_marker_files` counter, with an explicit non-passing interpretation.
  - `synthetic-workspace.discarding-*` and `*.tmp` siblings are classified.
- **Native E2E (Minor 9).**
  - It runs after the FHIR import journey, so the set-aside covers a real user-import workspace.
  - The manifest is deleted only after `quit()`, and only behind the CI disposable-runner guard.
- **Slot name (Minor 10).** `<UTC yyyymmddThhmmssZ>`, created with `create_dir`, so two attempts in the same second collide visibly.
- **Open questions answered.**
  1. **No** set-aside for `core_integrity_error` in step 5.
  2. **One slot plus a delete that is always reachable** is the chosen design.
**Builds on:** step 2 (reason codes), step 3 (instance lock), step 4 (automatic recovery, `VaultWriterGuard`, sibling scan).

## Problem

Automatic recovery (step 4) deliberately refuses any sample workspace it cannot prove holds only bundled fixture data. In these cases the user cannot open the fictional sample at all, and today the only fix is to delete app data by hand:
- a legacy missing manifest;
- user imports or rows;
- stale pack or ASR records;
- an invalid or forged manifest;
- a complete-manifest temp;
- `seed_recovery_refused`;
- a hot journal that SQLite left cold.

## Decision

A **user-confirmed** action that **moves** (never deletes) the sample workspace and every file that belongs to it into **one** app-owned slot. Opening the sample afterwards starts a fresh first seed.

### What is moved

These are the exact paths `host.rs` builds from `data_dir` (`manifest_path`, `pack_dirs_path`, `asr_pack_path`, `VAULT_NAME`). Each is moved only if it exists:
- `synthetic-workspace.desktop-packs.json`;
- `synthetic-workspace.desktop-asr.json`;
- `synthetic-workspace.desktop-seed.json` and its write temp;
- any `synthetic-workspace.discarding-*` folder;
- `synthetic-workspace/`, **last**.

Moving the siblings first and the folder last means a crash part-way leaves either:
- the folder without its siblings, which reads as legacy `manifest_missing` and is still a valid manual set-aside target; or
- everything moved.

Every entry is checked with `symlink_metadata`. Links are refused, never followed.

### Where

- Destination: `data_dir/synthetic-set-aside/<UTC yyyymmddThhmmssZ>/`, a single slot.
- If the slot already holds a set-aside, the action refuses and offers "Delete the previously set-aside sample".
- That delete is confirmed separately in the UI and removes only `synthetic-set-aside/`. It never touches the live workspace.

### Safety preconditions

All of these are checked inside the Host mutex:
1. No workspace is open. `state.workspace` must be `None`; otherwise the action refuses.
2. The instance lock is held (step 3).
3. If `synthetic-workspace/` exists, Core's `VaultWriterGuard` is held on it while the siblings move.
   - **Unix:** the guard is kept through the folder rename.
   - **Windows:** the guard is dropped just before the folder rename, which fails if any handle remains.
   - Renames retry Windows errors 5, 32 and 33 for up to 3 s (`with_fs_retry`).
   - `WriterHeld` → `writer_active`.
4. Each target name must be absent in the slot, checked with `ensure_absent`.

### Rollback on partial failure

If any move fails, already-moved items are moved back in reverse order on a best-effort basis, and the result is `workspace_io_failure`. If moving back also fails, the slot keeps them. **Nothing is deleted on any failure path.**

### UI

- On Welcome and Access, when opening the sample fails with any step 2 or step 4 reason in the `incomplete` category, show **"Set the sample workspace aside…"**.
- The confirmation dialog says, honestly:
  - the sample workspace and its settings will be **moved** to a folder inside MedScale's data;
  - nothing is deleted;
  - a new sample will be created;
  - FHIR files the user imported into the sample stay in the set-aside copy, and their original source files elsewhere are untouched.
- If a previous set-aside exists, the dialog offers to delete it first, with its own confirmation.
- Focus moves into the dialog, Escape cancels, the buttons have accessible names, and the result is announced through the existing `role="status"`.

### Commands

- `workspace_set_aside_synthetic` → `{ set_aside: true }`.
- `workspace_delete_set_aside_synthetic` → `{ deleted: true }`.
- Both are added to `capabilities/default.json`, `build.rs` and `main.rs`. `ipc.test.ts` enforces this.

### Privacy audit scripts

`webview-disk-audit.mjs` and `release-webview-postsample-audit.mjs` classify:
- `synthetic-set-aside/` as a named scope, **not** whitelisted as clean;
- `fixture-staging/` as a named scope.

## Tests

- **Rust:** after each of these refused states, set-aside moves every listed item with byte-identical contents (hashes compared), then `open_synthetic` seeds a fresh 12/12:
  - legacy `manifest_missing`;
  - a user-import workspace;
  - a stale sidecar;
  - an invalid manifest.
- **Rust refusals:**
  - a workspace is open;
  - the writer lock is held (`writer_active`);
  - the slot is occupied;
  - a symlinked sibling.
- **Rust failure injection:**
  - a failure mid-move rolls back;
  - nothing is deleted on any failure.
- **Rust:** delete-set-aside removes only the slot.
- **TS:** the dialog text and the parser for the new results.
- **Native Windows E2E:**
  1. Create a legacy state (seed, quit, delete the manifest).
  2. Relaunch and see the refusal guidance.
  3. Use set-aside and confirm.
  4. The sample opens with 12 documents.
  5. The set-aside folder exists, and the disk audit reports it in its own scope.

## Open questions for review

1. Should set-aside also be offered for `core_integrity_error`, which can come from a later Core failure rather than from setup?
2. Is a single slot with an explicit delete the right trade-off, versus keeping several set-asides and the hidden data they hold?
