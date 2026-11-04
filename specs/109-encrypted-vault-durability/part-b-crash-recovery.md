# Spec 109 Part B: recover a crashed encrypted session

**Status:** `v4, READY TO IMPLEMENT`. Goldfish reviewers (2026-11-04) rejected v1 and v2. Their failures had one cause, automatic actions on recovery state, which v3 removed. A fresh review of v3 returned READY WITH AMENDMENTS, applied here.

**Goal:** a crash, kill or failed seal while unlocked no longer loses committed changes, and nothing in this path can overwrite or destroy the last known-good seal.

**What this does not provide, stated plainly:**
- **Anti-rollback.** The marker stops a stale work file overriding a newer seal. It does not stop an attacker with write access to the vault folder.
- **Proven power-loss durability.** Process-kill tests cannot prove fsync ordering. Sync calls are correct by inspection only.
- **Any PHI claim.** `REAL_PHI_AUTHORIZATION` stays `NOT_AUTHORIZED`.

## Invariants

- **I1.** `meta.sealed` is replaced only by `try_close`, by `EncryptedVault::restore` of a backup, and by the explicit user-confirmed restore in PR 3. Each is atomic (temp, sync, rename) and holds the session lock. The explicit restore keeps the displaced seal. No path in Part B replaces it automatically.
- **I2.** Before a recovered session can seal, a verified durable copy of the seal it descends from exists.
- **I3.** "Could not check" is never "bad". Unknown errors are **retryable**, definitive is an allowlist, and a retryable result changes no file.
- **I4.** No code writes to a leftover before it verifies, except SQLite's rollback of a hot journal. Migration only ever runs on a temp file.
- **I5.** Legacy plaintext rekey runs only on bytes that came out of an authenticated seal.
- **I6.** Recovery state is never deleted automatically. A discard moves files to a new generation folder. Copies and generations are removed only by the retention limit below or by the user.
- **I7.** Every discard shows a notice, and that notice survives a crash.

## The binding marker

`store_state('base_seal_digest')` holds the SHA-256 of the exact `meta.sealed` bytes this work file descends from. `list_id_sequences` parses every `store_state` value as `u64`, so it must exclude this key.

**Restore:**
1. Clear `meta.work.sqlite3.tmp` and `meta.work.sqlite3.tmp-journal`.
2. Write the decrypted plaintext to the temp file and sync it.
3. Open the temp file (legacy rekey allowed: the bytes are authenticated), migrate, write the marker, close. `migrate()` autocommits, so these are separate commits. It doesn't matter: nothing adopts a temp file.
4. Sync the temp file. Move any leftover aside (I6) or remove its sidecars. Rename the temp file over the work file. Sync the directory on Unix, with the Windows sharing retry.

**`create`** ends with a standard restore, so a first-session crash has a marker.

**`try_close`** has three stages, each with its own error type:

| Stage | On failure |
|---|---|
| Write and sync the seal temp file, then rename it over `meta.sealed` | Nothing changed. The vault stays open, the marker stays correct, and a retry works. |
| Sync the directory (Unix) | The rename already happened. The close **succeeds with a warning**, and the work file is **not wiped**. If power loss then reverts the rename, the old seal and the work file's marker still match, so the work is recovered. If not, the next open sees `Stale`, whose contents equal the seal. |
| Wipe | Ignored, as today. Next open sees `Stale`. |

There is no read-closed state and no marker rewrite: after a successful rename the session is over. The close-time `sync_store_to_encrypted` runs before stage one, so a failure there also leaves everything unchanged.

## Open: classify the leftover

In `finish_open`, in this order: session lock → decrypt and digest `meta.sealed` → lease → clear temp files → classify. A torn seal therefore changes no file (existing test `encrypted_vault_durability_109.rs:71`).

1. If decryption fails, return `SealedMetaUnreadable` with nothing changed. A leftover is never adopted without an authenticated seal to compare it with.
2. Use `try_exists` for every presence check. A metadata error is retryable.
3. No work file and no sidecars: restore. Outcome `Sealed`.
4. Otherwise run `verify_work_file`:

