# Spec 016 contracts overview

- [field-mapping.md](field-mapping.md) — memory → durable map
- Error taxonomy (implement in contracts/storage as typed errors):
  - `WriterHeld`
  - `MigrationIncomplete`
  - `ContentMissing`
  - `DigestMismatch`
  - `ScopeMismatch`
  - `CorruptObjectBody`
  - `BackupIncomplete`
  - `RestoreRefused`

Stable string codes must remain serde-stable once introduced.
