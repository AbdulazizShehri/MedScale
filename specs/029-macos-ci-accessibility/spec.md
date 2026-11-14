# Feature Specification: macOS CI Matrix + CLI/Fixture Accessibility Honesty

**Feature Branch**: `spec/029-macos-ci-accessibility`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 022/027 release-qualification prep; Spec 006 CLI + FixtureUiViewModel; Spec 021 disclosure clarity  
**Does not**: claim `RELEASE_READY`, `PRIVATE_DATA_READY`, `MULTI_CLIENT_RELEASE_READY`, WCAG conformance, final v0 UI completeness, or `macos_qualified` / PLATFORM_QUALIFIED product status.

## User Stories

### US1 — macOS in CI matrix (P1)
CI `rust` job matrix includes `macos-latest` alongside `ubuntu-latest` and `windows-latest`, still using `--locked`. Install Homebrew OpenSSL/Perl when required for SQLCipher vendored OpenSSL builds on macOS.

### US2 — Accessibility READY_BASE honesty (P1)
Doctor axis `accessibility` reports that fixture/CLI keyboard-path and disclosure-clarity checks exist. Document limitations: no full WCAG audit; no final v0 UI. Never claim conformance.

### US3 — OS matrix doctor honesty (P1)
`release_qualification.macos_ci_present=true` when the workflow matrix includes macOS. `macos_qualified` remains **false** (baseline CI only, not product PLATFORM_QUALIFIED). `release_ready` remains **false**. Missing evidence classes stay honest about remaining gaps.

### US4 — FixtureUi / CLI label surface test (P1)
A small test proves FixtureUiViewModel and CLI help expose required labels/codes without claiming WCAG or RELEASE_READY.

## Requirements

- **FR-001**: Add `macos-latest` to `.github/workflows/ci.yml` rust matrix; keep `--locked` on clippy/test.
- **FR-002**: Handle macOS SQLCipher/OpenSSL/Perl build deps (Homebrew openssl/perl as needed).
- **FR-003**: Add `AccessibilityDoctorStatus` to doctor report with READY_BASE honesty fields and explicit non-claims (`wcag_conformance_claimed=false`, `final_v0_ui_present=false`, `release_ready=false`).
- **FR-004**: Extend `ReleaseQualificationDoctorStatus` with `macos_ci_present`; keep `macos_qualified=false` and `release_ready=false`; refresh missing evidence classes.
- **FR-005**: Test FixtureUiViewModel / CLI help required labels/codes; document limitations in evidence.
- **FR-006**: Spec Kit package + evidence; BUILD_QUEUE → 029 CLOSED; deferred advanced work **030+**.

## Out of scope

WCAG 2.x audit; final v0 visual UI; product macOS PLATFORM_QUALIFIED packaging/notarization; PRIVATE_DATA_READY; MULTI_CLIENT_RELEASE_READY; RELEASE_READY=true; configuring GitHub branch protection.

## Success Criteria

- Workspace fmt / clippy `-D warnings` / `cargo test --workspace --locked` PASS on Windows (local target dir)
- CI matrix includes macOS (proven when workflow runs)
- Doctor: `accessibility.ready_base=true`, no WCAG/RELEASE claims; `macos_ci_present=true`, `macos_qualified=false`, `release_ready=false`
- Queue/roadmap: Spec 029 CLOSED_CANONICAL READY_BASE; deferred **030+**
