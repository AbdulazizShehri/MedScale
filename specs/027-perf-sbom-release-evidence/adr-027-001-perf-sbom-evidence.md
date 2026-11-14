# ADR-027-001 — Perf harness + SBOM/checksum evidence scaffolds

## Status
Accepted

## Context
Trusted V1 Q05 release-evidence remnants beyond Spec 022 prep include performance budgets (not claimed achieved), reproducible package contents, and SBOM. Spec 022 left package/SBOM/install out of prep scope. Review Performance score was 0.

## Decision
1. Ship a deterministic `perf_harness_027` integration test (warmup + 30 runs) writing JSON evidence; CI asserts presence of reported timings only — never budget pass.
2. Ship PowerShell scripts that generate a CycloneDX-like SBOM scaffold from `cargo metadata` and a sha256 checksum manifest of workspace sources + lockfile.
3. Extend release-qualification doctor honesty with `perf_harness_present` and `sbom_scaffold_present` while keeping `release_ready=false`.

## Consequences
- Operators can regenerate evidence locally; committed samples document expected shape.
- Full release SBOM (native/model assets), signed packages, and budget attainment remain FALSE / missing classes.
- Deferred advanced work renumbers to **028+**.
