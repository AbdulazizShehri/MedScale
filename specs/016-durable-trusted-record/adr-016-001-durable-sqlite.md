# ADR-016-001: Durable authority metadata storage

**Status**: Accepted for Spec 016 implementation  
**Date**: 2026-09-09  
**Context**: Q02 requires full trusted-record persistence across process restart without a second authority service.

## Decision

1. **Retain one SQLite dependency family** (`rusqlite`) as the sole canonical metadata store, extended in-place from Spec 003/005 `SqliteMetaStore`.
2. **Add schema version 2** tables:
   - `authority_objects(object_id PK, object_class, realm_id, authority_scope_id, body_json, content_digest_hex NULL, created_schema INTEGER)`
   - `store_state(key PRIMARY KEY, value)` including `next_seq`
   - Keep existing `sources`, `gc_marks`, `migration_journal`
3. **Do not introduce SQLCipher or a second database in Spec 016.**
4. **Large source/derived bytes** remain in the blob store; JSON bodies store envelopes with digests; hydrate on load.
5. **EncryptedVault**: continue existing seal-at-close of the working DB file so new tables are covered when sealed; do not claim private-data readiness.
6. **Writer lock**: OS-exclusive lock file `{vault_root}/writer.lock` held while vault is open for write.

## Consequences

- Positive: minimal architecture change; transactional integrity; reuse migration journal; EncryptedVault seal path unchanged.
- Negative: synthetic/open working DB remains plaintext while unlocked—explicit limitation; Q03 required before PRIVATE claims.
- Migration risk: v1→v2 must preserve existing `sources` rows and create authority_object rows for known sources where possible (envelope reconstruction from `sources` + blob bytes).

## Alternatives rejected

| Alternative | Why rejected now |
|---|---|
| SQLCipher now | Unqualified build/platform cost; belongs to Q03 comparison |
| Per-row app encryption only | Premature without Q03 key/query design |
| Separate object DB | Second authority surface |
| Persist only sources (status quo) | Does not solve Q02 |

## Reversibility

Backup schema v2 + JSON bodies provide export escape. Replacing SQLite later would require a new ADR and dual-read migration; interface should sit behind `SqliteMetaStore` / store trait methods used by `CoreFacade` only.
