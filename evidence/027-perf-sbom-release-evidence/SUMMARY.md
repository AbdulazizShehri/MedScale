# Spec 027 SUMMARY — Perf harness + package/SBOM checksum evidence

**Status:** CLOSED_CANONICAL READY_BASE  
**Claims:** Does **not** claim `RELEASE_READY`, budget attainment, signed packages, or full release SBOM.

## Delivered

| Artifact | Path |
|---|---|
| Perf harness | `crates/medscale-core/tests/perf_harness_027.rs` |
| Harness JSON (written by test) | `evidence/027-perf-sbom-release-evidence/perf_harness_latest.json` |
| Methodology note | `evidence/027-perf-sbom-release-evidence/PERF_METHODOLOGY.md` |
| SBOM scaffold script | `scripts/generate-sbom-scaffold.ps1` |
| SBOM example | `evidence/027-perf-sbom-release-evidence/sbom-scaffold.json` |
| Checksum script | `scripts/generate-package-checksums.ps1` |
| Checksum manifest | `evidence/027-perf-sbom-release-evidence/package-checksums.json` |
| Doctor honesty | `perf_harness_present`, `sbom_scaffold_present`, `release_ready=false` |

## Regenerate

```powershell
$env:CARGO_TARGET_DIR = "D:\medscale-target"
cargo test -p medscale-core --test perf_harness_027 --locked
pwsh ./scripts/generate-sbom-scaffold.ps1
pwsh ./scripts/generate-package-checksums.ps1
```

## Honesty

- Delivery-plan p95 budgets are recorded as **targets**, not achieved claims.
- SBOM scaffold is cargo-metadata crates only (native/model assets missing).
- Checksums are unsigned source/lock digests, not a release package.
