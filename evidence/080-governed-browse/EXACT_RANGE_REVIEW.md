# Exact-Range Scope Record — Spec 080

A deterministic scope record under
`docs/planning/FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`. Not a semantic
review; no external reviewer ran or is required.

```text
BASE_SHA   = 561f97feabe6beb7f079e2af679a4119b50368ce (origin/main)
RANGE_HEAD = 5daec975ed76b78381d9eb6b976b077933ae90cd (code head; later
             commits touch only evidence/080 and specs/080)
DIFF       = 32 files, +6096 / -15
PLATFORM   = Windows 11 workstation, git for Windows, 2026-09-23
```

| Check | Command (bash) | Result |
|---|---|---|
| Files outside authorized scope | `git diff --name-only $B $H` filtered to `crates/medscale-(contracts\|storage\|core\|cli\|desktop\|network)/`, `docs/planning/(BUILD_QUEUE\|SPEC_080_PROMOTION).md`, `evidence/080-governed-browse/`, `specs/080-governed-browse/` | none outside |
| Dependency / CI / toolchain manifests | `git diff --name-only $B $H -- '*Cargo.toml' Cargo.lock deny.toml supply-chain .github rust-toolchain.toml scripts` | none changed |
| Network APIs outside `medscale-network` | `grep -rn -E "TcpStream\|UdpSocket\|ureq::\|reqwest\|hyper::" crates --include=*.rs` excluding `crates/medscale-network/` | only pre-existing Spec 030-052 sandbox probes and pre-existing 062/064 tests that assert Desktop has no HTTP client; nothing added by 080 |
| CLI/Desktop reaching storage or network | `grep -n -E "medscale_storage\|medscale_network"` over the CLI and Desktop 080 modules | none |
| Print calls outside tests (contracts, storage, Core, network, Desktop 080 modules) | awk scan before `#[cfg(test)]` | none |
| Added `unsafe` | `git diff $B $H -- 'crates/*.rs' \| grep '^+' \| grep -cE 'unsafe \{\|unsafe fn'` | 0 |
| Whitespace | `git -c core.whitespace=cr-at-eol diff --check $B $H` | clean |
| Line endings | `crates/medscale-cli/src/main.rs` and `crates/medscale-network/src/lib.rs` keep CRLF; their diffs are 7 and 6 added lines | verified |

## Defects found and fixed on this branch (all by CI, first compile of the code)

| Head | Run | Failure | Fix |
|---|---|---|---|
| 1386797 | 35822750596 | Clippy `double_must_use` on scripted-transport helpers | `1b8894a` |
| 1b8894a/05be8e3 | superseded | (pre-emptive) `manual_is_multiple_of` in storage; line-ending flip of CLI `main.rs` found by the scope check | `05be8e3`, `714fc49` |
| 714fc49 | 35823200018 | unused trait import | `543aa6e` |
| 543aa6e | 35825395386 | name clash `insert_receipt_on` with Spec 079 storage | `4b0ed08` |
| 4b0ed08 | 35827765002 | rustfmt after the rename | `224f707` |
| 224f707 | 35827988579 | Clippy constant assertion in two storage tests | `a79ee0f` |
| a79ee0f | 35828298068 | 079 crash-recovery test still pinned to schema 8 | `8691277` |
| 8691277 | 35828932513 | none (6/6 success); superseded by the qualification challenge | — |

## Defects found by the T080-06 qualification challenge (fixed in `5daec97`)

See `SECURITY_ADVERSARIAL.md` rows C1-C6: allowlist path-normalization
bypass, prefix segment confusion, 6to4/Teredo/local-use NAT64 gaps,
NXDOMAIN mislabelled as a private target, and restore accepting rows that a
normal write would reject. The scope checks above were re-run on the range
`561f97f..5daec97` with identical results.
