# Spec 057 — Release Qualification Residual Integrity

## Scope

Spec 057 closes two repository-owned qualification defects discovered by the fresh post-056 executability audit:

1. Trusted V1 performance measurement coverage omitted cold model-free launch and idle memory.
2. CI claimed immutable GitHub Action pins while `actions/upload-artifact@v4` remained mutable.

## Implemented

- bounded `medscale-desktop --perf-idle-ms` engineering probe;
- cross-platform cold-launch + idle-RSS harness with source/tree/lock/toolchain binding;
- runtime perf artifacts uploaded from the Windows/Linux/macOS CI matrix;
- `actions/upload-artifact` pinned to commit `ea165f8d65b6e75b540449e92b4886f43607fa02` (`v4.6.2`);
- regression test requiring every workflow `uses:` reference to be a 40-hex SHA;
- doctor `runtime_perf_measurement_coverage_present=true`.

No release-performance attainment is claimed.
## Qualification

- Local: fmt PASS; clippy workspace/all-targets PASS; workspace tests PASS.
- Exact implementation head: `e1be7d126560453ef8e1c28954aa07b0dbc5d729`.
- GitHub Actions run `34822371220`: six required jobs SUCCESS.
- Windows/Linux/macOS runtime performance coverage artifacts uploaded successfully.
- `cargo-deny`: SUCCESS in exact-head CI.
- Release/performance attainment remains false.
