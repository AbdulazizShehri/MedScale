# Security challenge — Spec 088 AudioFlow Advanced

| # | Attack | Control | Evidence |
|---|---|---|---|
| S01 | Recording without everyone's consent | Attaching a human recording needs every human participant's join + record consent at that moment | core consent test (Ben without record consent blocks the attach) |
| S02 | Consent for one act reused for another | Record, transcribe and export are separate grants | core consent test (record does not allow transcribe; transcribe does not allow export) |
| S03 | Stale consent after someone leaves | Withdrawing join clears every other consent; acts re-check at execution | core consent test (export refused after Ben leaves) |
| S04 | An agent consenting for humans or impersonating one | Agents may only join; agent audio must name an agent participant; a human cannot be attributed as an agent | core agents test |
| S05 | Unlabeled synthetic audio | Synthetic origin and label must agree (contract, storage, restore) | contract + storage tests, backup tamper case |
| S06 | Transcript text becoming an assertion or action | Proposals cite segments and stay `proposed` until a human review records a decision; nothing executes | core proposals test |
| S07 | Deleted audio still readable | Deletion removes the Spec 081 source row (bytes), transcripts and their receipts in one transaction with `secure_delete`; media state `deleted` | core retention test; storage deletion test; consistency check refuses `deleted` media with a source |
| S08 | Partial deletion after a crash | One transaction per deletion; a second deletion is refused and writes nothing | storage deletion test |
| S09 | Tampered huddle rows or backups | Column-body checks; consistency after restore (media vs source, participants vs huddles) | storage tamper tests |
| S10 | Hidden cloud ASR | Transcription uses the Spec 081 route policy; the cloud route is refused there | Spec 081 tests (unchanged) |

Not controlled / recorded: exports and OS-level copies outside the vault
are not reached by deletion; earlier vault backups keep deleted media;
medical ASR quality is `UNMEASURED`.
