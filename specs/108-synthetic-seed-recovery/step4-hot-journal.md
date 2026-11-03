# Spec 108 step 4: recover an interrupted first seed that left a hot journal

**Status:** `v2, READY TO IMPLEMENT`. A Goldfish review (2026-11-03) found no blocker and four major findings. The amendments below resolve them all and take precedence over the draft text that follows.

## v2 amendments

- **The guarantee and its conditions (MINOR-1).**
  - Rolling back cannot discard committed data **provided** three conditions hold:
    - the journal and the database stay paired;
    - `synchronous` is at least NORMAL (it is FULL by default, and storage sets no pragma);
    - fsync is honest.
  - If a journal sits beside a replaced `meta.sqlite3`, rollback would corrupt it. Core's own open would apply that journal too, so this is SQLite's documented hazard and not a regression introduced here.

- **The settle API (MAJOR-3, MINOR-2).**
  - New storage function `settle_meta_journal(root)`. It opens `root/meta.sqlite3` with `SQLITE_OPEN_READ_WRITE | SQLITE_OPEN_NO_MUTEX` and **no** CREATE flag. It never calls `create_dir_all` or `migrate()`, and never goes through `open_connection`.
  - `VaultWriterGuard` now records the root it locked. `settle_interrupted_write(&self)` takes no path, so it can only settle the folder it holds.
  - Sequence:
    1. `busy_timeout(0)`;
    2. one read: `SELECT COUNT(*) FROM sqlite_master`;
    3. `BEGIN IMMEDIATE; ROLLBACK` (MAJOR-2);
    4. close.
  - Error mapping: BUSY or LOCKED → `Held`. Anything else → `Io`.

- **Live writers (MAJOR-2).** The `BEGIN IMMEDIATE` probe returns BUSY whenever any other connection holds RESERVED or a stronger lock. Recovery then reports `workspace_busy`, never `seed_ledger_interrupted`. There are test rows for both an EXCLUSIVE holder and a RESERVED holder.

- **Host mapping (MAJOR-3).** The host settles only when `meta.sqlite3` exists as a regular file and a journal exists.
  - A journal with no `meta.sqlite3` keeps today's reason, `seed_ledger_interrupted`.
  - A settle failure with `Held` → `workspace_busy`.
  - A settle failure with `Io` → `workspace_io_failure`. This is retryable: if playback succeeded but the journal delete failed, replaying again is idempotent (MINOR-4).

- **The real junk-journal rule (MAJOR-4).**
  - A journal whose first byte is non-zero is **hot**. SQLite finds it shorter than one header, replays nothing, and **deletes** it in delete mode. This is safe, because with a synced header no database page was ever overwritten. Proof then proceeds.
  - A journal whose first byte is zero, or a zero-length journal, is not hot and stays on disk. The result stays `seed_ledger_interrupted`.
  - Tests cover both cases.

- **Why a private-copy rollback was rejected (MAJOR-4).** Increment 3 promised one, and this design replaces it.
  - Rolling back a copy cannot tell a live RESERVED writer from a crash, so it could prove a state the real file is not in.
  - The in-place rollback runs under Core's writer lock **and** SQLite's own EXCLUSIVE lock. Core would do the same on its next open.
  - Increment 3's rule that a refusal touches nothing now has two permitted exceptions: the `writer.lock.sqlite3*` files, and an in-place hot-journal rollback, which removes only uncommitted changes. Before/after snapshots on refusal compare `meta.sqlite3` only when no journal was present.

- **Test rigour (MAJOR-1).**
  - Use `File::open` with `io::copy`, not `std::fs::copy`. Note: on macOS, closing the copied file drops the process's POSIX locks on it. This is harmless here because connection A is idle between statements.
  - Before recovery, assert that a read-only open of the copy fails with `SQLITE_READONLY_ROLLBACK`, which proves the journal is hot.
  - Assert that a second copy **without** the journal shows the forged rows, which proves the uncommitted pages reached the database file.
  - After recovery, assert exactly 12/12 and no forged row.

