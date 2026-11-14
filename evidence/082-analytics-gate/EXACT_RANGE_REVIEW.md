# Exact-Range Scope Record — Spec 082

A deterministic scope record under
`docs/planning/FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`. Not a semantic
review; no external reviewer ran or is required. The deterministic security
challenge is in `SECURITY.md`.

```text
BASE_SHA   = 44f71e606ac74b632d417f2aa86202e79f71f9ea (origin/main)
RANGE_HEAD = 11150b5b6c578ac15d08e332af1266ed52b5f424 (code head)
DIFF       = 38 files, +5487 / -18
PLATFORM   = Windows 11 workstation, git for Windows, 2026-09-24
```

| Check | Command (bash) | Result |
|---|---|---|
| Files outside authorized scope | `git diff --name-only $B $H` filtered to `crates/medscale-(contracts\|storage\|core\|cli\|desktop)/`, `docs/planning/(BUILD_QUEUE\|SPEC_082_PROMOTION).md`, `evidence/082-analytics-gate/`, `specs/082-analytics-gate/`, `docs/engineering/admissions/005-sqlcipher-rusqlite.md`, `Cargo.toml` | none outside |
| Network crate | `git diff --name-only $B $H -- crates/medscale-network` | unchanged |
| Dependency / CI / toolchain manifests | `git diff --name-only $B $H -- '*Cargo.toml' Cargo.lock deny.toml supply-chain .github rust-toolchain.toml scripts` | only `Cargo.toml`: rusqlite gains the dependency-free `limits` feature (admission 005 amended); `Cargo.lock` unchanged |
| Network or process APIs in 082 modules | `grep -nE "TcpStream\|UdpSocket\|ureq\|reqwest\|hyper::\|medscale_network\|std::process::Command\|std::net"` over the six 082 source files | none |
| CLI/Desktop reaching storage, SQLite or network | `grep -nE "medscale_storage\|medscale_network\|rusqlite"` over the CLI and Desktop 082 modules | none |
| Print calls outside tests (contracts, storage, engine, Core, Desktop 082 modules) | awk scan before `#[cfg(test)]` | none |
| Added `unsafe` | `git diff $B $H -- 'crates/*.rs' \| grep '^+' \| grep -cE 'unsafe \{\|unsafe fn'` | 0 |
| Whitespace | `git -c core.whitespace=cr-at-eol diff --check $B $H` | clean |
| History | forward merge `965315e` of main into the branch; no rebase, no force-push | verified |

## Defects found and fixed on this branch

| Head | Found by | Defect | Fix |
|---|---|---|---|
| f71281c | qualification challenge | Engine could build giant values before result checks | `624989b` (SQLite length/attach limits, defensive mode) |
| f71281c | qualification challenge | Restore panicked on non-ASCII backup hex | `624989b` |
| f71281c | qualification challenge | Cohort type mismatches silently matched nothing | `624989b`, `52ef6c4` (CLI value typing) |
| f71281c | qualification challenge | Statistics indexed a stored row without a bounds check | `624989b` |
| f71281c | coverage review | Replay `input_unavailable` / `diverged` untested | `417431a` |

The same restore-hex defect in the closed Spec 080/081 modules is fixed
separately in PR #144.
