# ADR-032-001 — Privacy probes + NOTICE prep without readiness claims

## Context

Trusted V1 Q03 still blocks `PRIVATE_DATA_READY` on swap/hibernate/snapshot (and related pagefile) residual risk after Spec 028 OsKeyStore. Q05 still needs third-party attribution prep and stronger perf evidence binding without claiming RELEASE_READY.

## Decision

Ship one Spec Kit unit that:

1. Adds best-effort OS privacy **probes** and leftover/crash scan helpers with doctor honesty (`probes_present=true`; residual classes open).
2. Adds a NOTICE inventory generator from cargo metadata + deny allowlist (`notice_inventory_present=true`; `rights_license_decision=false`).
3. Strengthens Spec 027 perf harness binding fields while keeping `budgets_claimed_met=false`.

## Consequences

- Operators can see residual OS privacy surfaces without false PRIVATE_DATA_READY.
- Attribution inventory exists for counsel/license gate without choosing SPDX for MedScale.
- Perf evidence is reproducible-bound to tree/toolchain without budget attainment claims.
- EXTERNAL_GATES `OS_KEYRING_SWAP_SNAPSHOT_PRIVATE_DATA`, `PUBLIC_SOURCE_LICENSE_CHOICE`, and `REPO_BRANCH_PROTECTION_REQUIRED_CHECKS` remain uncleared by this unit.
