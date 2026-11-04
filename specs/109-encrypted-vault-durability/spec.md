# Spec 109: Encrypted vault durability across lock failure and crashes

**Status:** `v2`. Parts A and C are implemented. Part B is not. Three Goldfish reviewers (design, codebase details, zoom-out; 2026-11-04) read the v1 draft. It proposed automatic recovery of a verified work database in one change; all three found that unsafe as written. This version splits the work.

**Amends:** Spec 017 FR-003. Discarding an unverified leftover stays the behaviour, but it is no longer silent, and it never happens before the replacement is authenticated.

## Problem

A static audit on 2026-11-04, confirmed by reading the source, found these defects. They apply to synthetic or permitted data today, because `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`.

| ID | What happened |
|---|---|
| D1 | `fs::write` rewrote `meta.sealed` in place, and unlock wiped the work database before unsealing. Power loss during Lock tore the seal; the next correct unlock destroyed the only good copy, then failed. |
| D2 | Core persists every write to the work database, but a crash means the next unlock discards every write since the last unlock. It did so silently. |
| D3 | `CloseEncryptedVault` consumed the vault before sealing, so a failed seal could not be retried and returned `VaultClosed` on a second attempt. The screen moved to Access with the vault still open. |
| D4 | `CliSession::open_encrypted_vault` hard-codes `recovery_code: None`, yet the screens say recovery codes replace a lost passphrase. |
| D5 | Nothing stopped a second process from wiping a live session's work database. The holder name is always `cli-holder`, so the lease does not distinguish processes. |

## Part A: no destruction before a replacement exists (this change)

Fixes D1, D3 and D5, and makes D2 visible.

- **Atomic seal.** `meta.sealed.tmp`, `sync_all`, rename (with a bounded retry of Windows errors 5, 32 and 33), then a directory sync on Unix. The work database is wiped only after all of that returned.
- **Authenticate first.** `finish_open` decrypts `meta.sealed` into memory before it touches anything. A failure returns `SealedMetaUnreadable` (`AuthorityError::Corrupt`, reason `core_integrity_error`) with every file unchanged, including the work database.
- **Key before lease.** The lease and session lock are taken only after the key and the seal both proved good, so a wrong passphrase changes nothing on disk.
- **Atomic unseal.** The restored copy goes through `meta.work.sqlite3.tmp` and a rename. Sidecars are removed first, and a sidecar that cannot be removed is an error, so a stale journal can never be rolled back into a fresh copy.
- **`try_close`.** It seals with the connection open and drops the connection only on success. The store uses delete-journal mode and no transaction is open between requests, so the work file is complete. On failure it hands the vault back, and `CloseEncryptedVault` returns it to the slot, so the vault stays open and writable and a retry works. There is no reopen path and no "vault gone" state.
- **Session lock.** `.session.lock`, an OS lock held for the whole session, stops another process from wiping a live work database. It is taken before the lease, so a refused opener cannot rewrite the lease. The empty file stays after close, because unlinking it would let two openers lock different files. Because this lock is what refuses a live holder, anything found in `.writer.lock` is stale and is overwritten.
- **The leftover goes last.** The replacement is written and synced first, sidecars are removed, the leftover is scrubbed in place, and a rename replaces it, so a crash or full disk cannot hide a discard from the next open. A failed rename after the scrub fails that open but loses nothing that was going to be kept: the seal is intact and the next open succeeds. Part B must drop this scrub.
- **Durability of what the seal depends on.** Blobs are synced before their rename, and `sealed_blobs` is synced on Unix before the seal, so a sealed row cannot outlive its blob after power loss. An existing blob is not rewritten, since it is content-addressed and Core re-puts every blob per snapshot. `vault_header.json` is written durably at creation, so power loss cannot lose the key wraps under a durable seal.
- **Say what happened.** `EncryptedVaultReady.discarded_interrupted_session` reaches `workspace_unlock`, and the app shows "Recent changes may be lost". The wording says *may*, because a leftover can also mean a sealed session whose wipe failed.

