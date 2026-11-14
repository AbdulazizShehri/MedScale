# Migration and Recovery — Spec 079

Additive: storage schema v7 -> v8. No pre-079 table or identity changes.

## Tables

```text
privacy_classifications   (artifact_id PK within project; mutable, revisioned)
privacy_profiles          (mutable, revisioned; rules as validated JSON)
privacy_deid_receipts     (insert-once; status/revision mutable only for revocation)
privacy_pseudonym_maps    (mutable, revisioned; key_account only)
privacy_pseudonym_entries (insert-once; sealed value blob; unique (map_id, pseudonym))
privacy_reid_audit        (append-only)
privacy_egress_decisions  (append-only)
```

No foreign keys (repository convention since Spec 074); cross-row invariants
are checked by `verify_privacy_gate_consistency` on open and restore:

- a `DeidReceipt`-basis classification names an existing receipt whose
  output artifact is that artifact;
- a receipt's profile and pseudonym map exist in the same Project;
- a map's `entry_count` equals its entry rows;
- audit and decision rows reference existing maps/receipts when they name one.

## Recovery

- Crash mid-migration fails closed; the pre-migration backup restores.
- Uncommitted 079 writes are absent after reopen.
- A v7 backup restores into v8 with empty 079 tables.
- Backups contain sealed entries and key account names, never keys. After
  restore on a machine without the key, maps report `KeyUnavailable` on
  re-identification (denied and audited as `denied_key_unavailable`).
  Egress and non-pseudonymizing transforms still work; a pseudonymizing
  transform on that map is refused because its key is unavailable.

## Write order and crash behavior of a transform

A transform commits its receipt, output classification and sealed entries in
one metadata transaction, then Core persists the new derived artifact with
the rest of the authority store at the end of the request. A crash between
the two leaves a receipt whose output artifact is missing; egress then
recomputes the output digest, cannot find the artifact, and denies
(`DeniedDigestMismatch`). Nothing fails open.

## Schema version constant

`medscale_storage::CURRENT_META_SCHEMA_VERSION` (8) is the single top-version
constant. Earlier specs' storage tests assert against it instead of a pinned
literal, so an additive migration no longer requires editing them (the
pinned-literal pattern caused the Spec 078 CI regression fixed in
`d9c75a8`). Tests that rewind a vault to an earlier version drop every later
version's tables and journal rows.
