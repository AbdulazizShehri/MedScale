# Exact-range scope record — Spec 089 Research Packs (Clinical Research first)

Deterministic scope record per `FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`
(no external or LLM reviewer). Range: `origin/main` (`1ac0362`) to the
final head of the spec branch.

Files in scope: contracts `research_packs.rs` (new, incl. the built-in Pack), `envelopes/mod.rs` (2 capabilities, 5 requests, 5 responses), `lib.rs`; storage `research_packs.rs` (new, v18), `sqlite_meta.rs`, `backup.rs` (`restore_v18`), `lib.rs`; earlier rewind tests (+`rp_*`); Core `authority/research_packs.rs` (new), `authority/mod.rs`, `facade.rs`, `cli_session.rs`; CLI `research_packs.rs` (new), `main.rs` (+8, CRLF kept); tests `research_packs_089.rs` (Core, storage); promotion, spec package, evidence and the
BUILD_QUEUE row.

Checks on the range: no `Cargo.lock` change and no new dependency; no
`unsafe`; no network client; no process spawning except where the spec
says so; no existing assertion weakened (rewind tests only gain the new
tables); `crates/medscale-cli/src/main.rs` keeps CRLF line endings.