The screen-side half of D3 ships in PR #419.

### Known limitation after Part A

A crash or kill while unlocked still loses changes made since the last unlock. The difference is that the user is now told, and the last sealed state is never at risk. `docs/beta/QA_MATRIX.md` records this.

Two narrower limits remain, both found in the final review:
- A crash during creation can leave a header with no seal, which neither create nor unlock accepts. No data exists yet at that point, and the folder has to be removed by hand.
- A blob torn by power loss **before** this fix is not healed, because an existing blob is not rewritten. Later unlocks could then fail. Blobs written after this fix are synced first.

## Part B: recover a crashed session (not implemented)

This is the fix for D2's data loss. It must meet every requirement below, and it needs its own Goldfish review of its own design:

1. **Verify without writing.** `open_at_sqlcipher` creates missing files and runs `migrate()` first, so a zero-byte file gains an empty schema, "verifies", and then replaces the good seal at the next close. A new opener needs no create flag, must reject empty and short files, must run `cipher_integrity_check` and `integrity_check`, and must check the schema before any migration. An interrupted migration counts as unverifiable.
2. **"Could not check" is not "bad".** BUSY, disk full and sharing violations return a retryable error and wipe nothing. Only authenticated corruption or a wrong-key result may discard.
3. **Prove loadability.** A verified row can reference a lost blob, and `load_store_from_encrypted` would then fail on every unlock. Include the load in verification, or give a typed recoverable error.
4. **Bind to the seal.** `integrity_check` does not catch structurally valid rollback, and an old whole work file would beat a newer seal. Store `base_seal_digest` in the work database, and adopt it only on a match or an unreadable seal.
5. **Consistency.** Domain writes run before the authority snapshot (`facade.rs`), so a crash can leave them ahead of it. Wrap dispatch and snapshot in one transaction, or show that each handler tolerates this.
6. **Durability.** Set `fullfsync` on macOS, since SQLite's commit is otherwise not durable there.

Dropping `meta.sealed` for a single SQLCipher file at rest would remove most of this, at the cost of amending Spec 017 FR-002, the probes and the backup format. Part B's design must weigh that option.

### Rejected: seal after every request

`persist_open_vault` runs on almost every dispatch, including reads. Recordings are stored in this database at up to 32 MiB each, and `WrappedBlob` serializes as a JSON number array, about 3.6 times the size by estimate. Ten recordings would mean about 1.15 GB per request. That overhead also makes a full-disk failure at Lock more likely, which is why `try_close` matters. A binary seal format would be worthwhile on its own.

## Part C: recovery code unlock

The backend already worked (`facade.rs`, `open_with_recovery`). This adds `CliSession::open_encrypted_vault_with_recovery_code`, the command `workspace_unlock_with_recovery_code`, and a **Use a recovery code** toggle on the Access screen. Copy says what is true: a code opens the vault, the passphrase cannot be changed in this version, and using a code does not consume it.

A wrong passphrase or code now reads "denied" with a message that says so, instead of "vault or lease not ready".

## Tests

`crates/medscale-storage/tests/encrypted_vault_durability_109.rs` and `crates/medscale-core/tests/encrypted_close_retry_109.rs` (real SQLCipher, synthetic data):

- A torn seal refuses with nothing changed, alone and with a work database present.
- A wrong passphrase changes nothing with a leftover present.
- A crash discards the session and reports it; a clean close reports nothing.
- Stale temp files and a stale journal are ignored and removed.
- A failed seal hands back a vault that stays usable, and a retry seals both writes, in storage and through Core.
- A second open of a live session is refused and wipes nothing.
- A wrong passphrase creates no lease.
- A stale or torn lease does not strand the vault.
- A clean close leaves no session files except the empty `.session.lock`.
