# Spec 058 — Portable Release Package Qualification

## Fresh audit basis

After Spec 057, the release doctor still listed `reproducible_release_package_contents` and `release_package_upgrade_rollback_proof`. Spec 049 explicitly documented that no real package artifacts existed, while the Trusted V1 delivery plan requires package installation/upgrade/rollback.

## Intended closure

Spec 058 provides real unsigned portable package artifacts, deterministic package-assembly proof, fail-closed verification, and install/upgrade/rollback proof on all three CI operating systems.

This does not close signing/provenance, qualified-hardware performance, macOS final-product, final-v0/WCAG, or material-findings gates.

## Exact-head qualification note

The first PR exact-head CI discovered `RUSTSEC-2026-0285` against transitive `rustls 0.23.43`. The branch updates the lockfile to `rustls 0.23.45`, the minimum fixed line identified by cargo-deny. No advisory ignore or policy weakening is used.

The same first exact-head run exposed Unix path duplication when an absolute `OutputDir` was passed between qualification and builder scripts. Spec 058 now resolves absolute paths without re-prefixing the repository root and regression-checks that behavior.

The second exact-head CI proved deterministic ZIP assembly on Linux and macOS, then exposed a PowerShell harness bug: the verifier used the automatic `$Args` variable name for smoke-test arguments, so `--help` was not forwarded. The verifier now uses `CommandArgs`; regression coverage prevents reintroduction.