| Check, in order | On failure |
|---|---|
| Work file exists, while sidecars exist | Definitive: `Unverifiable` |
| At least one 4096-byte page | Definitive: `Unverifiable` (covers empty and zero-byte files) |
| Open `READ_WRITE` with **no create flag**, `apply_sqlcipher_key`, `verify_sqlcipher_readable`, **no `migrate()`** | Per the error table below |
| `PRAGMA cipher_integrity_check` has no rows | Rows are treated as `Unverifiable` only after the confirmation step below |
| `PRAGMA integrity_check` is `ok` | An error: per the error table. A non-`ok` row: same confirmation step as cipher rows |
| `migration_journal`, `sources`, `authority_objects` and `store_state` exist; `MigrationJournal` has no started-but-unfinished version | Definitive: `Unverifiable` |
| Schema version ≤ `CURRENT_META_SCHEMA_VERSION` (new check; `migrate()` has no upper bound today) | `UnsupportedSchema`: refuse with nothing changed |
| Marker present | Missing: definitive `Unverifiable` |
| Marker equals the digest from step 1 | `Stale` |

**Error table.** `NotADatabase` and `DatabaseCorrupt` are candidates for definitive. `DatabaseBusy`, `DatabaseLocked`, `CannotOpen`, `SystemIoFailure`, `DiskFull`, `PermissionDenied`, `ReadOnly`, `Unknown` and **everything else** are retryable (`_ => retryable`).

**Confirmation before any definitive verdict from SQLCipher.** A failed salt read can make a good file report NOTADB. So: confirm the file can be read end to end with plain file I/O, then wait 250 ms and rerun the keyed check once. Only two matching results are definitive; anything else is retryable.

**Outcomes:**

| Verdict | Action | Outcome |
|---|---|---|
| Retryable | Return `WorkFileCheckUnavailable` (mapped in `enc_err`). Nothing changes. The Access screen offers a confirmed **Move that session aside and open the last saved state**, an `OpenEncryptedVault` flag `discard_leftover` that still requires the key and the session lock. If the move itself fails, the error is the terminal `WorkFileBlocked`: it names the file, says nothing was changed and says the vault opens once the block clears. | Error |
| `Unverifiable` or `Stale` | Quarantine, then restore | `DiscardedInterruptedSession`, with a notice |
| Verified | Continue | |

**Quarantine.** Close the verify connection first, because SQLite on Windows opens without delete sharing. Create `discarded-session/<n>/` with a `verdict` file (`stale` or `unverifiable`) and a `.notice-pending` file, then move the work file, then its sidecars. If a move fails, move everything back, **remove the generation**, and return retryable. A crash between moves leaves sidecars only, which the next open quarantines in a new generation; nothing is replaced.

**Retention** runs only after a complete quarantine and counts only completed generations. It evicts only `stale` generations, oldest first, keeping 3. An `unverifiable` generation may hold real work, so it stays until the user deletes it. Eviction failures are ignored.

**The notice survives a crash.** The notice is a separate flag from the outcome. Any generation with `.notice-pending` sets `notice_pending` on the next open, and the **facade** removes those files only after load and the first persist both succeed.

**`work_source: Adopted | Restored`** is recorded on the open vault. The `next_seq` raise and every load action key on it, never on the notice.

**Adoption, after verification:**
1. Write the copy `pre-recovery/<seal-digest>.sealed` with temp, sync and rename. If it already exists, hash it, and rewrite it if the hash differs. A failure is retryable, and nothing is adopted. When a new copy would make more than 3, the oldest is removed.
2. Close the verify connection.
3. If the schema is older than this build, copy the work file to a temp file, migrate the temp file, sync it and rename it over. The marker survives. A crash leaves only a removable temp file.
4. Open with a new **no-create, no-migrate** keyed open. `open_at_sqlcipher` migrates in place, so adoption must not use it.
5. Outcome `RecoveredInterruptedSession`.

## Load

`OpenEncryptedVault` runs `load_store_from_encrypted` after the open. **The default is retryable.** Definitive is only:

- a blob reported missing by `ErrorKind::NotFound`, on two reads 250 ms apart;
- authentication or digest failure of a blob;
- a row that fails to decode, including the decode failures at `durable.rs:309-378`, which are flattened today.

`sealed_blob.rs` stops reporting every read error as `NotFound`. `durable.rs:52,136,142` stop flattening everything to `InvalidArgument`.

| Open outcome | Load result | Action |
|---|---|---|
| `work_source` | Load result | Action |
|---|---|---|
| Adopted | Succeeds | Raise `next_seq`, persist it, then fill the slot |
| Adopted | Definitive failure | Quarantine the work file, restore, load again. Outcome `Discarded` |
| Adopted | Retryable failure | Return it. Nothing changes |
| Restored | Succeeds | Fill the slot |
| Restored | Any failure | Remove the restored work file, which is a pristine copy of the seal and holds no session. Return the error. The seal is untouched |

