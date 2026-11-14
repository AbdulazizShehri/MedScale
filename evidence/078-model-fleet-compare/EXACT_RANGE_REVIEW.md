# Exact-Range Scope Record — Spec 078

Under `docs/planning/FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`, this is a
**deterministic scope record**, not a semantic review. No external reviewer
(Alibaba Open Code Review or any substitute) ran or is required. Every result
below came from a command that can be run again.

## Binding

```text
BASE_SHA   = ff2e677294b1ea8704bbeefa605d129c1a99e8f2 (origin/main)
RANGE_HEAD = 9f00d410b6e918702864610682788896b7f1aef3 (last code+docs head before
             the review-policy amendment commit; that commit and later ones only
             touch docs/planning, specs/078 and evidence/078)
DIFF       = 43 files, +11410 / -21
PLATFORM   = Windows 11 workstation `Shehr`, git for Windows, 2026-09-22
```

## Checks

| Check | Command (bash, from repo root) | Result |
|---|---|---|
| Files outside authorized scope | `git diff --name-only $B $H \| grep -v -E '^(crates/medscale-(contracts\|storage\|core\|cli\|desktop)/\|docs/planning/(BUILD_QUEUE\|EXTERNAL_GATES\|SPEC_078_PROMOTION)\.md\|evidence/078-model-fleet-compare/\|specs/078-model-fleet-compare/)'` | none |
| Dependency / CI / toolchain manifests | `git diff --name-only $B $H -- '*Cargo.toml' Cargo.lock deny.toml supply-chain .github rust-toolchain.toml scripts` | none changed |
| Spec 077 source untouched | `git diff --name-only $B $H -- '*medagent.rs'` | none changed |
| Network / process / env / unsafe / logging in new 078 modules | `grep -n -E "std::net\|TcpStream\|UdpSocket\|reqwest\|ureq\|hyper::\|Command::new\|env::var\(\|unsafe \|println!\|eprintln!\|dbg!\|tracing::\|log::"` over the six new `model_fleet*.rs` modules | only `println!` in `crates/medscale-cli/src/model_fleet.rs` (the CLI's human output surface; ids, states, counts and `escape_debug` role labels); none in contracts, storage, Core or Desktop |
| Added `unsafe` anywhere in the diff | `git diff $B $H -- 'crates/*.rs' \| grep '^+' \| grep -E 'unsafe \{\|unsafe fn'` | none |
| Score / rank / winner fields in contracts | `grep -n -E "pub (score\|winner\|rank\|best\|quality)" crates/medscale-contracts/src/model_fleet.rs` | none. Also enforced by the unit test `model_fleet_compare::tests::computation_has_no_score_rank_or_winner_logic` |
| Desktop/CLI direct storage or runtime access | `grep -n -E "medscale_storage\|SqliteMetaStore\|onnx\|ort::"` over `model_fleet_workspace.rs` and CLI `model_fleet.rs` | none in production code. The ONNX pack path appears only under `#[cfg(test)]` (line 256+) in `model_fleet_workspace.rs`, and it is installed through the Core session (`packs_install_local`) |
| Whitespace | `git -c core.whitespace=cr-at-eol diff --check $B $H` | clean. Plain `git diff --check` flags only CR-at-EOL in `crates/medscale-cli/src/main.rs` and `docs/planning/EXTERNAL_GATES.md`, which are CRLF files at BASE (`git show $B:<path> \| file -`) |
| Dependency direction | `scripts/check-dependency-direction.ps1` (CI step in every `rust (*)` job) | pass (see `EXACT_HEAD_QUALIFICATION.md`) |

## Changed pre-078 files (all expected)

- `crates/medscale-storage/tests/{project_graph_074,data_sources_075,collaboration_076,medagent_077}.rs`:
  they now assert the live top schema version instead of a pinned one. This
  fixed a real regression that CI run `35765069726` caught (`d9c75a8`).
- `crates/medscale-storage/src/{backup.rs,sqlite_meta.rs,lib.rs}`: schema v7
  registration and 078 backup/restore rows.
- `crates/medscale-core/src/{authority/facade.rs,authority/mod.rs,cli_session.rs}`,
  `crates/medscale-contracts/src/{envelopes/mod.rs,lib.rs}`,
  `crates/medscale-cli/src/main.rs`, `crates/medscale-desktop/{src/main.rs,ui/app.slint}`:
  wiring for the new operations and route.

## Limitations

- This record checks scope and structure. It does not claim semantic
  correctness. Behavior is proven by the tests and CI named in
  `EXACT_HEAD_QUALIFICATION.md` and the per-area qualification files.
- The historical OCR delegate preview (`logs/ocr-delegate-preview-547fb41.txt`)
  listed files only. No OCR findings were produced or claimed for Spec 078.