- **Journal mode coupling (MINOR-3).** A comment and a test tie `seed_only_files`'s allowlist to delete-mode journals. If Core ever moves to WAL, TRUNCATE or PERSIST, this must be revisited.
**Builds on:** step 4 increment 3 (PRs #414, #415, #416), `main@3cc7715`.

## Problem

Core stores the vault ledger in `meta.sqlite3` in SQLite's default rollback-journal mode. No `journal_mode` pragma is set anywhere in `medscale-storage`.

- **How the hot journal arises.** Every Core request rewrites `authority_objects` inside one transaction. If MedScale is killed or loses power during that transaction, `meta.sqlite3-journal` survives as a *hot journal*.
- **What recovery does today.** `seed_only_files` treats a `meta.sqlite3-journal` name as `Unproven`. `hold_and_prove_fixture_only` then returns `seed_ledger_interrupted`, so the user has to recover manually.
- **Why that matters.** A crash in the middle of a transaction is probably the most common real interruption of a first seed.

## SQLite's own rule

A hot journal holds the original content of pages that an **uncommitted** transaction changed. SQLite's documented behaviour on the next open is to roll it back, which restores the last committed state.

- Core does exactly this the next time it opens the vault (`SqliteMetaStore::open`).
- Rolling back cannot discard committed data: commit is the act of deleting or invalidating the journal.
- A journal whose header is invalid or zeroed is not hot. SQLite ignores it.

## Decision

**New Core API:** `VaultWriterGuard::settle_interrupted_write(&self, vault_root) -> Result<(), VaultWriterGuardError>`, added to the narrow guard from #414.

- **Preconditions:**
  - The guard already holds Core's exclusive writer lock on `vault_root`.
  - The function takes `&self`, so it can only be called while that lock is held.
- **Mechanism:**
  - Open `vault_root/meta.sqlite3` **read-write** through `medscale-storage`. The surface never opens it.
  - Set `busy_timeout(0)`.
  - Run one read (`SELECT COUNT(*) FROM sqlite_master`). That read makes SQLite roll back a hot journal, exactly as Core's own open would.
  - Close the connection.
- **Errors:**
  - BUSY or LOCKED → `Held`.
  - Anything else → `Io`.
- **What it never does:**
  - It does not run migrations.
  - It does not create the file if it is missing.
  - It does nothing when no journal is present.

**Recovery change in `hold_and_prove_fixture_only`:**

- If `seed_only_vault` returns `Unproven` **and** `meta.sqlite3-journal` exists:
  - call `settle_interrupted_write` while still holding the guard;
  - run `seed_only_vault` again.
- The second result maps exactly as today:
  - `FixtureOnly` → proceed;
  - `NotFixtureOnly` → `seed_recovery_refused`;
  - `Unproven` → `seed_ledger_interrupted`.
- The fixture-only proof is always applied to the **settled, committed** state, which is the state Core would see anyway.

## What changes on disk when recovery refuses

- When recovery refuses after settling, the vault is no longer byte-identical: the hot journal has been rolled back.
- This is exactly what Core's next open of that vault would do, and it removes only uncommitted changes.
- It is documented as the single permitted side effect of a refusal, in addition to the `writer.lock.sqlite3*` files already excluded.

## Tests (real SQLite, deterministic)

**Building a genuine hot journal inside the test:**
1. Seed through the crash child (`after_fixture_1`).
2. Open connection A on `meta.sqlite3` with `PRAGMA cache_size=1` (so pages spill to the database file mid-transaction).
3. `BEGIN IMMEDIATE`, then insert many forged non-fixture `authority_objects` rows, so uncommitted pages reach the database file.
4. Without committing, copy `meta.sqlite3` and `meta.sqlite3-journal` over a second, identical crash-child vault.
5. Roll A back.

The copy now has a valid hot journal and contains uncommitted forged rows.

**Expected results:**

| Case | Expected |
|---|---|
| The hot-journal copy above | Recovery settles it, proves fixture-only, rebuilds; exactly 12/12 and no forged row |
| The same, but with a **committed** forged row added before the uncommitted one | Settled, then refused with `seed_recovery_refused`; the committed forged row is still present |
| A junk journal with an invalid header | Settled (SQLite ignores it), then proceeds as fixture-only. The existing junk-journal test changes its expectation from `seed_ledger_interrupted` to recovered |
| The ledger held BUSY by another connection while a journal exists | `workspace_busy`; nothing settled |

## Open questions for review

1. Is a read-write open plus one read under the held `WriterLock` exactly equivalent to Core's own open, given that Core also runs migrations and integrity checks?
2. On Windows, can SQLite's rollback fail because the journal file is held by antivirus or similar software, and is mapping that to `Io` correct?
3. Is the test's copy-while-uncommitted technique a faithful hot journal on both Windows and macOS, given that `std::fs::copy` reads the database while connection A holds a RESERVED lock?