A successful load purges nothing, since it does not prove domain rows are sound.

These actions need new consuming `EncryptedVault` methods that close the connection and keep the session lock, because the `?` at `facade.rs:1690` would otherwise drop the vault and release the lock.

A crash between restoring and loading leaves a pristine file whose marker matches, so the next open reports `Recovered` although there was nothing to recover. That's a false notice in a rare window, and no work is at risk. A failure at `try_close` stage 2 or the wipe can also produce a false notice.

## Consistency after a crash

Domain writes commit before the authority snapshot, so a crash between them leaves domain rows ahead of it. Load doesn't check those rows.

- **Reused IDs (the real defect).** The in-memory `next_seq` regresses, so an ID such as `derived-N` can be reissued, and a surviving receipt (`privacy_gate.rs:821,859`, `medagent.rs:1034,1059`) would then name the wrong artifact. **Fix:** on `RecoveredInterruptedSession`, raise `next_seq` by 2^32 and persist it before the vault enters the slot. No single request allocates that many IDs. `get_next_seq` must return an error on a corrupt value instead of silently returning 0 (`sqlite_meta.rs:775`).
- **Dangling references remain.** A receipt from the interrupted request can name an object the snapshot never got. It reads as not found, which is honest; it is not silently wrong.
- **Lost audit rows.** `project_graph.rs:333-334` writes audit rows for the request in flight. A crash can lose them, as on the synthetic vault today. This is disclosed, and only the one interrupted request is affected.

A one-transaction dispatch would fix all three, but handlers open their own transactions, and SQLite has no nested `BEGIN`. That's a larger change, and it is not needed for the guarantee.

## Durability

- `PRAGMA fullfsync = ON` on every keyed connection (it is per-connection, so the verify connection too). A macOS CI measurement decides whether it stays. `synchronous` stays FULL, and a test pins that.
- `SealedBlobStore::put` syncs `sealed_blobs` on Unix after renaming a new blob, and retries Windows sharing errors on the rename.
- Windows has no directory sync; NTFS journaling is relied on, unproven.

## Backup and restore

`EncryptedVault::restore` today overwrites `meta.sealed` with `fs::copy` and no lock (`encrypted_vault.rs:425`). It changes to: take the session lock, copy blobs **first**, write the header and then the seal by temp and rename, and move any destination leftover to a generation folder last. A seal is never in place before the blobs it names. A crash before the last step leaves a work file whose marker doesn't match, which is `Stale`. Only tests call it today.

`backup` omits `pre-recovery/` and `discarded-session/`. Both folders and all temp files go in `RESERVED_VAULT_PATHS` and `plaintext_marker_absent_on_disk`.

## Telling the user

`EncryptedVaultReady.discarded_interrupted_session` becomes `open_outcome: Option<VaultOpenOutcome>`, omitted for `Sealed`. Users to update: `mod.rs:1828`, `facade.rs:1654,1697`, `cli_session.rs:22,204,221`, `encrypted_vault.rs:70,217`, `main.rs:1544`, `host.rs:2387`, `cmd_core.rs:145,163`, `seed-failure.ts`, `Gate.tsx`, and the tests.

| Outcome | Notice |
|---|---|
| `Recovered…` | "Recovered changes from your last session. MedScale did not close normally. Check that your recent work looks right." |
| `Discarded…` | "Recent changes may be lost. MedScale did not close normally and could not verify that session, so the vault opened from its last locked state." |

The second is wrongly shown only in a crash between a good seal and its wipe, which is why it says "may".

## Security analysis

- **Confidentiality:** unchanged. Work files and copies are encrypted, and a clean close wipes them.
- **Weaker than the seal:** a recovered work file has per-page HMAC, which covers the page number but no version. **Salt and key persist across seals**, so an attacker with write access and old work files from **any earlier session** can splice in pages. The marker covers one page, and `integrity_check` catches this only when the structure breaks. Sealing a recovered session then launders the result into an AEAD seal.
- **An attacker's existing power:** they can already restore an older valid seal. This adds splicing, not a new class of access. It's accepted for non-PHI Beta use and **must be re-examined before any PHI authorization**.
- **Deleting a blob** forces a discard of the recovered session. That is denial of service, not a read or a forgery.
- **The explicit restore in PR 3** can bring back deleted data and truncate the audit trail. Its confirmation must say so.
- **Rejected: one SQLCipher file at rest.** The deciding reason is that every commit would mutate the only copy, which cannot satisfy "never risk the last known-good". Its authentication and migration costs are weaker arguments.

