# Exact-range scope record — Spec 086 R Workspace

Deterministic scope record per `FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`
(no external or LLM reviewer). Range: `origin/main` (`61855e4`) to the
final head of `spec/086-r-workspace`; 40 files, about 5.5k insertions, 10
deletions (`git diff --stat origin/main...HEAD`).

| Area | Files | In scope? |
|---|---|---|
| Contracts | `medscale-contracts/src/r_workspace.rs` (new), `envelopes/mod.rs` (4 capabilities, 9 requests, 8 responses), `lib.rs` | Yes: the promotion's frozen contracts |
| Harness | `medscale-contracts/src/bin/r_fake_ide.rs`, `Cargo.toml` `[[bin]]` | Yes: test-only stand-in for an IDE; std only |
| Storage | `medscale-storage/src/r_workspace.rs` (new), `sqlite_meta.rs` (migration 15, backup families), `backup.rs` (`restore_v15`), `lib.rs` (v15, exports) | Yes: additive v15 |
| Earlier storage tests | 078-085 rewind tests: add `rws_*` tables to drop lists and key filters; 085 test no longer pins v14 | Yes: mechanical, assertions unchanged |
| Core | `r_workspace_host.rs` (new), `authority/r_workspace.rs` (new), `authority/data_sources.rs` (+`vault_dir`), `facade.rs` (host field, setter, 9 arms, capability map), `cli_session.rs` (10 methods), `lib.rs`, `authority/mod.rs` | Yes |
| CLI | `medscale-cli/src/r_workspace.rs` (new), `main.rs` (+8 lines; CRLF preserved, `git diff --numstat` 8/0) | Yes |
| Docs and evidence | promotion, spec package, evidence, BUILD_QUEUE row | Yes |

Checks performed on the range:

- No new dependency: `Cargo.lock` unchanged; no `Cargo.toml`
  `[dependencies]` change (only a `[[bin]]` entry).
- No `unsafe`; no shell invocation; `Command` is used only in
  `r_workspace_host::launch` (configured absolute program, one argument,
  `env_clear`, stdio null) and by nothing that runs R.
- No network code added.
- No existing test assertion weakened; the only removed line pins the
  schema version constant to 14 in the Spec 085 test, following the Spec
  084 precedent.
- No write path to any Spec 074-085 object other than the audit trail.
- Line endings: `crates/medscale-cli/src/main.rs` remains CRLF.

Defects found by CI during qualification and fixed forward: Clippy
`field_reassign_with_default` in the Core test host builder (`dc2fd47`).
