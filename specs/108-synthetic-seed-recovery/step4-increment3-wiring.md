# Spec 108 step 4, increment 3: wire the fixture-only proof into first-seed recovery

**Status:** `v3.2, READY TO IMPLEMENT (pending #410)`. The second Goldfish review confirmed that v3.0's B1, M3, M5 and M6 are resolved. It found one new blocker and four major issues in v3.1, and expected the design to be ready once they were fixed. The v3.2 amendments below resolve all of them and **take precedence over §1–§6 where they differ.**

## v3.2 amendments (second review)

- **BLOCKING-1, Unix self-lock.** On Unix, the step-2 `WriterLock` guard on the vault stays open through the rename. Step 4 then re-runs `seed_only_vault` on the discard folder **under that same guard**. It never takes a second lock on the same `writer.lock.sqlite3`, because SQLite's in-process locking would return `WriterHeld`. A macOS test asserts that recovery reaches the delete.
- **MAJOR-1, exact order on Windows.**
  1. Lock the vault and classify it.
  2. Release the lock and rename (retry 5/32/33 for up to 3 s).
  3. Lock the discard folder, re-classify, release the lock.
  4. Delete, or rename back.

  Step 4's delete and rename-back retry 5/32/33 the same way. If the retry window runs out, return `workspace_io_failure`, leave the tree as it is, and never delete. The vault and the discard folder are never locked at the same time.
- **MAJOR-2, nonce.**
  - `SeedManifest` gains `#[serde(default, skip_serializing_if = "Option::is_none")] discard_nonce: Option<String>`. Complete manifests stay byte-for-byte as today.
  - "Exact pending shape" means the raw JSON key set is exactly `{pending, subjects, sources, discard_nonce}`, with `pending: true`, empty arrays, and a nonce of exactly 32 lowercase hex characters.
  - The discard folder is found by comparing the names `read_dir` returns. It is never built by joining a formatted string.
  - The nonce is written once with the pending manifest, and the commit's new manifest drops it.
- **MAJOR-3, `fixture-staging`.** It is removed only if every entry is a regular file whose name and SHA-256 match a bundled fixture. Otherwise it is left in place and recovery continues without deleting it. The headline invariant covers it as well.
- **MAJOR-4, lock side effects.**
  - The lock is taken only on a candidate that `symlink_metadata` shows is an existing real directory, so the lock's `create_dir_all` never creates a folder.
  - "Nothing touched" excludes `writer.lock.sqlite3` and its journal, which the lock may create or roll back. Tree snapshots ignore `writer.lock.sqlite3*`.
- **Minor fixes.**
  1. `atomic_temp_target` is the exact inverse of `write_atomic`'s name generator, with a round-trip test. If #405 or #407 change the format, this lands with them.
  2. New crash points:
     - during the pending-manifest write (temp written, not renamed) → Fresh again;
     - after `remove_dir_all`, before cleanup;
     - part-way through a Windows rename-back.
  3. A complete-manifest temp beside a pending manifest is still refused: deliberately conservative.
  4. Just before `create_dir_all`, `open_synthetic` checks again that the vault is absent; otherwise it refuses.
  5. If both the vault and a discard folder exist, the discard folder is classified and deleted first, then the vault is processed.
  6. A sidecar on the Fresh path maps to `seed_stale_sidecar` only.
  7. The crash harness uses a unique temp dir per parent test (pid plus nanoseconds), passed only through `Command::env`.
  8. The E2E runs after the WebView baseline relaunch. It checks the discard folder's absence directly, not through audit counts, and is built with `e2e-webdriver`.
  9. Core exposes a small guard, `medscale_core::VaultWriterGuard::try_hold(path) -> Result<Guard, {Held, Refused, Io}>`, instead of re-exporting `WriterLock`.
  10. This builds on PR #410's single read transaction, so implementation starts after #410 merges.

