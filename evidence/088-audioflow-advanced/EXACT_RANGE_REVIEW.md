# Exact-range scope record — Spec 088 AudioFlow Advanced (huddle foundation)

Deterministic scope record per `FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`
(no external or LLM reviewer). Range: `origin/main` (`a40bd1e`) to the
final head of the spec branch.

Files in scope: contracts `huddles.rs` (new), `envelopes/mod.rs` (2 capabilities, 3 requests, 3 responses), `lib.rs`; storage `huddles.rs` (new, v17), `sqlite_meta.rs`, `backup.rs` (`restore_v17`), `lib.rs`; earlier rewind tests (+`hud_*`); Core `authority/huddles.rs` (new, on the Spec 081 `Audio` authority; four `Audio` helpers made `pub(super)`), `authority/mod.rs`, `facade.rs`, `cli_session.rs`; CLI `huddles.rs` (new), `main.rs` (+8, CRLF kept); tests `huddles_088.rs` (Core, storage); promotion, spec package, evidence and the
BUILD_QUEUE row.

Checks on the range: no `Cargo.lock` change and no new dependency; no
`unsafe`; no network client; no process spawning except where the spec
says so; no existing assertion weakened (rewind tests only gain the new
tables); `crates/medscale-cli/src/main.rs` keeps CRLF line endings.
