# Feature Specification: Host Perf Measurement Path (Q05 residual)

**Feature Branch**: `spec/050-host-perf-measurement`  
**Promotion**: `docs/planning/SPEC_050_PROMOTION.md`

## Requirements

- **FR-001**: Operator script runs Spec 027/042/045 perf harness with explicit env scale knobs.
- **FR-002**: Script writes host binding sidecar (OS, arch, CPU, RAM, rustc, source/tree/lock SHAs, scale knobs, UTC timestamp).
- **FR-003**: Doctor `host_perf_measurement_path_present=true`; `release_ready=false`; `budgets_claimed_met` remains false / missing class retained.
- **FR-004**: Evidence SUMMARY/LIMITATIONS under `evidence/050-host-perf-measurement/`.

## Out of scope

Claiming budget attainment, RELEASE_READY, cross-platform qualification from one host.
