# Exact-range scope record — Spec 091 Federation (bounded bundle exchange)

Deterministic scope record per `FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`
(no external or LLM reviewer). Range: `origin/main` (`ee7e1d5`) to the
final head of the spec branch.

Files in scope: contracts `federation.rs` (new), `envelopes/mod.rs` (2 capabilities, 2 requests, 2 responses), `lib.rs`; storage `federation.rs` (new, v20; identity secret column never exported), `sqlite_meta.rs`, `backup.rs` (`restore_v20`), `lib.rs`; earlier rewind tests (+`fed_*`); Core `authority/federation.rs` (new), `authority/mod.rs`, `facade.rs`, `cli_session.rs`; CLI `federation.rs` (new), `main.rs` (+8, CRLF kept); tests `federation_091.rs` (Core, storage); promotion, spec package, evidence and the
BUILD_QUEUE row.

Checks on the range: no `Cargo.lock` change and no new dependency; no
`unsafe`; no network client; no process spawning except where the spec
says so; no existing assertion weakened (rewind tests only gain the new
tables); `crates/medscale-cli/src/main.rs` keeps CRLF line endings.
