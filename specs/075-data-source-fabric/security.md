# Security and Threat Delta — Spec 075

**Status:** implementation contract

Spec 075 adds governed data-source acquisition (local tabular files, read-only databases, brokered remote datasets), immutable snapshots, saved views, and deterministic transformations. It adds no new product runtime network path beyond the existing fail-closed Network Broker, no remote worker, no model execution authority, no team sync, and no real-PHI authorization.

## 1. Trust boundary

The authority path remains:

```text
CLI/Desktop
   -> typed Core command/query
      -> actor/session/scope checks
         -> capability/authority check
            -> input + revision validation
               -> source-locator/credential-ref validation
                  -> brokered/quarantined acquisition
                     -> encrypted storage transaction
                        -> audit/receipt/result
```

No CLI/Desktop component may access 075 tables, snapshot blobs, credential material, database drivers, or download caches directly. Database drivers and download code run behind Core, never in the Desktop/UI process authority.

## 2. Threats and required controls

### T1 — Hostile tabular input

Attack: malformed CSV/TSV/JSON/Parquet/Arrow/XLSX (control characters, invalid UTF-8, ragged rows, type-confusion cells, billion-row resource bombs, zip-bomb-compressed XLSX, formula-injection strings) corrupts storage, exhausts memory/disk, or executes on render.

Controls:

- bounded streaming parse with explicit row/byte/cell limits; oversized inputs fail closed, never OOM;
- strict encoding policy (invalid UTF-8 is rejected or explicitly repaired-and-counted per frozen policy);
- validation before materialization; failures quarantine with reasons, never half-materialize;
- decompression ratios bounded for compressed formats;
- cell strings render as text in UI/CLI, never as executable markup/formulas;
- logging safely escapes/bounds user data.

Tests: malformed fixtures per admitted format, max-boundary and over-max inputs, hostile Unicode/control characters, ragged rows, null-byte injection, formula-like strings, oversized single cell/row/file.

### T2 — Credential exfiltration and secret logging

Attack: database passwords, API tokens, or keystore material leak into logs, receipts, snapshots, backups, error messages, or crash dumps.

Controls:

- opaque credential references only; plaintext credentials never enter manifests, receipts, snapshots, logs, backups, or error strings;
- credential resolution at acquisition time through the admitted keystore path, held in memory only for the operation;
- secret-marker scans over test log output and persisted fixtures;
- error messages name the credential slot, never its value.

Tests: revoked/rotated credential flows, debug-log capture scans, persisted-artifact scans, error-string assertions across denied/timeout paths.

### T3 — Database adapter escape from read-only

Attack: crafted source configuration or query text performs writes, DDL, stacked statements, or reads outside the admitted schema.

Controls:

- adapters open connections with read-only enforcement at every layer available (connection flags, role, explicit statement allow-list of SELECT-only single statements);
- no free-SQL product path in 075: queries are explicit typed/bounded operations recorded by identity in receipts;
- stacked statements and multi-statement strings rejected;
- bounded schema discovery (table/column allow-lists, row-count caps on preview);
- every acquired row set binds to an immutable snapshot with query identity recorded.

Tests: write/DDL/multi-statement attempts denied, out-of-scope table access denied, unbounded discovery capped, query-identity recorded in receipt.

### T4 — SSRF and private-network exfiltration via remote datasets

Attack: crafted dataset identifier/URL reaches internal hosts, file URLs, or cloud metadata endpoints; response content exfiltrates vault material on retry/upload.

Controls:

- all remote acquisition flows through the existing Network Broker allowlist/transport decision; no direct HTTP client in adapters;
- no upload/competition-submission path exists; adapters are download/read-only by construction;
- private-network/file-scheme destinations denied by broker policy;
- response bytes treated as hostile: hash-verified, quarantined, admitted only after validation.

Tests: private-network/file/metadata destinations denied, upload-shaped calls absent (static + behavioral), tampered/corrupt downloads quarantined, digest mismatch rejected.

### T5 — Trusted remote code execution

Attack: dataset loader scripts, notebook payloads, or archive members execute during import (pickle payloads, loader hooks, macro sheets, external entity expansion).

Controls:

- no trusted remote code: never execute loader scripts, notebook code, macros, or deserialized executable formats during acquisition;
- data-only parsing with XXE/DTD expansion disabled where applicable;
- archives (if admitted) enumerated with traversal guards, member caps, and no permission/symlink trust.

