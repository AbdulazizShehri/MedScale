# Spec 108 step 4: automatic recovery of an interrupted first seed

**Status:** `NOT READY TO IMPLEMENT`. The second Goldfish review (2026-11-02) confirmed the single-file pending → complete model. No code path lets a pending manifest coexist with a workspace that reached the UI, and older builds fail safe. Factor 4(b) and the test plan must be rebuilt as v3 before any code is written.

## v3 progress

- **Increment 1 (PR #408): implemented.** `seed_only_vault` is a read-only classifier built from a source-backed inventory of what open and seed write. It is **not wired to any action**.
- A fresh-context Goldfish code review found:
  - one compile blocker, now fixed;
  - no Core write path that a single-process user action can push through as `FixtureOnly`.

- **Increment 2: implemented** (M1 snapshot read, M2 typed exact shapes, M3 real-Core and crafted rejections). Remaining before wiring: the caller-held writer lock across classification and rename (increment 3); the hot-journal private-copy rollback (v3 M5); blob content hashing.

### Gates before `seed_only_vault` may authorise deletion (increment 2)

- **M1 — consistent snapshot.** Run every SQL read in one read transaction. The caller holds the instance lock and Core's `WriterLock` for `vault_root` from before the classification until the rename to `.discarding`, or re-classifies after taking the lock.
- **M2 — exact shape, not a subset.**
  - Decode each body with the typed contract struct (`deny_unknown_fields`: `Proposal`, `ClinicalAssertion`, `ActionAuditRecord`, `IdentityAssertion`, `SourceRecord`).
  - Require `header.id == object_id`, and the header's realm and scope equal to the columns. All rows in `authority_objects` and `sources` must share one realm and scope.
  - Ingest audit: `actor="ingest"`, `payload_digest` equal to its source's digest, `detail` and `effect_state` are `None`.
  - Promote audit: `actor="cli-operator"`, and `targets[1].promoted_from_proposal_id == targets[0]`.
  - At most one audit per source and one per assertion.
  - Assertion: `authorized_by="cli-operator"`, the epoch `recorded_time`, and `effective_time = None`.
  - Proposal: `confidence = None`.
  - Identity: exactly the `patient-golden` source with `http://hospital.example/mrn`/`P-004`.
- **M3 — database-level rejections driven by real Core.** Add tests for:
  - fixture bytes re-imported for the same subject (a deduplicated source, caught by `plan_used`);
  - fixture bytes imported for the other subject;
  - project or data-source creation;
  - a Pack evaluation;
  - an assertion amendment;
  - `CreateIdentityAssertion`.

  Also add hand-crafted rejections for: an unknown class, a second identity, a forged audit action, a non-empty `quarantine`, `tombstone/`, and a non-fixture `.tmp` blob.
- **Minor:**
  - Hash the blob contents, and allow a `.tmp` blob only if it is a prefix of a fixture.
  - `symlink_metadata(vault_root)` must not be a link.
  - Decide whether `gc_marks` is allowed. It is currently treated as user data, which fails safe.
  - `sources` rows must agree with any matching source object.

## Required for v3 (second review, source-verified)

- **B1 — the seed object allowlist is too narrow.** Seeding writes `source`, `audit` and `identity` objects and `sources` table rows, not only `proposal`/`assertion` (`ingest_ops.rs:124,128,158,186`; `promote.rs:60-61`). Define the exact producible set:
  - `source`: one per distinct fixture digest. There are 10, because `condition-uri` and `observation-weight` deduplicate.
  - `identity`: references fixture sources only.
  - `audit`: seed ingest and promotion actions only.
  - `proposal`: `payload == {"resource": fixture}`, evidence pointing to that fixture's source, at most one per plan entry.
  - `assertion`: at most one per proposal. A proposal without an assertion is allowed.
  - Any other class means unproven.
- **M2 — user activity lives outside `authority_objects`.**
  - Enumerate `sqlite_master` and require zero rows in every table except `migration_journal`, `store_state`, `sources` (fixture digests only), `gc_marks` and `authority_objects`.
  - Allowlist the vault folder contents: `meta.sqlite3` with its journal, `writer.lock.sqlite3` with its journal, and fixture-digest blobs.
- **M3 — leftover complete `.tmp`.** A complete-manifest `.tmp` next to a pending manifest means the commit happened. Treat it as unproven and never delete it.
- **M4 — crash tests need a real process.** `std::mem::forget` keeps Core's exclusive writer lock inside the same process. Run seeding in a child test process and kill it at each boundary, including mid-transaction.
- **M5 — hot journal.** A crash mid-transaction is the most likely crash, and it leaves a hot rollback journal. Either inspect a private copy of `meta.sqlite3` plus its journal, opened read-write so SQLite rolls it back while the originals stay untouched, or document that this case goes to manual recovery.
- **Minor:**
  - `SeedManifest` struct literal at `host.rs:1472`.
  - Add a `Fresh | Complete | Pending` preflight result.
  - `complete_synthetic_seed` and `import_fhir` refuse `pending`.
  - Retry the commit rename on Windows errors 5/32/33.
  - Name the stale sidecar in the refusal message.
  - Optionally bind `.discarding` to a nonce stored in the pending manifest.

## v2 design (kept for history)

**Depends on:**
- step 1 (`write_atomic`, PR #402);
- step 2 (reason codes, PR #404);
- step 3 (instance lock).

## Problem

If MedScale closes, crashes or loses power during the very first sample seed, the sample workspace cannot be opened again unless the user deletes app data by hand. Preflight refuses to replay, which is correct (`seed_manifest_missing`).

## What v1 got wrong (review B1/B2)

v1 used a separate pending marker file. The review showed that a marker can outlive the UI handoff through:
- an ignored removal failure;
- an unflushed deletion;
- mixed app versions, where an older build reseeds and never removes the marker.

Two of v1's proof conditions were also always true: `state.workspace` is cleared by `workspace_open_synthetic`, and the instance lock is always held in the app. A leftover marker combined with an externally deleted manifest could therefore have auto-deleted real imports. v1 also deleted a leftover `.discarding` folder on the strength of its name alone.

## Decision v2: one file moves atomically from pending to complete

1. **Pending manifest.** When preflight returns `Ok(SeedManifest::default())` (no manifest, absent or empty folder), the desktop first writes the seed manifest itself as `{"pending": true, "subjects": [], "sources": []}` with `write_atomic`. Only then is Core created or opened.
   - `SeedManifest` gains `#[serde(default)] pending: bool`.
   - Older builds parse the pending manifest as an empty roster and refuse safely:
     - folder with entries → `seed_manifest_incomplete`;
     - empty folder → `seed_vault_missing`.
2. **Commit.**
   - After all 12 fixtures are promoted, `seed_synthetic` replaces it, via `write_atomic`, with the complete manifest (`pending: false`). This is a single rename.
   - `state.workspace` is set only after that succeeds, so **no workspace reaches the UI while its manifest is pending**.
   - Every successful open and every user import already requires a complete, non-pending manifest. The preflight is extended so that `pending: true` is never treated as complete.
3. **Before writing the pending manifest:**
   - Refuse if any `.desktop-packs.json` or `.desktop-asr.json` exists without a manifest. Such files are stale records from an earlier workspace; they go to step 5 (review M5).
4. **Proof that the state is recoverable.** All of the following must hold, checked under the host lock inside `open_synthetic` before Core opens anything:
   - (a) The manifest exists, parses, and has `pending: true`.
   - (b) **Second, independent factor:** the Core ledger, read-only through `synthetic_core_ledger` helpers, is either absent or empty, or contains only:
     - `proposal` and `assertion` objects whose subject, claim kind and resource type match a bundled fixture;
     - at most 12 of each.

     An unknown object class, an unreadable ledger or a hot journal all mean *unproven*.
   - (c) Every `data_dir` entry whose name starts with `synthetic-workspace` is in this allowlist:
     - `synthetic-workspace/` (a real directory; `symlink_metadata`, with no symlink or junction/reparse point);
     - the manifest and its `.tmp`;
     - `synthetic-workspace.discarding/`.

     Any other sibling, including pack or ASR records, means unproven.
   - (d) Any filesystem error other than `NotFound` while checking (a)–(c) means unproven.

   If any check fails, refuse with the step 2 reason and leave every file untouched. The user goes to step 5.
5. **Recovery,** only when 4 holds:
   1. If `synthetic-workspace.discarding/` exists, `remove_dir_all` it first. `std` removes links without traversing them, junctions included (CVE-2022-21658 fix, toolchain 1.97.1); a test pins this. If that fails, return `workspace_io_failure` and stop. The manifest stays pending.
   2. If `synthetic-workspace/` exists, probe Core's `WriterLock`.
      - Any result other than `Ok` refuses: `WriterHeld` → `writer_active`, anything else → `workspace_io_failure`.
      - Drop the probe before step 3 on Windows.
      - The probe may create `writer.lock.sqlite3` inside the folder about to be discarded. That is acceptable.
   3. Rename `synthetic-workspace/` → `synthetic-workspace.discarding/`.
      - On Windows, retry error codes 5, 32 and 33 for up to 3 s.
      - Never fall back to copy plus delete.
      - On failure, return `workspace_io_failure` and stop.
   4. `remove_dir_all("synthetic-workspace.discarding")`, plus `fixture-staging/` and any manifest `.tmp`. On failure, stop. The manifest stays pending, so the next attempt resumes at step 1.
   5. Continue the normal first seed. The pending manifest is still in place and is reused, not rewritten.
   6. `workspace_open_synthetic` returns `recovered_interrupted_seed: true` alongside `vault_label`. A TS contract test covers it. The UI shows: "The sample workspace had not finished setting up and was rebuilt."
6. **No orphan deletion.** `synthetic-workspace.discarding/` is only ever deleted while the manifest is pending (5.1). If it exists while no manifest is pending, the open refuses and routes to step 5.

## Interruption tests (Rust, real Core)

Crashes are simulated with `std::mem::forget` on the session at the boundary, so SQLite is not closed cleanly. A test that only drops the session is labelled as such. Each test snapshots the full `data_dir` tree before and after.

1. Crash after the pending manifest, before Core creation → recovered (no folder), then 12/12, 2 subjects.
2. Crash after Core initialization, with 0 fixtures → recovered, then 12/12.
3. Crash after 1 fixture promotion → recovered, then 12/12, with no duplicate proposal.
4. Crash with a hot journal (forgotten session mid-write) → ledger unproven → refuse `seed_ledger_interrupted`, nothing touched.
5. Complete manifest written but crash before `state.workspace` is set → normal open, no recovery, 12/12.
6. Complete workspace, user FHIR import, then the manifest is deleted externally → `seed_manifest_missing`, **not recovered**, nothing touched.
7. A pending manifest whose ledger also contains a non-fixture proposal (an imported Patient) → refuse, nothing touched.
8. Unknown `synthetic-workspace.something` sibling, or a pack sidecar → refuse.
9. A junction or symlink inside `.discarding` pointing to a sentinel folder outside `data_dir` → the sentinel survives (Windows and Unix).
10. `.discarding` present without a pending manifest → refuse.
11. Writer lock held by another `CliSession` → `writer_active`, nothing renamed.
12. Legacy `seed_manifest_missing` (a folder with no manifest at all) → not recovered.

## Native E2E (Windows)

- A test-only, compile-time `#[cfg(feature = "e2e-webdriver")]` hook pauses seeding after fixture N when an env var is set.
- The journey:
  1. Launches with the hook and kills the process during the pause.
  2. Relaunches without the hook and opens the sample.
  3. Asserts the rebuilt notice, then that 2 subjects and 12 documents exist.
  4. Restarts once more and asserts there is no notice and the counts are still 12.

## Audit scripts

- `fixture-staging/` and the manifest need no new classification. The fixtures contain no audited marker strings, and `webview-disk-audit.mjs` already maps `fixture-staging`.
- `.discarding/` is deliberately **not** whitelisted. If it survives a normal sample run, the post-sample audit should stay inconclusive.

## Residual risk

On Windows, `sync_parent_dir` is a no-op. A power loss could lose the pending manifest's directory entry while Core's later writes survive. That gives "entries without a manifest": manual recovery, the safe direction.
