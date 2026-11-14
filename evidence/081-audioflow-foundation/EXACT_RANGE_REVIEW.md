# Exact-Range Scope Record — Spec 081

A deterministic scope record under
`docs/planning/FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`. Not a semantic
review; no external reviewer ran or is required.

```text
BASE_SHA   = c682178ed4b8a4bbee67fe64d89cc94450d60b13 (origin/main)
RANGE_HEAD = 502b49aaca3acc977ebf124a3f329b8cee7371d6 (code head; later
             commits touch only evidence/081 and specs/081)
DIFF       = 31 files, +6977 / -10
PLATFORM   = Windows 11 workstation, git for Windows, 2026-09-23
```

| Check | Command (bash) | Result |
|---|---|---|
| Files outside authorized scope | `git diff --name-only $B $H` filtered to `crates/medscale-(contracts\|storage\|core\|cli\|desktop)/`, `docs/planning/(BUILD_QUEUE\|SPEC_081_PROMOTION).md`, `evidence/081-audioflow-foundation/`, `specs/081-audioflow-foundation/` | none outside |
| Network crate | `git diff --name-only $B $H -- crates/medscale-network` | unchanged |
| Dependency / CI / toolchain manifests | `git diff --name-only $B $H -- '*Cargo.toml' Cargo.lock deny.toml supply-chain .github rust-toolchain.toml scripts` | none changed |
| Network or process APIs in 081 modules | `grep -nE "TcpStream\|UdpSocket\|ureq\|reqwest\|hyper::\|medscale_network\|std::process::Command\|std::net"` over the five 081 source files | none |
| CLI/Desktop reaching storage or network | `grep -nE "medscale_storage\|medscale_network"` over the CLI and Desktop 081 modules | none |
| Print calls outside tests (contracts, storage, Core, Desktop 081 modules) | awk scan before `#[cfg(test)]` | none |
| `COMMAND` execution paths | `grep -rn "VoiceInputMode::Command"` outside tests | none |
| Added `unsafe` | `git diff $B $H -- 'crates/*.rs' \| grep '^+' \| grep -cE 'unsafe \{\|unsafe fn'` | 0 |
| Whitespace | `git -c core.whitespace=cr-at-eol diff --check $B $H` | clean |
| Line endings | `crates/medscale-cli/src/main.rs` keeps CRLF (1600 CRLF lines) | verified |

## Defects found and fixed on this branch (all by CI)

| Head | Run | Failure | Fix |
|---|---|---|---|
| fafe3bc (first push) | 35885773446 | `DigestSha256` is not `Copy`: moved out of a borrowed revision | `710e59c` (7 sites) |
| 710e59c | 35887232414 | Clippy `manual_is_multiple_of` | `f1e6356` (3 sites; loop counter restructured pre-emptively) |
| f1e6356 | 35888572661 | Spec 066 hardening test: warning colour used as text colour in the new Audio route | `502b49a` |