Tests: script-bearing fixtures never execute, XXE/entity-expansion fixtures fail closed, path-traversal archive members rejected, symlink members ignored.

### T6 — Snapshot identity confusion / stale rewrite

Attack: changed external content is presented as the old snapshot, or two different contents share one snapshot identity.

Controls:

- snapshot identity binds exact source revision (digest/length/version/files); content digest verifies on read from blob stores;
- refresh always creates a new snapshot or an unchanged receipt, never an in-place edit;
- digest mismatch on read yields `Corrupt`, never best-effort data.

Tests: mutate source, prove old snapshot byte-identical; corrupt blob bytes, prove `Corrupt` on read; identical re-import yields identical digest.

### T7 — Transformation as code-execution smuggling

Attack: computed-expression or join parameters smuggle arbitrary code, unbounded computation, or cross-snapshot authority confusion.

Controls:

- frozen bounded op set only; expressions (if admitted) are side-effect-free, total, and versioned in `ops_digest`;
- resource bounds on transformation execution (row caps, expression complexity caps);
- output is a new snapshot with exact lineage; failed ops never produce silent partial outputs;
- no eval/exec/FFI/plugin hook anywhere in the transform path.

Tests: unknown op rejected, over-bound inputs fail closed, cast failures explicit, lineage replay byte-identical.

### T8 — Stale-write overwrite on mutable 075 state

Attack: two callers update a source manifest or saved view from the same old state and the later request silently overwrites the newer result.

Controls:

- explicit expected revision/precondition on mutable 075 rows;
- transaction verifies revision atomically with write;
- mismatch -> `Conflict`;
- no generic last-write-wins.

Tests: stale manifest update, archive/update race, view update race.

### T9 — Metadata injection

Attack: oversized/control-character/markup/SQL-like source names, view names, or dataset-card text cause storage/query/UI issues.

Controls:

- bounded UTF-8 validation with explicit constants;
- parameterized storage APIs;
- UI renders text as text, not executable markup;
- logging safely escapes/bounds user metadata.

Tests: empty/whitespace, max boundary, over max, unusual Unicode, control characters per frozen policy, SQL-like strings.

### T10 — Information leak through views/summaries

Attack: actor cannot read a source but learns existence/content/counts from view state, summary aggregates, or error messages.

Controls:

- authorization/scope filtering before returning source/snapshot/view metadata, counts, and aggregates;
- denied resolution does not reveal locator/credential details;
- cache, if any, is scoped to the same authorization context; 075 prefers no new sensitive shared cache.

Tests: mixed visible/denied sources, summary counts, error-string content assertions.

### T11 — Cascade deletion / ownership confusion

Attack: archiving a source or deleting a view destroys snapshots, lineage, or canonical patient/document/evidence/model data.

Controls:

- source references are non-owning toward canonical targets; snapshots own only their own bytes/lineage;
- no cascade from source/view tables into canonical stores or into snapshot history;
- GC cannot treat absence of views as proof snapshots are unneeded;
- destructive source erasure is not a 075 feature.

Tests: archive/detach/view-removal leaves snapshots, lineage, and canonical targets intact and resolvable.

### T12 — Half-committed authority after crash

Attack: snapshot metadata commits but bytes do not (or the reverse), or a receipt/index partially persists.

Controls:

- snapshot materialization is atomic from the reader's perspective; content-addressed blobs verified on read;
- interrupted acquisitions leave no visible snapshot or an explicitly corrupt/partial one, never a silently complete one;
- migration journal fail-closed semantics from the existing framework are preserved for v3 → v4.

Tests: crash-point matrix from `migration.md` section 6, including mid-stream byte kill and mid-download kill.

### T13 — Dependency and supply-chain smuggling via new parsers/drivers

Attack: a new CSV/Parquet/Arrow/database/HTTP dependency introduces copyleft/transitive risk, build-script execution, or unreviewed network behavior.

Controls:

- every new dependency requires exact version/license/security/exit review recorded in the evidence packet before admission;
- `cargo-deny` and supply-chain gates must pass on the exact head;
- build scripts of new dependencies are reviewed for network/filesystem behavior;
- dependency-direction gate continues to hold (no CLI/Desktop → storage/driver edges).

Tests: deny/supply-chain/direction gates green; dependency review record complete.

## 3. Explicit non-capabilities

075 introduces no capability for: arbitrary code execution, free SQL, remote upload, cross-realm reads, ambient credential access, background sync, or silent cloud fallback. Any test or review finding suggesting such a capability is a blocking defect, not a scope question.
