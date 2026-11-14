# EXACT_RANGE_REVIEW — Spec 075 (draft; finalized at exact head)

## Canonical base and branch

```text
BASE = ae0441918296c2d1510a71061249c7e55e65d760 (PR #128 merge)
BRANCH = spec/075-data-source-fabric
```

## Authorized scope

`docs/planning/SPEC_075_PROMOTION.md` (only Spec 075: Data Source Fabric +
Data Workbench Foundation; 076+ explicitly not authorized).

## Changed files (at PR #129 head; verify with git diff at close)

Product code:

```text
crates/medscale-contracts/src/data_sources.rs (new)
crates/medscale-contracts/src/envelopes/mod.rs (14 capabilities, 18 requests, 14 responses, Cancelled)
crates/medscale-contracts/src/lib.rs (module registration)
crates/medscale-contracts/src/network/mod.rs (DatasetMirrorRead purpose only)
crates/medscale-contracts/tests/data_sources_075.rs (new)
crates/medscale-storage/src/data_sources.rs (new: v4 rows + external SQLite adapter)
crates/medscale-storage/src/sqlite_meta.rs (v4 migration + snapshot_bytes v4)
crates/medscale-storage/src/backup.rs (restore_v4, manifest v4)
crates/medscale-storage/src/lib.rs (module + type exports)
crates/medscale-storage/tests/data_sources_075.rs (new)
crates/medscale-core/src/authority/data_sources.rs (new: authority paths)
crates/medscale-core/src/authority/data_acquire.rs (new: parsers/materialize/remote)
crates/medscale-core/src/authority/facade.rs (ds() helper + 18 arms + capability pairs)
crates/medscale-core/src/authority/mod.rs (module registration)
crates/medscale-core/src/cli_session.rs (16 wrappers)
crates/medscale-core/src/authority/data_acquire.rs unit tests (inline)
crates/medscale-core/tests/data_sources_075.rs (new)
crates/medscale-core/Cargo.toml (rusqlite dev-dependency for fixture creation only)
crates/medscale-cli/src/data_source.rs (new: 5 command groups)
crates/medscale-cli/src/main.rs (wiring)
crates/medscale-cli/src/project.rs (Cancelled arm only)
crates/medscale-desktop/src/data_workbench.rs (new: view-models + tests)
crates/medscale-desktop/src/main.rs (Data route wiring + actions)
crates/medscale-desktop/src/project_workspace.rs (Cancelled arm only)
crates/medscale-desktop/ui/app.slint (Data structs/properties/nav/route)
```

Planning/spec/evidence (authorized promotion + package + evidence):

```text
docs/planning/SPEC_075_PROMOTION.md (new)
docs/planning/BUILD_QUEUE.md (075 row + footer)
specs/075-data-source-fabric/ (7 files)
evidence/075-data-source-fabric/ (this packet)
```

## Unexpected files

```text
NONE EXPECTED. Verify at close: git diff --name-only BASE...HEAD must equal
the union of the two lists above. Any other file is a blocking defect.
```

## New dependencies

```text
PRODUCT DEPENDENCIES ADDED: NONE.
- No new [dependencies] in any crate (verified via Cargo.toml diff + cargo-deny).
- medscale-core [dev-dependencies] gains workspace-pinned rusqlite (same
  qualified 0.37.0 already in Cargo.lock) for external-SQLite fixture
  creation in tests only. No product edge: Core reaches SQLite exclusively
  through medscale-storage. Dependency-direction gate stays green.
```

## Secret scan

```text
- credential_ref is OpaqueId-only; no password/token literal exists in new code.
- CLI JSON outputs serialize Core results; scanned for secret markers at close.
- Fixture data is synthetic (town names/doses); no PHI, no licensed content.
```

## Semantic pass (2026-09-21, before merge)

A full exact-range review (`/code-review high`, base `ae04419...`, head at
the time `bbc4d89`) was run over the complete PR diff before merge, not just
formatting. It ran 8 finder angles plus a verification pass over the whole
changed surface listed above (product code, storage schema, Core authority
paths, CLI, Desktop) and returned 18 CONFIRMED findings. All are
correctness/security defects that were already present in the implementation
this review inspected — none were introduced by this review pass itself.

