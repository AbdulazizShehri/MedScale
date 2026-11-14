# Spec 108 step 3: one MedScale process per data directory

**Status:** `RESOLVED`. A fresh-context Goldfish design review found no blockers and 6 major findings. All are resolved below; the draft text follows for history.

## Review resolution (implemented)

| Finding | Resolution |
|---|---|
| M1: the way the file is opened could truncate it or allow deletion | `OpenOptions` read+write+create, `truncate(false)`. On Windows `share_mode(READ \| WRITE)` without DELETE, so a held file cannot be deleted or renamed (Windows test). On Unix the locked inode is compared with the path's inode and the lock is retried on mismatch. The file stays empty and is never deleted by MedScale. |
| M2: Windows may release a killed process's lock late | `WouldBlock` and Windows sharing or lock violations on open are retried in 50 ms steps for up to 3 s before exit 3. |
| M3: macOS/Linux has no native dialog | Stated: the message goes to stderr only on macOS/Linux; exit code 3 still applies. Windows uses an information `MessageBoxW`. |
| M4: the parent's writer-lock check was dropped | Moved explicitly to steps 4–5. Any rename command first clears `state.workspace`, probes Core `WriterLock`, refuses on `WriterHeld`, and drops the probe before renaming. |
| M5: errors could reach the WebView2 message | `startup_failure::exit_already_running()` (exit 3, information icon) and `exit_instance_unconfirmed()` (exit 2) exit directly. They never return `Err` from `setup`. |
| M6: the E2E could hang or falsely pass | The dialog is suppressed only under `feature = "e2e-webdriver"` with `MEDSCALE_E2E_HEADLESS_SECOND_INSTANCE=1`. The journey spawns the exact binary with the first copy's env, asserts exit code **3** exactly (2 fails), counts one `page` target on the debug port, and checks the first copy still responds. |
| m2: storing the lock in `State` | Uses a process-global `OnceLock<File>`, never dropped. |
| m6: stale busy message | Reworded to "wait a moment and try again". |
| m7: version skew | A pre-step-3 build does not take the lock. This must be noted in the release notes. |
**Parent:** `spec.md` (Spec 108), finding B2. Two processes can both see a live first seed as "manifest missing". On macOS a rename can succeed under open handles.

## Problem

- **No single-instance guard.** `main.rs` registers only `tauri_plugin_dialog`, so two MedScale processes can run against the same `app_local_data_dir()`.
- **Core's lock covers too little.** Core's `WriterLock` (`crates/medscale-storage/src/writer_lock.rs`) protects one *open vault*. It does not cover:
  - the sibling seed manifest;
  - the `.desktop-packs.json` / `.desktop-asr.json` records;
  - `fixture-staging/`;
  - the preflight that runs *before* Core opens the vault.
- **Check-then-act risk.** Any recovery or cleanup that inspects these items and then renames them races a second process.

## Decision

Hold an **exclusive OS file lock on `data_dir/medscale-instance.lock` for the whole life of the process**, taken in `setup` before any workspace or vault code runs.

- **Mechanism:**
  - Use `std::fs::File::try_lock()`, stable since Rust 1.89; the toolchain is 1.97.1. It calls `LockFileEx` on Windows and `flock` on Unix.
  - No new crate. `fs2 =0.4.3` is already admitted, but it isn't needed.
  - The OS releases the lock when the handle closes, including on crash, kill or power loss. **A lock file left behind therefore never blocks a restart**: the file is not the lock, the open locked handle is.
- **Ownership:**
  - The locked `File` is stored in a field of `host::Host`, Tauri-managed state that lives as long as the app, so it is never dropped early.
  - Nothing ever unlocks it explicitly.
- **Second instance:**
  - `try_lock` returns `WouldBlock`.
  - Show a native message through the existing `startup_failure::native::show` path (`MessageBoxW` on Windows, existing macOS path): "MedScale is already running. Switch to the open MedScale window."
  - Exit with a new defined code, `ALREADY_RUNNING_EXIT_CODE = 3`.
  - Do not open a window and do not touch any workspace file.
- **Lock unavailable for any other reason** (permission, I/O, unsupported filesystem):
  - **Fail closed** with the same native message mechanism: "MedScale could not confirm it is the only running copy."
  - Exit 2, the existing startup-failure code.
  - Rationale: continuing without exclusivity would reintroduce B2.
- **Order in `setup`:**
  1. `create_dir_all(data_dir)`
  2. acquire the lock
  3. store `data_dir` and the lock in state

  No vault, manifest or staging access happens before step 2.

## What this guarantees

- Within one data directory, at most one MedScale process runs workspace code. Inside that process, commands are already serialized by the `Host` mutex (`cmd_core::lock`). Together these make "inspect, then act" in later steps (4, 5) free of races with other MedScale processes.
- What it does **not** cover:
  - other software (antivirus, indexers, backup tools) briefly opening files. Step 5 still needs a short retry on Windows sharing violations.
  - a user running two copies with **different** data directories. That is not a conflict.

## Tests

- **Rust unit test** (`instance_lock.rs`):
  - Acquire on a temp dir.
  - A second `try_lock` on a separately opened handle to the same file returns `WouldBlock`. Locks are per open handle on Windows and per open file description with `flock`, so the test works inside one process.
  - After the first guard is dropped, acquiring again succeeds.
  - A pre-existing lock file with stale contents does not block acquisition.
- **Native Windows E2E** (`desktop-journey.mjs`):
  - While the app is running, launch the same binary a second time.
  - Assert that it exits within a bounded time with code 3.
  - Assert that the first instance still responds (WebDriver session alive, page text readable).
  - Assert that only one MedScale window exists.
- **macOS:** covered by the Rust unit test only. A native second-launch check on the macOS runner is a follow-up.

## Risks

- **Message boxes in E2E.** A blocked second launch shows a native message box. The E2E must close it, or run with an environment flag that suppresses UI. Proposal: when `MEDSCALE_E2E_HEADLESS_SECOND_INSTANCE=1` is set, skip the dialog and only exit 3. Release builds also honor the flag, but its only effect is no dialog; exclusivity is unchanged.
- **Network or cloud-synced data directories.** `flock` and `LockFileEx` semantics on network or cloud-synced filesystems may differ. `app_local_data_dir()` is local by platform convention. On error we fail closed rather than run unprotected.
- **Partial overlap with Tauri's single-instance plugin**, which would also focus the existing window. Not chosen: it is a new dependency under cargo-deny and supply-chain review, and on Linux it pulls in D-Bus crates.

## Open questions for review

1. Is failing closed on non-`WouldBlock` lock errors acceptable for Beta users, or should it degrade to a warning?
2. Is an env-flag-suppressed dialog acceptable, or should the E2E dismiss the dialog with UI automation instead?
3. Does any code path touch `data_dir` before `setup` runs (for example the panic hook or log setup)?
