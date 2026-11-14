# Security — Spec 083 Knowledge + Research Canvas

| # | Threat | Control | Proof |
|---|---|---|---|
| K1 | Cross-Project disclosure | the index is built only from the Project's own sources; every span is re-resolved against the Project on read; canvas evidence must be a current span of the canvas's Project | Core `projects_and_scopes_never_leak` |
| K2 | Cross-scope disclosure | every row carries realm and scope; reads check them; sessions are scope-bound | Core `projects_and_scopes_never_leak`; storage consistency and restore tamper tests |
| K3 | Stale text served as current | freshness computed per request; default search excludes stale spans; superseded spans are labelled; tombstoned spans never carry text | Core `stale_and_archived_sources_are_reported_not_served` |
| K4 | A cache bypassing authorization | no retrieval cache exists; each request re-checks scope, Project and freshness (Q31) | code path; Core tests |
| K5 | The index as a second truth | the index is a projection; hit text is re-read from the pinned live revision; receipts and canvases store references, not text | Core `a_canvas_holds_live_references_and_inspects_its_evidence` |
| K6 | Forged evidence references | a cited span must resolve inside the pinned revision (digest match, locator present, character range in bounds) | Core canvas test (forged range refused) |
| K7 | Tampered index or receipts | the chunk list is re-digested on every read; restore re-checks every invariant | storage `index_versions_are_atomic_contiguous_and_digest_checked`, `restore_rejects_hand_edited_083_snapshots` |
| K8 | Query injection | the query is split into words; it never reaches SQL or a parser | Core `refused_retrievals_leave_receipts` (SQL- and markup-shaped query) |
| K9 | Resource exhaustion | query at most 512 characters, 32 terms and 50 hits; index at most 100,000 chunks of at most 1,000 characters; canvas at most 500 nodes, 2,000 edges and 64 ops per edit | contract and Core deny tests |
| K10 | Silent empty answers | `insufficient_evidence` outcome, recorded on the receipt | Core search and deny tests |

Results stay local; there is no egress path. Real PHI remains unauthorized,
and all fixtures are synthetic.
