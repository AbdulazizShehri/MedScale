# CLI encrypted-vault seal fix (PR #178): closure

**State:** `CLOSED_CANONICAL`.

Independent CLI/Core correctness fix. It is not a numbered spec and not part of the 095–101 productization chain. Qualified under the active 2026-09-22 deterministic review policy; external semantic review was not required (founder decision, 2026-10-07).

## Defect

`medscale vault create` and `medscale vault open` returned without Core `CloseEncryptedVault`. While unlocked, the encrypted vault works in `meta.work.sqlite3`, which is SQLCipher page-encrypted (Spec 023). Skipping the close had two effects:

1. Writes made since the last seal could be discarded on the next open: Core treats a leftover work DB as a crash leftover and restores from `meta.sealed`.
2. The unsealed work DB, its SQLite sidecars and the `.writer.lock` lease remained on disk after the command returned. ADR-017-001 requires sealing and removal on close.

## Fix

- `CliSession::close_encrypted_vault` dispatches the existing `CloseEncryptedVault` capability.
- `vault create` and `vault open` seal before a success return. A seal failure fails the command. Output adds `vault_sealed: true`.
- Tests:
  - no work DB or lease after create;
  - open seals again and rejects a wrong passphrase;
  - a repeated open works;
  - writes made before close survive a reopen.

## Qualification

| Item | Value |
|---|---|
| Final head | `6356eced515b7aedcff854a61dbee35b3e112457` (2 files, +135 / −9; CRLF preserved in `main.rs`) |
| Exact-head CI | run `37556133627`: all six required jobs passed |
| Exact-range scope and traceability | PR #178 comment `6038178687` |
| Merge | normal merge commit `98c26aa3a0f919f4dfbb0c2adc27265a767631e4` (parents `1e2b7d9`, `6356ece`), 2026-10-07T12:45:10Z |
| Post-merge `main` CI | run `37623282105` on `98c26aa`: all six required jobs passed (Rust on Ubuntu, macOS and Windows; perf delivery-plan scale; cargo-deny; supply-chain policy) |

The PR #177 desktop branch adds a byte-identical `CliSession::close_encrypted_vault`. `git merge-tree` of that branch against `98c26aa` reports no conflict, and the merged tree has a single definition.

## Limitations

- The fix covers the CLI `vault create` and `vault open` commands. Other CLI paths that open an encrypted vault were checked; there are none on `main`.
- Seal behavior is verified by unit tests and exact-head and post-main CI on GitHub-hosted runners. No new privacy claim follows: `PRIVATE_DATA_READY=false` and `RELEASE_READY=false` are unchanged.
