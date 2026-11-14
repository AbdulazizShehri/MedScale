# Exact-Range Scope Record — Spec 083

A deterministic scope record under
`docs/planning/FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`. Not a semantic
review; no external reviewer ran or is required. The deterministic security
challenge is in `SECURITY.md`.

```text
BASE_SHA   = f08f90314243267eca9e2edae1d6666f3fb74a10 (origin/main, PR #144 merge)
RANGE_HEAD = 8a05ccfa5e5a2fde922e938a988999c2ee124286 (code head)
DIFF       = 36 files, +5889 / -19
PLATFORM   = Windows 11 workstation, git for Windows, 2026-09-24
```

| Check | Command (bash) | Result |
|---|---|---|
| Files outside authorized scope | `git diff --name-only $B $H` filtered to `crates/medscale-(contracts\|storage\|core\|cli\|desktop)/`, `docs/planning/(BUILD_QUEUE\|SPEC_083_PROMOTION).md`, `evidence/083-knowledge-canvas/`, `specs/083-knowledge-canvas/` | none outside |
| Network crate | `git diff --name-only $B $H -- crates/medscale-network` | unchanged |
| Dependency / CI / toolchain manifests | `git diff --name-only $B $H -- '*Cargo.toml' Cargo.lock deny.toml supply-chain .github rust-toolchain.toml scripts` | none changed; no new dependency |
| Network or process APIs in 083 modules | `grep -nE "TcpStream\|TcpListener\|UdpSocket\|ureq\|reqwest\|hyper::\|medscale_network\|std::process::Command\|std::net"` over the five 083 source files | none |
| CLI/Desktop reaching storage, SQLite or network | `grep -nE "medscale_storage\|medscale_network\|rusqlite"` over the CLI and Desktop 083 modules | none |
| Print calls outside tests (contracts, storage, Core, Desktop 083 modules) | awk scan before `#[cfg(test)]` | none |
| `unwrap`/`expect`/`panic!`/`unreachable!` outside tests (storage and Core knowledge modules, `backup.rs`) | awk scan before `#[cfg(test)]` | none |
| Added `unsafe` | `git diff $B $H -- 'crates/*.rs' \| grep '^+' \| grep -cE 'unsafe \{\|unsafe fn'` | 0 |
| Whitespace | `git -c core.whitespace=cr-at-eol diff --check $B $H` | clean (`medscale-cli/src/main.rs` is CRLF on main; added lines keep it) |
| Format / dependency direction | `cargo fmt --all -- --check`; `scripts/check-dependency-direction.ps1` | clean; passed |
| History | forward merges `9625f04` (main `4b9351a`) and `8a05ccf` (main `f08f903`); no rebase, no force-push | verified |

## Defects found and fixed on this branch

| Head | Found by | Defect | Fix |
|---|---|---|---|
| 9f04a4d | qualification challenge (PR #144 lesson) | A restore family present but not an array, or a v12 knowledge family missing, restored as empty state | `334cf6b` |
| 9f04a4d | qualification challenge | A restored receipt could list hits that are not chunks of its index (moved span, missing chunk, foreign source) | `82869e4`, tests `64d0679` |
