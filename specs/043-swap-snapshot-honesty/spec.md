# Feature Specification: Swap / Snapshot / Core-Dump Honesty (Q03 residual)

**Feature Branch**: `spec/043-swap-snapshot-honesty`  
**Created**: 2026-09-10  
**Status**: READY  
**Depends on**: Spec 032 privacy probes CLOSED  
**Promotion**: `EXISTING_Q03_RESIDUAL_ELIGIBLE_FOR_PROMOTION` — see `docs/planning/SPEC_043_PROMOTION.md`  
**Does not**: claim `PRIVATE_DATA_READY`; clear `OS_KEYRING_SWAP_SNAPSHOT_PRIVATE_DATA`; claim `PROTECTION_MEASURED` for swap/snapshot; expand into advanced 044+ product.

## Requirements

- **FR-001**: Extend OS privacy probes with distinct swap (where applicable), snapshot existence (best-effort), and core-dump/crash-dump configuration detection.
- **FR-002**: Each residual surface reports a `ProbeHonestyClass`: `configuration_detected` | `protection_measured` | `not_measurable_on_host` | `owner_or_os_policy_required`.
- **FR-003**: For swap/pagefile/hibernate/snapshot/core-dump in this unit: never set `protection_measured`; protection posture is `owner_or_os_policy_required` (or not measurable).
- **FR-004**: Doctor exposes Spec 043 fields; `private_data_ready=false`; residual risk classes remain open.
- **FR-005**: Evidence under `evidence/043-swap-snapshot-honesty/` with LIMITATIONS; CI/host runs exercise probes without claiming protection.
- **FR-006**: BUILD_QUEUE promotes 043 then closes after merge; advanced deferred **044+**.

## Clarifications

| Question | Resolution |
|---|---|
| Is 043+ forever forbidden? | No — only advanced product; this is existing Q03 residual. |
| Does Detected pagefile mean protection? | No — configuration/existence only. |
| Clear PRIVATE_DATA_READY? | No. |