## Implementation order

1. **PR 1, no recovery yet: leftovers are still discarded.** It does change `try_close` staging and adds the schema bound. Typed blob and load errors, restore via temp and marker, `create` ends with restore, the `list_id_sequences` exclusion and a `restore_id_sequences` rejection of the marker key, `get_next_seq` error, `fullfsync`, blob directory sync, schema upper bound, no-migrate open, `try_close` stages, and the crash harness. Leftovers are still discarded.
2. **PR 2, one change.** Classification, confirmation, adoption, copies, quarantine, the load table, the `next_seq` raise, notices, the confirmed move-aside, and the native kill journey.
3. **PR 3, required before Beta.** A Settings section listing recovery copies and discarded sessions with their dates and sizes. Explicit confirmed restore (keeps the displaced seal; says plainly it can bring back deleted data and shorten the audit trail) and explicit delete. Without this, the copies are hidden data, the objection Spec 108 raised.

## Tests

**Crash harness.** No keyed crash test exists today, so this is new: unit tests inside `encrypted_vault.rs` re-execute the test binary (the `host.rs` pattern) with a `#[cfg(test)]` `crash_point`, using real SQLCipher. Boundaries: each restore step, seal temp write, seal rename, wipe, copy write, adoption, quarantine, migrate-on-copy. After each one, the next open succeeds and `meta.sealed` still authenticates.

**Property:** after any crash point, the next open shows the last seal or the last committed write, or returns a retryable error with nothing changed. It is never empty, and never older than the last seal.

**Classification, real SQLCipher:** empty, short, zero-filled and random files; plaintext SQLite; wrong key; real hot journal (copy the database and journal mid-transaction, since a dropped connection rolls back); corrupt pages; marker missing; marker naming an older seal; interrupted migration; newer schema; held lock and permission denied, both retryable with every file byte-identical. A one-shot injected failure on the first keyed read must end up **adopted**, not discarded.

**Attacks:** seal deleted with a good work file: refused. Work file from an older session: not adopted, quarantined, notice shown.

**Load:** missing blob on a recovered session falls back, and the session is in a generation folder afterwards. Blob read error (not missing) is retryable with nothing changed. A load failure after a restore leaves the seal alone and removes the pristine file.

**Retention and notices:** nothing is purged by a successful open. A crash mid-quarantine still produces the notice. A fourth generation removes only the oldest.

**`try_close`:** failure before the rename stays open and retries; failure at the directory sync leaves the work file and closes.

**IDs:** crash during a request that allocates and writes a domain row, then recover, allocate: no ID is reused.

**Native E2E, Windows:** create a vault with a throwaway passphrase, add a project, kill the process tree, relaunch, unlock, see the notice and the project.

**Measured, not assumed:** `cipher_version` is logged. A torn-file test confirms `cipher_integrity_check` behaviour on this build, since the source isn't available to read.

## Changes from v2

- **Nothing purges automatically.** A successful load was deleting the quarantine in the same open, and the copy before a later seal was proven.
- **Generation folders** replace the single slot, which could lose good work to a pristine restore.
- **The notice survives a crash.**
- **Read-closed is gone.** Nearly every request persists, including reads, and the close path syncs first, so it could never work. `try_close` now has three typed stages instead.
- **Migration never touches the real work file.** `open_at_sqlcipher` migrated in place before the copy step.
- **Copies are verified by hash,** since a name doesn't validate contents.
- **Load errors default to retryable,** with a recheck for a missing blob.
- **A confirmed way out** when a retry never clears.
- **I1 is true now:** `restore` is included.
- **Windows:** quarantine closes the verify connection first.

## Changes from v1

- **Automatic rollback is gone.** It could overwrite the newest seal on a transient read.
- **Definitive is an allowlist,** with a confirmation step and quarantine, since NOTADB can be transient.
- **`create` writes the marker.**
- **Every mismatch shows a notice.** "Can only mean predates" was false.
- **Reused IDs are fixed,** rather than treating this as the synthetic vault's accepted behaviour.
- **The security claim is corrected:** any earlier session, not just the same one.
