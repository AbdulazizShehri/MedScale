# Q05 / Spec 022 release evidence

## Immutable CI action pins (prior partial)

Pinned GitHub Actions to full commit SHAs with workflow `permissions: contents: read` (PR #39).

Does **not** configure branch protection / required checks (repository-owner external gate: `REPO_BRANCH_PROTECTION_REQUIRED_CHECKS`).

Does **not** claim `RELEASE_READY`.

## Locked builds (Spec 022)

- Root `Cargo.lock` is committed and required for CI.
- CI `cargo clippy` / `cargo test` use `--locked`.
- Documented verify paths: README + `specs/022-release-qualification-prep/quickstart.md`.

## Exact main SHA / TREE binding procedure

On the commit under qualification:

```text
git rev-parse HEAD          # commit SHA
git rev-parse HEAD^{tree}   # tree SHA
git status --porcelain      # must be empty for release-candidate binding
```

Record both SHAs in the owning evidence SUMMARY for that qualification attempt. Spec 022 prep does not publish a RELEASE_READY binding.

## Toolchain / lock identity

| Artifact | Identity |
|---|---|
| Rust toolchain | `rust-toolchain.toml` channel `1.97.1` (+ rustfmt, clippy) |
| CI toolchain | `dtolnay/rust-toolchain` with `toolchain: "1.97.1"` |
| Dependency lock | committed `Cargo.lock` (hash via `Get-FileHash Cargo.lock -Algorithm SHA256` / `sha256sum`) |

## OS matrix honesty

| Surface | Status |
|---|---|
| Windows CI (`windows-latest`) | Baseline CI only |
| Linux CI (`ubuntu-latest`) | Baseline CI only |
| macOS | **Unqualified** |
| iOS / Android | Scaffold / doctor READY_BASE only (Spec 009); not release-qualified apps |

Host-independent Rust tests do **not** imply Desktop/mobile app readiness.

## Branch protection pointer

See `docs/planning/EXTERNAL_GATES.md` row `REPO_BRANCH_PROTECTION_REQUIRED_CHECKS`. Cursor must not change repository settings via API.

## Spec 022 package evidence

See `evidence/022-release-qualification-prep/` for checklist vs RELEASE_READY=FALSE and LIMITATIONS.
