# Exact-range scope record — Spec 090 Institutional Adapters (object storage path)

Deterministic scope record per `FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`
(no external or LLM reviewer). Range: `origin/main` (`8df3e45`) to the
final head of the spec branch.

Files in scope: contracts `institutional.rs` (new), `envelopes/mod.rs` (2 capabilities, 2 requests, 2 responses), `lib.rs`; storage `institutional.rs` (new, v19), `sqlite_meta.rs`, `backup.rs` (`restore_v19`), `lib.rs`; earlier rewind tests (+`ia_*`); Core `institutional_transport.rs` (new: unavailable product transport, in-process qualification store), `authority/institutional.rs` (new), `authority/mod.rs`, `lib.rs`, `facade.rs`, `cli_session.rs`; CLI `institutional.rs` (new), `main.rs` (+8, CRLF kept); tests `institutional_090.rs` (Core, storage); promotion, spec package, evidence and the
BUILD_QUEUE row.

Checks on the range: no `Cargo.lock` change and no new dependency; no
`unsafe`; no network client; no process spawning except where the spec
says so; no existing assertion weakened (rewind tests only gain the new
tables); `crates/medscale-cli/src/main.rs` keeps CRLF line endings.
