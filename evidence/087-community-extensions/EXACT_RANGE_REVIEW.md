# Exact-range scope record — Spec 087 Community Extensions

Deterministic scope record per `FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`
(no external or LLM reviewer). Range: `origin/main` at promotion to the
final head of `spec/087-community-extensions`.

| Area | Files | In scope? |
|---|---|---|
| Contracts | `medscale-contracts/src/extensions.rs` (new), `envelopes/mod.rs` (3 capabilities, 10 requests, 5 responses), `lib.rs` | Yes |
| Storage | `medscale-storage/src/extensions.rs` (new), `sqlite_meta.rs` (migration 16, backup families), `backup.rs` (`restore_v16`), `lib.rs` (v16, exports) | Yes: additive v16 |
| Earlier storage tests | 078-086 rewind tests: add `ext_*` tables to drop lists and key filters | Yes: mechanical |
| Core | `authority/extensions.rs` (new: verification, lifecycle, grants, invocation, SDK helpers), `authority/mod.rs` (exports), `facade.rs` (10 arms, capability map), `cli_session.rs` (12 methods) | Yes |
| CLI | `medscale-cli/src/extensions.rs` (new), `main.rs` (+8 lines; CRLF preserved) | Yes |
| Tests | `medscale-core/tests/extensions_087.rs`, `medscale-storage/tests/extensions_087.rs`, contract and CLI unit tests | Yes |
| Docs and evidence | promotion, spec package, evidence, BUILD_QUEUE row | Yes |

Checks performed on the range:

- No new dependency: `Cargo.lock` unchanged; ed25519 comes from the
  already-admitted `medscale-keys` (`ed25519-dalek`, strict verification).
- No `unsafe`; no process spawning; no dynamic loading; no network code;
  no file access in Core (packs arrive as bytes; only the CLI host reads
  files).
- No existing test assertion weakened.
- Host API v1 is three read-only operations executed by Core; there is no
  write path from an extension.
