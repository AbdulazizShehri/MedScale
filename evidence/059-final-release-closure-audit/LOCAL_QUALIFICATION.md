# Spec 059 Local Qualification

Branch: `spec/059-final-release-closure-audit`
Base main: `0822f2e4dbc86ba43c0e1e43b9def07954750ca6`

Local available gates on the candidate working tree:

- `cargo fmt --all -- --check` — PASS.
- `git diff --check` (with repository CRLF convention respected) — PASS.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — PASS.
- `cargo test --workspace --locked` — PASS (`RC=0`).
- `cargo test -p medscale-core --locked --test final_release_closure_059` — PASS (`2/2`).
- `cargo test -p medscale-core --locked --test portable_release_package_058` — PASS (`2/2`).
- `cargo test -p medscale-core --locked --test release_qualification_022` — PASS (`1/1`).

GitHub PR exact-head run `34889698756` passed all six required checks on `8845b347ff225e598fedd7ca014928b16b0367a0`. PR #100 subsequently merged; `RELEASE_READY=false` remained unchanged.

Post-merge note: main run `34892900032` exposed a Windows-only `tasklist` RSS parser defect after PR #100 merge. Local corrective coverage adds parsing tests for empty/non-matching output; Windows CI remains authoritative for the fix.

Corrective Windows RSS-parser local gates on `fix/windows-runtime-rss-parser`:

- `cargo test -p medscale-desktop --locked --test runtime_perf_057 -- --nocapture` — PASS (`2/2`).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — PASS (`RC=0`).
- `cargo test --workspace --locked` — PASS (`RC=0`).
- `cargo fmt --all -- --check` — PASS.
- `git diff --check` — PASS.

The fix retries transient/non-parseable `tasklist` samples with a strict bound and still fails closed if no valid PID-bound RSS sample can be obtained. PR #101 exact-head run `34894123749` passed all six required jobs on `2b93dfdac4c7f1e2551a5f95603755e1cf499499`; PR #101 merged as `449e4ba00b21eeabb526b699e90d78954bcd01f8`; post-merge main run `34895017496` passed all six required jobs. Terminal repository-owned qualification is therefore complete.
