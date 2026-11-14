# Security Challenge — Spec 083 Knowledge + Research Canvas

Deterministic challenge of the retrieval and Canvas surfaces, run on this
branch under `FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`. No external
or LLM reviewer ran. Each row names the control and the test that proves it,
or states the residual honestly. Test names refer to:
- Core: `crates/medscale-core/tests/knowledge_083.rs` and the unit tests in
  `crates/medscale-core/src/authority/knowledge.rs`;
- storage: `crates/medscale-storage/tests/knowledge_083.rs`;
- contracts: unit tests in `crates/medscale-contracts/src/knowledge.rs`.

## Challenge matrix

| Challenge | Result | Control / proof |
|---|---|---|
| Cross-Project search | no disclosure | the index is built only from the Project's own sources; Core `projects_and_scopes_never_leak` (Project B finds nothing of Project A) |
| Cross-Project citation | refused | a cited span must resolve as current inside the canvas's own Project; Core `projects_and_scopes_never_leak` |
| Cross-scope reads (receipt, canvas, status, search) | refused | scope-bound sessions; every read checks realm and scope; Core `projects_and_scopes_never_leak` |
| Forged span: another source's lineage, a wrong digest, or a range past the text | tombstoned, never resolved | each source kind checks Project, lineage, scope and pinned digest; the locator must exist and the character range must be in bounds; Core canvas test (forged range refused) |
| Superseded source served as current | never | freshness computed per request; default search lists current spans only; Core `stale_and_archived_sources_are_reported_not_served` |
| Archived source text disclosed | never | archived sources are tombstoned; tombstoned hits and nodes carry no text; Core stale and canvas tests |
| Cache reuse across authorization contexts | not possible | no retrieval cache exists; the per-request span cache lives only for one request (Q31) |
| Index text treated as truth | no | hit text is re-read from the pinned live revision, never from the index; receipts store references only; canvases store references only (Core canvas test checks the stored JSON has no evidence text) |
| Tampered chunk, dropped chunk, removed version | refused | chunk list re-digested on every read; contiguous versions; storage `index_versions_are_atomic_contiguous_and_digest_checked`, `restore_rejects_hand_edited_083_snapshots` (17 cases) |
| Tampered receipt (query, digest), duplicate rows, scope moved | refused on restore | storage restore tamper cases |
| Backup family missing or of the wrong JSON type (string, object), chunk or hit replaced by a scalar | refused on restore, never empty state | a family that is not an array is refused for every schema; a v12 snapshot must carry all three knowledge families; storage restore tamper cases |
| Receipt hit naming another chunk, no chunk, a moved span or another Project's source | refused on restore | every hit must be a chunk of its index, span for span (`verify_knowledge_consistency`); storage restore tamper cases |
| Repeated query terms or a term repeated in a document | bounded | query terms are deduplicated; term frequency is capped at 3; Core unit `scores_match_hand_computed_values` |
| Giant sources | bounded | windows of at most 1,000 characters; an index build over 100,000 chunks is refused, not truncated |
| Canvas revision gaps, Project moves, stale writers | refused | storage `receipts_and_canvas_revisions_hold_their_invariants`; Core canvas test (stale expected revision) |
| Query injection (SQL, markup) | inert | the query is split into words and never reaches SQL or a parser; Core `refused_retrievals_leave_receipts` |
| Oversized query, too many terms, bad hit limit, no index, empty query | refused with receipts | Core `refused_retrievals_leave_receipts` |
| Unicode text and windowing | windowed by characters, not bytes | Core unit `tokens_and_windows_are_deterministic` (Arabic text) |
| Nondeterministic ranking | deterministic | integer scoring and sequence tie-break; Core unit `scores_match_hand_computed_values`; Core search test repeats a query and compares hits |
| Deletion cascade into sources | impossible | knowledge tables have no foreign keys into Spec 075-082 tables and no delete path; knowledge never writes a source |

## Honest residuals

- Web evidence that Spec 080 flagged as instruction-like is indexed and
  shown as inert text; the hit does not repeat the flag. It is never
  executed or followed.
- A corrupt Spec 082 derived table makes an index build or status request
  fail closed (`corrupt`) for the whole Project until it is repaired.
- Lexical only: no stemming, synonyms or semantic matching, so relevant
  evidence phrased differently is missed. "Insufficient evidence" means no
  lexical match among current spans, not absence of evidence.
- Retrieval and index-build cost at the 100,000-chunk bound is unmeasured.
- Row-level integrity in unencrypted vaults is structural, not
  cryptographic, as in Specs 078-082.
