# Exact-range scope record — Spec 093 Research OS Desktop parity (read-only slice)

Deterministic scope record per `FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`
(no external or LLM reviewer). Range: `origin/main` (`41d2a9a`) to the
final head of the spec branch.

Files in scope: Desktop `research_os_workspace.rs` (new), `ui/app.slint`, `main.rs`; docs; promotion, spec package, evidence and the
BUILD_QUEUE row.

Checks on the range: no `Cargo.lock` change and no new dependency; no
`unsafe`; no network client; no process spawning except where the spec
says so; no existing assertion weakened (rewind tests only gain the new
tables); `crates/medscale-cli/src/main.rs` keeps CRLF line endings.