**Builds on:**
- the single-file pending → complete model (`step4-first-seed-recovery.md` v2), confirmed by the second Goldfish review;
- `seed_only_vault` from PR #408, plus the typed exact shapes and single read transaction from PR #410;
- the step 3 instance lock (PR #406).

## Invariant

**MedScale deletes a folder only after `seed_only_vault` returns `FixtureOnly` for that exact folder, with Core's `WriterLock` held on it at the time of the check.** No folder is ever deleted because of its name.

## 1. Manifest states and preflight

`SeedManifest` gains `#[serde(default, skip_serializing_if = "std::ops::Not::not")] pending: bool`, so a complete manifest is written byte-for-byte as today.

The preflight returns `Fresh | Complete(seed) | Pending`:

- **`Pending`** — the manifest is *exactly* `{pending: true, subjects: [], sources: []}`. Any other pending manifest is refused with `seed_recovery_refused`.
- **Defence in depth** — `complete_synthetic_seed` returns false when `pending` is set, and `import_fhir` refuses while `workspace.seed.pending` is set.
- **`Fresh`** (B1, M6) — requires all of:
  - no manifest;
  - `synthetic-workspace` is absent, or a real empty directory (`symlink_metadata`, no link or junction);
  - the **sibling scan** (§2) finds nothing except manifest temps that are torn or pending-shaped.

  An orphan `synthetic-workspace.discarding*`, a pack or ASR sidecar, a sidecar temp, or any unknown sibling refuses with `seed_recovery_refused`. Sidecars without a manifest refuse with `seed_stale_sidecar`.
- **Pending manifest timing** — it is written once, and only when the preflight returns `Fresh`. It is written before `create_dir_all` and `CliSession::open_synthetic_vault`. After recovery the existing pending manifest is **reused**, never rewritten.
- **Commit** — `seed_synthetic` builds a fresh, non-pending manifest and saves it last. `state.workspace` is set only after that save. This is unchanged.

## 2. Sibling scan (M2)

`write_atomic` gets a single shared predicate, `atomic_temp_target(file_name) -> Option<target_name>`. It recognises today's `<name>.tmp` and the unique temp names from the open step 1 hardening (#405/#407), including any leading-dot form. Recovery and the fresh check use only this predicate.

The scan covers every `data_dir` entry whose name contains `synthetic-workspace`, compared ASCII case-insensitively. Each entry is classified:

| Entry | Classification |
|---|---|
| `synthetic-workspace` real directory | the vault |
| exact manifest name | the manifest |
| manifest temp that parses as the pending shape | allowed; removed after a successful recovery |
| manifest temp that is torn or unparseable | allowed (its rename never happened) |
| manifest temp that parses as a non-pending (complete) manifest | refuse: the commit may have happened |
| temp of any other target (packs, ASR) | refuse: evidence that the UI was reached |
| `synthetic-workspace.discarding-<nonce>` matching the nonce in the pending manifest | the discard folder (see §3) |
| anything else, including links | refuse |

## 3. Recovery (B1, M1)

`recover_interrupted_first_seed(data_dir, vault_root) -> CmdResult<bool>` runs inside `open_synthetic` under the Host mutex, and only when the preflight returns `Pending`.

The pending manifest also carries a random `discard_nonce`. The discard folder name is bound to it: `synthetic-workspace.discarding-<nonce>`.

1. **Sibling scan (§2).** On refusal, nothing is touched.
2. **For each existing candidate folder** — the vault, then a nonce-matching discard folder:
   - It must be a real directory (`symlink_metadata`).
   - Acquire `WriterLock` on it:
     - `WriterHeld` → `writer_active`;
     - `Claim` error → `seed_recovery_refused`;
     - other errors → `workspace_io_failure`.
   - Run `seed_only_vault` while the lock is held:
     - `FixtureOnly` → continue;
     - `NotFixtureOnly` → `corrupt` / `seed_recovery_refused`;
     - `Unproven` → `seed_ledger_interrupted` if a hot journal exists, `workspace_busy` if busy, otherwise `workspace_io_failure`.
3. **Rename the vault to the discard folder.**
   - **Unix:** the lock is held through the rename. Renaming a directory with open file descriptors is legal, and the lock stays on the inode.
   - **Windows:** the lock is dropped first. The rename then fails if any handle is open. Windows errors 5, 32 and 33 are retried for up to 3 s.
   - Never copy and delete.
4. **Re-classify after the rename, on all platforms (M1).**
   - Acquire `WriterLock` on the discard folder. Nobody else uses that path.
   - Run `seed_only_vault` again. If the result is not `FixtureOnly`:
     - try to rename the folder back;
     - if that fails, leave it and refuse;
     - never delete it.
5. **Delete.** `remove_dir_all` the discard folder. A fixture-only tree with some files removed is still fixture-only (an absent meta file → `FixtureOnly`), so a crash part-way through is resumed by re-classifying from step 2. A test pins this.
6. **Remove leftovers:** `fixture-staging/` (its contents are bundled fixture copies; junction behaviour is pinned by a test) and any allowed manifest temps.
7. **Return `Ok(true)`.** The pending manifest is reused by the seed that follows.

### Residual risk

An external Core CLI could hold or open the vault without MedScale's instance lock. On Unix it is excluded by the held `WriterLock` and the post-rename re-check. On Windows a writer that opens, writes and exits entirely inside the drop → rename gap is caught by the post-rename re-check.

### Not handled by this increment

A hot journal from a crash mid-transaction is **not** recovered here. It returns `seed_ledger_interrupted` and goes to manual recovery (step 5) until the private-copy rollback exists. This is a known coverage gap, and it is likely the most common real crash.

## 4. API, UI and dependencies

- `open_synthetic` returns `CmdResult<bool>`.
- `workspace_open_synthetic` alone adds `"recovered_interrupted_seed"` to its JSON. `vault_label` stays shared with `workspace_status`.
- Welcome and Access show the notice when the flag is true. TS test and `qa/first-run-browser.mjs` mock updated.
- New reasons `seed_recovery_refused` and `seed_stale_sidecar` are added in Rust `seed_reason` and in TS.
- The `allow(dead_code)` markers on `seed_only_vault` are removed.
- `WriterLock` is reached through a narrow Core re-export, `medscale_core::vault_writer_probe`, not a direct desktop dependency on storage layout.

## 5. Crash tests (M3, M4)

### Mechanism
- A `#[cfg(test)]` hook at named boundaries.
- When `MEDSCALE_TEST_SEED_CRASH=<boundary>` matches, it writes a marker file **outside** `data_dir` naming the boundary, then calls `std::process::exit(86)`. No `Drop` runs, and the OS releases locks and handles. This is not a power-loss simulation, and makes no claim to be one.
- The parent re-runs `std::env::current_exe()` with:
  - `host::tests::seed_crash_child --exact --ignored --test-threads=1 --nocapture`;
  - inputs passed only through `Command::env` (`set_var` is `unsafe` under edition 2024, and `unsafe_code` is denied).
- The parent asserts **exit code 86 and the marker**, and prints the child's output on failure.
- The child test returns immediately when the env var is absent.
- These tests run on the Windows and macOS Tauri CI jobs only. No Linux coverage is claimed.

### Expected results

| Boundary | Expected |
|---|---|
| After the pending write, before any Core file | recovered (no folder), 12/12, 2 subjects |
| After Core open, before fixture 1 | recovered, 12/12 |
| After `ingest_fhir_file`, before promote | recovered, 12/12, no duplicate proposal |
| After fixture 1 promoted | recovered, 12/12 |
| After fixture 11 promoted | recovered, 12/12 |
| All 12 promoted, before the commit | recovered, 12/12 |
| During the commit (temp written, not renamed) | refused (non-pending temp) |
| After the commit, before `state.workspace` | opens normally, no recovery |
| Recovery: after the rename, before the delete | the next open resumes, 12/12 |
| Recovery: part-way through `remove_dir_all` | the next open resumes, 12/12 |

Also covered:
- a proposal with no assertion, built by driving Core directly;
- a hand-crafted hot journal → `seed_ledger_interrupted`, nothing touched;
- a junction or symlink inside the discard folder and inside `fixture-staging` → the sentinel outside survives;
- an orphan discard folder, or one whose nonce doesn't match → refused;
- an orphan discard folder beside a fresh vault → refused;
- a complete-manifest temp beside a pending manifest → refused;
- a legacy `seed_manifest_missing` → not recovered;
- a user import or a pack sidecar → refused;
- `writer_active`.

Every test snapshots the `data_dir` tree before and after.

## 6. Native E2E (Windows)

- A `#[cfg(feature = "e2e-webdriver")]` hook writes a marker and **blocks** at the boundary. It does not call `abort`, so Windows Error Reporting cannot hold the process and its instance lock.
- `launch()` takes extra environment variables.
- The journey runs on a pristine profile, first:
  1. Launch with the hook set, click "Try MedScale with sample data", and wait for the marker.
  2. `child.kill()`, then wait until the debug port closes.
  3. Relaunch and open the sample. Assert the rebuilt notice, 2 subjects and 12 documents.
  4. Restart. Assert no notice and the same counts.
- The post-journey disk audit is expected to show no discard folder.
