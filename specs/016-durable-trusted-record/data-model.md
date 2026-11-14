# Data model: Spec 016

## Schema version

| Version | Meaning |
|---:|---|
| 1 | Spec 003/005: `sources` + `gc_marks` + journal |
| 2 | Spec 016: + `authority_objects` + `store_state` |

`schema_version` / migration journal version **≠** clinical `effective_time` / `recorded_time`.

## Tables

### `authority_objects`

| Column | Type | Rules |
|---|---|---|
| `object_id` | TEXT PK | OpaqueId string; stable across restart |
| `object_class` | TEXT NOT NULL | `source` / `derived` / `proposal` / `assertion` / `audit` / `identity` / `merge` / `evaluation` / `projection` |
| `realm_id` | TEXT NOT NULL | Indexed with scope |
| `authority_scope_id` | TEXT NOT NULL | Indexed with realm |
| `body_json` | TEXT NOT NULL | serde JSON of contract type; Source/Derived omit or empty `bytes` array and rely on digest |
| `content_digest_hex` | TEXT NULL | Required for source/derived |
| `updated_seq` | INTEGER NOT NULL | Monotonic store seq at write |

Indexes:

- `(authority_scope_id, object_class)`
- `(realm_id, authority_scope_id, object_id)` unique already by PK + check on read

### `store_state`

| key | value |
|---|---|
| `next_seq` | integer as text |
| `schema_note` | optional |

### `sources` (unchanged semantics)

Remains the content-addressed visibility index for ingest/GC. Every visible Source authority object MUST have a matching `sources` row; digest unique per scope.

## Object body rules

- Source/Derived: `bytes` in JSON MAY be empty on disk; loader fills from blob store after digest verify.
- Proposal/Assertion/Evaluation/Identity/Merge/Audit/Projection: full serde body including nested values.
- Class column MUST match deserialized type; mismatch → refuse load of that row.

## Transactions

Single SQLite transaction per canonical promotion grouping:

- PromoteProposal: insert Assertion + Audit (+ optional Projection rebuild) atomically.
- Ingest accept: blob finalize + sources row + Source authority_object + optional Identity/Evaluation/Audit atomically after blob stage.
- TransitionEffect: update audit body in place only for effect_state field via rewrite of JSON in transaction (append-only preferred if effect changes require lineage—016 allows in-place effect_state update matching current memory semantics, documented).

## Soft delete / overwrite

- No DELETE of source history for corrections; corrections are new objects with lineage.
- `insert` of duplicate `object_id` is reject/replace only for effect_state updates on same audit id as today’s `get_audit_mut`.
