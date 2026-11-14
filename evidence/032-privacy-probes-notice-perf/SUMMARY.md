# Evidence SUMMARY — Spec 032 Privacy Probes + NOTICE + Perf Binding

**Status:** CLOSED_CANONICAL READY_BASE  
**Branch:** `spec/032-privacy-probes-notice-perf`  
**Base:** main `39538a2`

## Delivered

- OS privacy probes (`medscale-storage::privacy_probes`): pagefile/hiberfil (or OS equivalents) existence, vault leftover + crash-sidecar scan helpers
- Doctor vault privacy: `probes_present=true`; residual classes `swap`, `hibernate`, `snapshot`, `pagefile` open; `private_data_ready=false`
- `scripts/generate-notice-inventory.ps1` → `docs/legal/NOTICE_INVENTORY.md` + `evidence/032-privacy-probes-notice-perf/notice-inventory.json`
- Doctor release qualification: `notice_inventory_present=true`; `rights_license_decision=false`
- Perf harness binding (schema_version 2): git SHA/tree, rustc, OS/host/CPU, Cargo.lock digest, fixture identity; `budgets_claimed_met=false`
- Stale Spec 023 / admission MemoryMock-only wording corrected for Spec 028 OsKeyStore

## Honesty

- PRIVATE_DATA_READY = FALSE
- RELEASE_READY = FALSE
- MULTI_CLIENT_RELEASE_READY = FALSE
- rights_license_decision = FALSE (no MedScale public SPDX choice)
- budgets_claimed_met = FALSE
- EXTERNAL_GATES `OS_KEYRING_SWAP_SNAPSHOT_PRIVATE_DATA` remains OPEN (probes do not clear)
- EXTERNAL_GATES `PUBLIC_SOURCE_LICENSE_CHOICE` remains PENDING (inventory does not clear)
- EXTERNAL_GATES `REPO_BRANCH_PROTECTION_REQUIRED_CHECKS` unchanged
