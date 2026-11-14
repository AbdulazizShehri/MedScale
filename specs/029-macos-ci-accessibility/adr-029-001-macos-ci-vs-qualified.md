# ADR-029-001: macOS CI present ≠ macos_qualified

## Status

Accepted (Spec 029 READY_BASE)

## Context

RELEASE_READY requires a qualified OS matrix. Spec 022 recorded macOS as missing (`qualified_os_matrix_macos`). Expanding CI to `macos-latest` proves workspace builds/tests on Apple runners but does not equal product platform qualification (packaging, notarization, UX/privacy shell claims).

## Decision

1. Add `macos-latest` to the rust CI matrix with `--locked` and defensive Homebrew openssl/perl for SQLCipher builds.
2. Set `ReleaseQualificationDoctorStatus.macos_ci_present = true`.
3. Keep `macos_qualified = false` and `release_ready = false`.
4. Replace missing class `qualified_os_matrix_macos` with `macos_platform_product_qualification`.
5. Add separate `accessibility` doctor READY_BASE for fixture/CLI honesty checks without WCAG claims.

## Consequences

- Operators see honest “CI green on macOS” without false RELEASE_READY / PLATFORM_QUALIFIED claims.
- Later product macOS qualification can flip `macos_qualified` only with evidence beyond CI.
- Accessibility READY_BASE can grow when final v0 UI arrives without rewriting OS matrix semantics.
