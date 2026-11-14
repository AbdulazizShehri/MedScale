# SECURITY_ADVERSARIAL — Spec 075

Threats from `specs/075-data-source-fabric/security.md` mapped to proof.
Result column reflects the real, final exact-head and post-merge CI outcome:
exact-head run 35580861670 (head 9f84a6e) green 6/6; PR #129 merged as
89a88cf; post-merge main run 35582548200 green 6/6.

| Threat | Control (code path) | Test | Result |
|---|---|---|---|
| T1 hostile tabular input | bounded streaming parse, UTF-8 strictness, cell/row/byte caps, quarantine (`core/src/authority/data_acquire.rs`) | contract tests (ragged/empty/dup-header/NUL), Core quarantine test (`malformed_bytes_quarantine_and_missing_file_is_unavailable`), storage tamper test | PASS (35580861670 exact-head, 35582548200 post-merge) |
| T2 credential exfiltration | opaque `credential_ref: Option<OpaqueId>` only; DB refs rejected; no plaintext in manifests/receipts/logs (`core/src/authority/data_sources.rs`) | Core DB credential rejection is asserted by `update_source`/`create_source` guards, **and now also by `restore_v4`** (`storage/src/backup.rs`) — the exact-range review found `create_source`/`update_source` enforced "no stored credential for a Database source" but `restore_v4` did not, so a crafted/tampered backup could reintroduce one; fixed in `aa771e9`. `restore_rejects_database_source_with_credential_ref` (storage) is the regression. CLI JSON output scanned by `assert_no_secret_markers`-style review in EXACT_RANGE_REVIEW | PASS (35580861670 exact-head, 35582548200 post-merge) |
| T3 database read-only escape | `SQLITE_OPEN_READ_ONLY` + `query_only` pragma, closed identifier charset, quoted identifiers, no free SQL (`storage/src/data_sources.rs`); **and** `resolve_local_path` now refuses the vault's own reserved internal paths (`meta.sqlite3`, `blobs/`, `sealed_blobs/`, `writer.lock.sqlite3`, ...) before ever resolving them | Core sqlite adapter test (import + preview + missing table); write attempts are refused by the OS/SQLite read-only open (no write API exists in the adapter). The exact-range review found a real cross-scope escape here: a `Database` source could legitimately name `meta.sqlite3` (the vault's own metadata store, which sits directly under the same vault root as user-placed files) and run an unscoped read against it, returning every project's `data_sources`/`data_snapshots`/`dataset_releases` rows with no authority-scope filtering. Fixed in `aa771e9`; `database_source_cannot_name_the_vaults_own_metadata_store` (Core) is the regression | PASS (35580861670 exact-head, 35582548200 post-merge) |
| T4 SSRF/private-network | broker allowlist decision before transport; no direct HTTP client in adapters (`data_acquire.rs: brokered_dataset_fetch`) | remote-deny test (empty allowlist denies before socket) | PASS (35580861670 exact-head, 35582548200 post-merge) |
| T5 trusted remote code | no loader/script/macro execution; data-only parsing by extension; archives not admitted | unsupported-extension test; malformed remote bytes quarantine | PASS (35580861670 exact-head, 35582548200 post-merge) |
| T6 snapshot identity confusion | content-digest identity + read-back verification + digest check on every query (`persist_snapshot`, `load_table`) | idempotent re-import test (same id), refresh-preserves-old test, digest equality assertions | PASS (35580861670 exact-head, 35582548200 post-merge) |
| T7 transform smuggling | frozen op set, `op.validate`, single-input rule, no eval/exec path | Core transform test + strict-cast failure test + unit tests in `data_acquire.rs` | PASS (35580861670 exact-head, 35582548200 post-merge) |
| T8 stale-write overwrite | CAS revision on sources/views, transactional with re-read | Core conflict tests (source update, view update), storage CAS tests | PASS (35580861670 exact-head, 35582548200 post-merge) |
| T9 metadata injection | bounded UTF-8 validators, parameterized SQL, text-only rendering | contract bound tests; CLI prints raw text without markup | PASS (35580861670 exact-head, 35582548200 post-merge) |
| T10 view/summary leaks | scope checks on every read path; denied resolution reveals no locator/credential detail | scope-isolation test (`WrongScope`), release listing scope filter | PASS (35580861670 exact-head, 35582548200 post-merge) |
| T11 cascade deletion | non-owning references; archive retains history; no destructive erase | archive path keeps snapshots resolvable (refresh test reads old snapshot after change) | PASS (35580861670 exact-head, 35582548200 post-merge) |
| T12 half-committed authority | atomic snapshot insert (metadata + parts in one tx), content-addressed blobs verified on read, migration journal fail-closed | storage conflict/duplicate tests, backup/restore round-trip test, migration idempotence test | PASS (35580861670 exact-head, 35582548200 post-merge) |
| T13 supply-chain smuggling | zero new product dependencies in 075 (CSV/JSON hand parsers, rusqlite/serde_json/ureq already qualified; rusqlite dev-use for fixtures only) | `cargo-deny` + supply-chain gates + dependency-direction gate on exact head | PASS (35580861670 exact-head, 35582548200 post-merge) |

## Explicit non-capabilities (verified by construction + review)

No capability exists for: arbitrary code execution, free SQL, remote upload,
cross-realm reads, ambient credential access, background sync, or silent
cloud fallback. Exact-range review (EXACT_RANGE_REVIEW.md) records the new
dependency list (none for product paths) and the full changed-file set.