Ten were ranked material enough to report; three more were named as
additional confirmed findings dropped only for the report's severity cap;
one more (RemoteDataset `".."` traversal in `SourceLocator::validate()`) was
confirmed real but currently unreachable, since remote fetch is gated behind
`UreqTransport::send` unconditionally returning `ExternalGateRequired` in
this repository state.

Disposition, fixed forward in `aa771e9` unless noted:

1. **Authority bypass / cross-scope leak (FIXED).** `resolve_local_path`
   only checked containment within the vault root, and the vault's own
   `meta.sqlite3` lives inside that root next to user-placed files. A
   `Database` source naming `meta.sqlite3` could run an unscoped read
   against the app's own metadata store — every project's
   `data_sources`/`data_snapshots`/`dataset_releases` rows, no
   authority-scope filtering. Now refused via a reserved-path check before
   resolution. Regression: `database_source_cannot_name_the_vaults_own_metadata_store`.
2. **Deferred creation-time validation (NOT A DEFECT — reviewed, confirmed
   intentional).** `create_source` never calls `resolve_local_path`, so a
   `LocalPath`/`Database` locator that will fail at acquisition time is
   still accepted and listed Active/Healthy at creation. This is the same,
   already-tested design as format/engine qualification below (#10): create
   is structural-only (`locator.validate()`), acquisition is where
   fail-closed enforcement lives. Left unchanged.
3. **Credential reintroduction on restore (FIXED).** `restore_v4` re-validated
   a restored `DataSourceManifest`'s name/locator/capabilities/revision but
   not "no stored credential for a Database source", which
   `create_source`/`update_source` both enforce. A crafted/tampered backup
   could reintroduce a stored DB credential. Regression:
   `restore_rejects_database_source_with_credential_ref`.
4. **Wrong-snapshot bug (FIXED).** `list_snapshots` is keyset-paginated
   oldest-`snapshot_id`-first; its doc comment claimed "newest first" and
   Desktop's `open_data_source` took `.first()` as the latest, so opening a
   refreshed source showed the oldest snapshot's schema/rows. Doc comment
   corrected; Desktop now takes `.last()`. No automated regression — see
   "Desktop coverage gap" below.
5. **Cross-project stale selection (FIXED).** `open_project_detail` never
   cleared `data_active_source`/`data_active_snapshot`/the data lists on a
   project switch, so a subsequent data action could mutate the previous
   project's data source while the UI showed the newly opened one. Now
   cleared on every switch. No automated regression — see below.
6. **Resource safety (FIXED).** `acquire_parsed` read a `LocalPath` file
   fully into memory before `parse_delimited`/`parse_json_bytes` ever
   checked `SNAPSHOT_BYTES_MAX`, so an oversized local file was fully read
   despite the documented bounded-parsing guarantee. Now rejected via file
   metadata length before the read. Regression:
   `oversized_local_file_is_rejected_before_full_read` (uses a sparse file
   via `File::set_len`, so the test itself stays cheap in CI).
7. **Lexicographic id-string ordering (NOT FIXED — recorded as a tracked,
   cross-cutting gap, not this spec's to fix).** Every `list_*` keyset
   pagination function orders `WHERE id > cursor ORDER BY id` on the raw
   `"prefix-N"` id string, which is wrong once `N` reaches two digits
   (`"dsrc-10" < "dsrc-2"` lexicographically). This is a pre-existing
   pattern shared by every `list_*` function since at least Spec 074/016
   (`project_graph.rs`'s `get_next_seq`/`alloc_id` share the same shape),
   not introduced by or unique to Spec 075. Fixing it means a cross-cutting
   id-generation/ordering change spanning multiple already-closed specs —
   out of this spec's bounded scope per its own promotion package. Flagged
   here for whichever future spec owns id-scheme maintenance.
8. **Data honesty / silent truncation (FIXED).** `list_releases` discarded
   the real pagination cursor from `meta().list_dataset_releases` and always
   returned `next_cursor: None`, so a project with more releases than one
   page silently looked complete. Now returns the real cursor. Regression:
   `release_list_pagination_cursor_is_not_silently_discarded`.
9. **TOCTOU between check and use (NOT FIXED — recorded, disproportionate
   for the threat model).** `resolve_local_path` canonicalizes and checks
   containment once; the later `fs::read`/`rusqlite::Connection::open` is a
   separate syscall against the same path string. A symlink swapped in
   between by a concurrent local process could escape the check. This
   requires an already-privileged local attacker (able to write inside the
   vault directory while a request is in flight), at which point materially
   worse attacks are already available. Full elimination needs handle-based
   (not path-based) reads on both the file and rusqlite paths; not attempted
   here.
10. **Deferred format/engine qualification at creation time (NOT A
    DEFECT — already covered by an existing, passing test).** Same shape as
    #2: `create_source` never checks whether a format/engine is qualified in
    075 (Parquet/ArrowIpc/Xlsx/Postgres/Mysql/SqlServer parse but are
    rejected only at acquisition). This is exactly what
    `unqualified_formats_and_engines_fail_closed` already tests and asserts
    as intended behavior. Left unchanged.

Additional findings named but dropped from the top-10 report, all addressed:

- **Reserved-path check duplicated instead of reusing
  `medscale_storage::claim::assert_claim_path`'s cloud-sync-root refusal
  list.** Evaluated and not applied: the vault root itself is already
  `assert_claim_path`-checked at vault-open time, and any resolved path that
  escapes the vault root already fails the existing containment check
  (`starts_with(&root)`) regardless of whether it happens to contain a sync
  marker string. The incremental value of also running
  `assert_claim_path` on the post-containment-checked path was judged
  negligible (would only matter for a symlink target that both stays inside
  `root` post-canonicalization and coincidentally contains a marker
  string). The reserved-path fix (#1) is the change that actually mattered
  here.
- **`import_health_hook` vs `refresh_source` classified
  `Quarantined`/`Rejected`/`Unsupported` inconsistently (FIXED).**
  `refresh_source` already marked these `SourceHealth::Stale`;
  `import_health_hook` left health untouched (silently `Healthy`) for the
  same failure classes. Now mirrors `refresh_source`. Regression:
  `quarantined_import_marks_source_health_stale_not_silently_healthy`.
- **`alloc_data_fabric_id` silently reset to `1` on a corrupted sequence
  (FIXED).** `.unwrap_or(0)` on a parse failure meant a corrupted
  `store_state` row silently restarted the sequence, risking a colliding id
  with one already issued. Now returns `MetaError::CorruptObjectBody`.
  Regression: `corrupt_id_sequence_fails_closed_instead_of_resetting`. (The
  differently-named, pre-existing `get_next_seq` from Spec 016 has the same
  shape but belongs to that spec, not this one, and is untouched.)
- **`SourceLocator::validate()` gap for `".."` in `RemoteDataset` file names
  (FIXED, defense in depth).** Confirmed real but currently unreachable
  (`UreqTransport::send` always returns `ExternalGateRequired`). Fixed
  anyway since it is cheap, correct, and closes the gap before remote fetch
  is ever wired to something real. Regression:
  `remote_dataset_file_names_reject_traversal_and_absolute_paths`.

### Desktop coverage gap (honest, not fabricated)

Findings #4 and #5 above are fixed in `crates/medscale-desktop/src/main.rs`
but have no automated regression test: this codebase's Desktop crate has no
headless-`AppWindow` test harness (`AppWindow::new()` appears only in the
real `main()`, never in any `#[cfg(test)]` module across the whole crate,
confirmed by grep at review time). Building one is infrastructure work
disproportionate to a review-response fix and is not attempted here. Both
fixes were verified by tracing the exact call chain (property flow through
`app.slint`'s `in`/`in-out` property declarations, matched against the
Rust setter calls) rather than by execution. This gap should be closed by
whichever future spec adds Desktop UI-logic test infrastructure, not
silently treated as covered.

### Local verification constraint (unchanged from earlier commits)

This machine's MSVC toolchain is incomplete (VC Tools directory present but
`link.exe`/`cl.exe`/Windows SDK absent), so nothing in this PR has been
compiled or run locally end-to-end. Every fix above was traced against the
actual call sites and existing error-mapping chains by reading, not
assumed; `cargo fmt --check` (which does work locally without linking) is
clean. Real, multi-platform verification is exact-head GitHub Actions CI —
see the run recorded in `LIVE_TRUTH.md` / `BUILD_QUEUE.md` at close.
