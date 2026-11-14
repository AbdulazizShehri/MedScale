# Exact-Range Scope Record — Spec 079

A deterministic scope record under
`docs/planning/FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`. Not a
semantic review; no external reviewer ran or is required.

```text
BASE_SHA   = cf8731e497273633efe2968ade4cbdd6523e5595 (origin/main)
RANGE_HEAD = 82d31547df44047c45736127a67396992630b115 (code head; later commits touch only
             evidence/079-privacy-gate and specs/079-privacy-gate)
DIFF       = 38 files, +10398 / -31
PLATFORM   = Windows 11 workstation, git for Windows, 2026-09-23
```

| Check | Command (bash) | Result |
|---|---|---|
| Files outside authorized scope | `git diff --name-only $B $H \| grep -v -E '^(crates/medscale-(contracts\|storage\|core\|cli\|desktop\|keys)/\|docs/planning/(BUILD_QUEUE\|SPEC_079_PROMOTION)\.md\|evidence/079-privacy-gate/\|specs/079-privacy-gate/)'` | none |
| Dependency / CI / toolchain manifests | `git diff --name-only $B $H -- '*Cargo.toml' Cargo.lock deny.toml supply-chain .github rust-toolchain.toml scripts` | none changed |
| Network / process / env / unsafe / logging in the six new 079 modules | `grep -n -E "std::net\|TcpStream\|UdpSocket\|reqwest\|ureq\|hyper::\|Command::new\|env::var\(\|unsafe \|tracing::\|log::"` | none |
| Print calls outside tests (contracts, storage, Core, Desktop 079 modules) | awk scan for `println!/eprintln!/dbg!` before `#[cfg(test)]` | none (the CLI module prints its human output by design) |
| Added `unsafe` in the diff | `git diff $B $H -- 'crates/*.rs' \| grep '^+' \| grep -cE 'unsafe \{\|unsafe fn'` | 0 |
| CLI/Desktop reaching storage or keys | `grep -n -E "medscale_storage\|medscale_keys\|SqliteMetaStore"` over the CLI and Desktop 079 modules | none |
| Whitespace | `git -c core.whitespace=cr-at-eol diff --check $B $H` | clean (`crates/medscale-cli/src/main.rs` keeps its CRLF endings) |

## Changed pre-079 files (all expected)

- Wiring: `contracts/src/{lib.rs,envelopes/mod.rs}`, `core/src/{authority/mod.rs,authority/facade.rs,cli_session.rs}`, `cli/src/main.rs`, `desktop/src/main.rs`, `desktop/ui/app.slint`.
- Storage v8: `storage/src/{lib.rs,sqlite_meta.rs,backup.rs}`.
- `medscale-keys`: exports `generate_key32` and `zeroize_key` (no new dependency).
- Earlier specs' storage tests (074-078): assert `CURRENT_META_SCHEMA_VERSION` instead of a pinned `7`; the 078 rewind helper also drops v8 state; a hidden `let top = 7;` pin in the 074 test is replaced.
