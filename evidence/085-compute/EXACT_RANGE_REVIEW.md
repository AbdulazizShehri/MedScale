# Exact-Range Scope Record — Spec 085

A deterministic scope record under
`docs/planning/FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`. Not a semantic
review; no external reviewer ran or is required. The deterministic security
challenge is in `SECURITY.md`.

```text
BASE_SHA   = 434d1c71f9739d74e886b235a237cb741ddc51b5 (origin/main, Spec 084 closure PR #150)
RANGE_HEAD = 4018239624a11e1db9ac056d64b963bc93bd77c0 (checks run here)
CODE_HEAD  = 3bd00d1ca46a0a20d8724bd033aa61920dd9f6d2 (`git diff 3bd00d1 4018239 -- crates` is empty)
DIFF       = 41 files, +5920 / -11
PLATFORM   = Windows 11 workstation, git for Windows, 2026-09-26
```

| Check | Command (bash) | Result |
|---|---|---|
| Files outside authorized scope | `git diff --name-only $B $H` filtered to `crates/medscale-(contracts\|storage\|core\|cli)/`, `docs/planning/(BUILD_QUEUE\|SPEC_085_PROMOTION).md`, `evidence/085-compute/`, `specs/085-compute/` | none outside |
| Network crate | `git diff --name-only $B $H -- crates/medscale-network` | unchanged |
| Dependency / CI / toolchain manifests | `git diff --name-only $B $H -- '*Cargo.toml' Cargo.lock deny.toml supply-chain .github rust-toolchain.toml scripts` | only `crates/medscale-contracts/Cargo.toml` (two `[[bin]]` targets); no dependency added; `Cargo.lock` unchanged |
| Network APIs in 085 modules | `grep -nE "TcpStream\|TcpListener\|UdpSocket\|ureq\|reqwest\|hyper::\|medscale_network\|std::net"` over the 085 source files | none |
| Process launch | `grep -n "Command::new"` over the 085 source files | one: `compute_supervisor.rs` (the resolved worker only, cleared environment) |
| Worker linkage | `crates/medscale-contracts/src/bin/compute_worker.rs` imports | `medscale_contracts` and `std` only |
| CLI reaching storage, SQLite or process APIs | `grep -nE "medscale_storage\|rusqlite\|std::process"` over `medscale-cli/src/compute.rs` | none outside tests (the CLI test uses `std::process::id` for a temp directory name) |
| `unwrap`/`expect`/`panic!`/`unreachable!` and print calls outside tests (contracts, storage, Core 085 modules, worker) | awk scan before `#[cfg(test)]` | none |
| Added `unsafe` | `git diff $B $H -- 'crates/*.rs' \| grep '^+' \| grep -cE 'unsafe \{\|unsafe fn'` | 0 |
| Whitespace and line endings | `git -c core.whitespace=cr-at-eol diff --check $B $H`; `git diff --numstat $B $H` (no whole-file rewrites) | clean |
| Format / dependency direction | `cargo fmt --all -- --check`; `scripts/check-dependency-direction.ps1` | clean; passed |
| History | forward merges of main only; no rebase, no force-push | verified |

## Defects found and fixed on this branch

| Head | Found by | Defect | Fix |
|---|---|---|---|
| 6b88ad8 | CI (Clippy) | collapsible `if` in manifest validation | `0bbfaed` |
| 0bbfaed | exact-range review | manual loop counter in the consistency check (would fail Clippy) | `7adf90f` |
| 7adf90f | exact-range review | a scripted edit rewrote `medscale-cli/src/main.rs` from CRLF to LF (1,623-line diff) | `e26a773` |
| 7adf90f | CI (Clippy) | assertions on constants in schema tests | `7560ad5` |
| 7560ad5 | CI (compile, all OSes) | Core test helper borrowed the lab mutably while building its request | `9f173fe` |
| 9f173fe | CI (contract test) | `{"kind":"column_profile","code":"x"}` parsed: serde ignores unknown fields on unit variants of internally tagged enums, so the closed-params claim did not hold for that kind | `ColumnProfile {}` is an empty struct variant; unknown fields are refused (`3bd00d1`) |
