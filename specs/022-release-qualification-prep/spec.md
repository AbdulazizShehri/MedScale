# Feature Specification: Release Qualification Prep (Trusted V1 Q05 remnants)

**Feature Branch**: `spec/022-release-qualification-prep`  
**Created**: 2026-09-09  
**Status**: CLOSED_CANONICAL READY_BASE (prep)  
**Depends on**: Specs 016–021 CLOSED_CANONICAL; Q05 partial (immutable Action SHAs + `permissions: contents: read`)  
**Does not**: claim `RELEASE_READY`, change GitHub repository settings/branch protection, accept gated terms, mutate MESC, or authorize real PHI.

## User Stories

### US1 — Locked reproducible verify path (P1)
An operator (or CI) verifies the workspace with Cargo `--locked` against the committed `Cargo.lock`, with the toolchain pinned in `rust-toolchain.toml`.

### US2 — Exact SHA/TREE evidence binding (P1)
Evidence documents the procedure to bind `main` commit SHA and tree identity, toolchain channel, and lockfile identity without inventing a full release pipeline.

### US3 — OS matrix honesty (P1)
Evidence and doctor report state Windows+Linux CI baseline only; macOS unqualified; mobile scaffold-only — never infer app/release readiness from host Rust tests.

### US4 — Honest release readiness (P1)
Doctor axis `release_qualification` reports `RELEASE_READY=false` and lists missing evidence classes. Planning docs mark Spec 022 CLOSED_CANONICAL READY_BASE (prep); deferred advanced work is **023+**. Branch protection / required checks remain an EXTERNAL_GATES owner-settings item.

## Requirements

- **FR-001**: CI `cargo clippy` / `cargo test` use `--locked`; documented local verify paths match.
- **FR-002**: Evidence under `evidence/q05-release-evidence/` and/or `evidence/022-release-qualification-prep/` covering SHA/TREE binding, toolchain/lock identity, OS matrix, RELEASE_READY checklist vs FALSE, and pointer to EXTERNAL_GATES for repo settings.
- **FR-003**: Doctor axis (e.g. `release_qualification`) with `release_ready=false` and explicit missing evidence classes; CLI displays the axis.
- **FR-004**: `EXTERNAL_GATES.md` row for repository branch protection / required checks / settings authority (no API mutation).
- **FR-005**: Update `BUILD_QUEUE.md`, `SPECKIT_MASTER_ROADMAP_V2.md`, `START_HERE.md`: Spec 022 CLOSED_CANONICAL READY_BASE (prep); deferred advanced = **023+**; next status may be external-gate-only without claiming RELEASE_READY / PRIVATE_DATA_READY / MULTI_CLIENT_RELEASE_READY.

## Out of scope

Configuring GitHub branch protection or required checks; signing/notarization; full SBOM product packaging; license SPDX publication; macOS/mobile release qualification; RELEASE_READY=true; inventing a new release pipeline beyond existing CI/cargo-deny.

## Success Criteria

- Workspace fmt / clippy `-D warnings` / `cargo test --workspace --locked` PASS
- Doctor honesty: `release_ready=false`; missing classes listed
- Evidence LIMITATIONS honest; queue/roadmap renumber deferred to 023+
- No RELEASE_READY / PRIVATE_DATA_READY / MULTI_CLIENT_RELEASE_READY claim
