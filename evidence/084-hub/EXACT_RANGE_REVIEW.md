# Exact-Range Scope Record — Spec 084

A deterministic scope record under
`docs/planning/FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`. Not a semantic
review; no external reviewer ran or is required. The deterministic security
challenge is in `SECURITY.md`.

```text
BASE_SHA   = 426bb3448d4e6e4b9c641fa256d6d2af530c2820 (origin/main, Spec 083 closure PR #148)
RANGE_HEAD = 38c47e33710d026f74bada5cce53350422194423 (checks re-run here)
CODE_HEAD  = 785810645e965b3d9db9191ff6df0ad440a6ffd7 (`git diff 7858106 38c47e3 -- crates` is empty)
DIFF       = 41 files, +7575 / -14 (first run on base 2892860 / head 7858106:
             38 files, +7470 / -12; the difference is 084 documentation)
PLATFORM   = Windows 11 workstation, git for Windows, 2026-09-24
```

| Check | Command (bash) | Result |
|---|---|---|
| Files outside authorized scope | `git diff --name-only $B $H` filtered to `crates/medscale-(contracts\|storage\|core\|cli\|keys)/`, `docs/planning/(BUILD_QUEUE\|SPEC_084_PROMOTION).md`, `evidence/084-hub/`, `specs/084-hub/` | none outside |
| Network crate | `git diff --name-only $B $H -- crates/medscale-network` | unchanged |
| Dependency / CI / toolchain manifests | `git diff --name-only $B $H -- '*Cargo.toml' Cargo.lock deny.toml supply-chain .github rust-toolchain.toml scripts` | none changed; no new dependency |
| Network or process APIs in 084 modules | `grep -nE "TcpStream\|TcpListener\|UdpSocket\|ureq\|reqwest\|hyper::\|medscale_network\|std::process::Command\|std::net"` over the six 084 source files | none |
| CLI reaching storage, SQLite or keys | `grep -nE "medscale_storage\|rusqlite\|medscale_keys"` over `medscale-cli/src/hub.rs` | none |
| `unwrap`/`expect`/`panic!`/`unreachable!` and print calls outside tests (contracts, keys, storage and Core 084 modules) | awk scan before `#[cfg(test)]` | none |
| Added `unsafe` | `git diff $B $H -- 'crates/*.rs' \| grep '^+' \| grep -cE 'unsafe \{\|unsafe fn'` | 0 |
| Whitespace | `git -c core.whitespace=cr-at-eol diff --check $B $H` | clean (`medscale-cli/src/main.rs` is CRLF on main; added lines keep it) |
| Format / dependency direction | `cargo fmt --all -- --check`; `scripts/check-dependency-direction.ps1` | clean; passed |
| History | branch started from the Spec 083 branch while 083 closed; forward merges of main (`7858106`, then `426bb34`); no rebase, no force-push | verified |

The Spec 024 IPC module gains one additive method,
`HostIpcServer::serve_connection_limited`; existing serving is unchanged.

## Defects found and fixed on this branch

| Head | Found by | Defect | Fix |
|---|---|---|---|
| before first CI | design challenge | device key swap and planted invitation in a hand-edited backup | key bound into `device_enrolled`; open invitations restore revoked |
| 8f28157 | CI (compile) | `OutboxEntry` name clash with the actions contract | renamed `HubOutboxEntry` (`21a5c16`) |
| 21a5c16 | CI (Clippy) | large enum variant | boxed fields (`ae116fc`) |
| ae116fc | CI (Clippy) | needless `mut` | `d731747` |
| d731747 | CI (test compile) | test binding shadowed a helper | `4b1aef7` |
| 4b1aef7 | security challenge | sessionless lease-holder reads over the Hub's device-facing endpoint | capability-limited serving (`8852460`) |
