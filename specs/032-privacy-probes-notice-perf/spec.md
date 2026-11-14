# Feature Specification: Privacy Probes + NOTICE Inventory + Perf Binding (Q03/Q05 residuals)

**Feature Branch**: `spec/032-privacy-probes-notice-perf`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Specs 017/023/027/028 CLOSED; Trusted V1 residuals still MedScale-owned  
**Does not**: claim `PRIVATE_DATA_READY`, `RELEASE_READY`, `MULTI_CLIENT_RELEASE_READY`, clear `OS_KEYRING_SWAP_SNAPSHOT_PRIVATE_DATA`, clear `REPO_BRANCH_PROTECTION_REQUIRED_CHECKS`, choose public SPDX for MedScale crates, claim perf budgets met, or clear `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED`.

## User Stories

### US1 — Q03 privacy probes / scaffolding (P1)
Platform probes detect/report pagefile and hibernate-file existence where readable, expose vault temp/work leftover scan helpers, and reuse Spec 017 crash-sidecar detection. Doctor lists residual risk classes (`swap`, `hibernate`, `snapshot`, `pagefile`) as **open** with `probes_present=true`. Never flip `private_data_ready`.

### US2 — Q05 NOTICE / third-party attribution prep (P1)
A PowerShell script builds a NOTICE/third-party inventory from `cargo metadata` + `deny.toml` license allowlist into evidence and `docs/legal/NOTICE_INVENTORY.md`. Doctor: `notice_inventory_present=true`, `rights_license_decision=false`. Do not choose a public SPDX license for MedScale crates.

### US3 — Perf binding evidence (P1)
Strengthen `perf_harness_027` evidence binding: git SHA/tree, rustc version, OS, hostname/CPU note, Cargo.lock digest, fixture identity. Keep `budgets_claimed_met=false`. Commit updated `perf_harness_latest.json` and methodology note if needed.

### US4 — Documentation honesty (P1)
Align `START_HERE` / `BUILD_QUEUE` / roadmap: Specs 016–031 closed + Spec 032 CLOSED; deferred **033+**. Fix stale Spec 017/023 LIMITATIONS that still imply MemoryMock-only keyring after Spec 028 OsKeyStore. EXTERNAL_GATES remain OPEN for swap/snapshot and license/branch-protection as applicable.

## Requirements

- **FR-001**: Implement OS privacy probe helpers (existence/report only) for pagefile/hiberfil (or OS equivalents) and vault leftover/crash-sidecar scan.
- **FR-002**: Doctor vault privacy: `probes_present=true`; residual classes open; `private_data_ready=false`.
- **FR-003**: `scripts/generate-notice-inventory.ps1` + inventory artifacts; no SPDX product license choice.
- **FR-004**: Doctor release qualification: `notice_inventory_present=true`; `rights_license_decision=false`.
- **FR-005**: Perf harness emits binding fields; budgets not claimed.
- **FR-006**: Spec Kit + evidence; BUILD_QUEUE 032 CLOSED; deferred **033+**; EXTERNAL_GATES notes that probes/inventory do not clear swap/snapshot or license/protection gates.

## Out of scope

PRIVATE_DATA_READY=true; measured secure wipe of OS swap/hibernate/snapshots; public MedScale crate SPDX; RELEASE_READY; multi-client release; AppContainer FS PLATFORM_QUALIFIED; REAL_PHI; MESC mutation.

## Success Criteria

- Workspace `--locked` green (fmt, clippy `-D warnings`, test) with `CARGO_TARGET_DIR=D:\medscale-target`
- Doctor honesty flags as above; EXTERNAL_GATES unchanged in cleared status
- Evidence under `evidence/032-privacy-probes-notice-perf/`
