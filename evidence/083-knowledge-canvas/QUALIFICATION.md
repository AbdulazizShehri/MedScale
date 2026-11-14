# Qualification — Spec 083 Knowledge + Research Canvas

Exact-head CI for the final code head is recorded in
`EXACT_HEAD_QUALIFICATION.md`; the counts below are test inventories.

All sources are synthetic. Retrieval is lexical only (`lexical-v1`); no
vector, embedding or model ranking exists, and "insufficient evidence"
means no lexical match among current spans.

## Contracts (`crates/medscale-contracts/src/knowledge.rs`, 6 tests)

`vocabularies_round_trip_and_are_closed` (the only retrieval stage is
`lexical`; `vector` does not parse), `spans_match_their_source_kind`,
`chunks_and_manifests_hold_their_invariants` (text length equals span,
sorted unique sources, known tokenizer, stable digest),
`receipts_state_insufficient_evidence_honestly` (results exactly when a
current span matched; stale hits only with `include_stale`; ranks 1..n;
query digest), `canvas_revisions_and_ops_validate` (unique keys, no dangling
or self edges, removing a node removes its edges, key syntax, title),
`index_status_is_fresh_only_when_nothing_is_stale_or_missing`.

## Storage v12 (`crates/medscale-storage/tests/knowledge_083.rs`, 8 tests)

`migration_v11_to_v12_is_additive` (pre-083 state unchanged; idempotent
reopen), `crash_mid_v12_migration_fails_closed_and_backup_recovers`,
`index_versions_are_atomic_contiguous_and_digest_checked` (wrong digest,
skipped version and foreign-source chunk write nothing; edited or deleted
chunks are refused on read), `receipts_and_canvas_revisions_hold_their_invariants`,
`consistency_check_detects_invariant_breaks`,
`backup_restore_roundtrips_every_083_row_exactly`,
`restore_rejects_hand_edited_083_snapshots` (17 cases: chunk text changed,
chunk dropped, index version removed, receipt names another digest, receipt
query edited, canvas revision removed, canvas in another scope, duplicate
receipt, receipt family replaced by a string, canvas family dropped, index
versions replaced by an object, chunk replaced by a number, receipt hit
names another chunk, receipt hit names no chunk, receipt hit span moved, receipt hit names another project's source,
receipt hit replaced by a string; every
receipt hit must be a chunk of its index span for span; a family that is
not an array is refused for every schema, and a v12 snapshot must carry all
three knowledge families), `pre_083_v11_backup_restores_with_empty_knowledge_tables`.
The Spec 078, 079, 080, 081 and 082 storage suites drop the v12 tables when
they rewind and pass with `CURRENT_META_SCHEMA_VERSION = 12`.

## Core (`crates/medscale-core/tests/knowledge_083.rs`, 6 tests; 3 unit tests)

| Behavior | Test |
|---|---|
| Index pins the latest snapshot, the corrected transcript revision (not the earlier one), the web evidence item and the derived table; status fresh; a search matches all four kinds with live text; ranking is repeatable; the receipt reads back; a rebuild of unchanged sources has the same chunk digest; no match is `insufficient_evidence` | `the_index_pins_exact_sources_and_search_returns_live_spans` |
| A re-import makes the indexed snapshot superseded and the new one unindexed; default search does not serve it; `include_stale` lists it labelled with its pinned text; archiving tombstones it and hides its text; a rebuild is fresh again | `stale_and_archived_sources_are_reported_not_served` |
| No index, empty query, too long, too many terms, hit limit 0 and 51 are refused with receipts; SQL- and markup-shaped queries are words | `refused_retrievals_leave_receipts` |
| Project B finds nothing of Project A and cannot cite it; another scope cannot read receipts, canvases or status, or search | `projects_and_scopes_never_leak` |
| A Canvas stores references, not text; live resolution; unsupported notes; contradictions; stale writer refused; forged span refused; superseded and missing evidence after a correction and an archive; earlier revisions readable | `a_canvas_holds_live_references_and_inspects_its_evidence` |
| Index, receipts and canvases survive reopen; search after reopen matches | `knowledge_state_survives_reopen` |
| Unit: tokenizing and character windowing (including Arabic text), hand-computed integer scores, numeric id ordering | `authority::knowledge::tests::*` |

## CLI (`crates/medscale-cli/src/knowledge.rs`, 1 test)

`knowledge_commands_run_through_core_across_fresh_sessions`: status without
an index, build, search and receipts in human and JSON form, canvas create,
a malformed `--ops-json` refused, an edit, a citation from a stored receipt
hit, a stale citation refused, and canvas show and list in both forms,
across fresh Core sessions in one test process.

## Desktop (`crates/medscale-desktop/src/knowledge_workspace.rs`, 1 test)

`knowledge_view_models_flow_through_a_real_core_session`: status before and
after a build, a refused search without an index, a labelled current match,
insufficient evidence, the canvas list, and a missing Project reported as
missing. Desktop has no storage or index code.

## Not demonstrated (recorded, non-blocking)

- No rendered Desktop screenshot (no CI rendering step); the Desktop route
  searches and lists canvases, while canvas editing is CLI-only.
- Retrieval and build cost at the chunk bound is unmeasured.
- No literature library, graph expansion, or semantic matching (out of
  scope by promotion).
