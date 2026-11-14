# Feature Specification: Minimum Lovable Trusted Workflow (Trusted V1 Q07)

**Feature Branch**: `spec/021-minimum-lovable-workflow`  
**Created**: 2026-09-09  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Specs 016–020 (durable record, privacy honesty, host sessions, semantics, FHIR interchange)  
**Does not**: claim RELEASE_READY, PRIVATE_DATA_READY, final v0 visual UI, real PHI, live SMART/NPHIES, MESC mutation, or multi-client OS IPC release.

## User Stories

### US1 — Restartable synthetic journey (P1)
An operator runs a documented CLI journey (or scripted sequence of facade ops) that covers: install/identity check → start offline → inspect privacy/capability (doctor) → load synthetic vault → import FHIR → preview → accept/reject → timeline/brief/coverage → source drill-down → close → reopen → verify same record → export → disclosure append → backup → verify backup → restore.

### US2 — Shared CoreFacade authority (P1)
The journey uses the same CoreFacade capabilities as other clients. CLI does not open canonical storage directly. Close/reopen proves durable identity across process boundaries (composing Spec 016 restart evidence).

### US3 — Disclosure clarity (P1)
Exporting a scoped synthetic Brief/bundle records a local append-only DisclosureRecord (purpose, scope, artifact refs, synthetic_only). Disclosure is not authority and never claims RELEASE_READY.

### US4 — Honest readiness (P1)
Doctor/notes report `WORKFLOW_READY_BASE=true` and `RELEASE_READY=false`. No product release, private-data, or full FHIR conformance claim is implied by completing the journey.

## Requirements

- **FR-001**: Typed workflow contracts: `DisclosureRecord`, import preview types, `WorkflowDoctorStatus`, journey step/result types, stable CLI JSON error envelope.
- **FR-002**: Facade capabilities for reject-proposal (non-promote), append/list disclosures; preview composed from ingest + proposal + loss-aware export.
- **FR-003**: Core journey runner composing existing Open/Ingest/Promote/Presentation/Export/Backup/Restore/Close ops.
- **FR-004**: CLI `journey run` (or equivalent) with stable exit codes and `--json` errors.
- **FR-005**: Integration test exercising the journey through CoreFacade including close/reopen (two-process pattern from Spec 016).
- **FR-006**: Doctor axis + notes: WORKFLOW_READY_BASE vs RELEASE_READY=false.
- **FR-007**: Evidence SUMMARY + LIMITATIONS; BUILD_QUEUE marks 021 CLOSED_CANONICAL READY_BASE; next unit honesty without RELEASE_READY.

## Out of scope

Final visual Desktop UI; accessibility certification; PRIVATE_DATA_READY; RELEASE_READY; live partner endpoints; MESC; plugins/GraphRAG; real PHI.

## Success Criteria

- Journey CLI + facade integration tests PASS (including reopen identity)
- Workspace fmt / clippy `-D warnings` / `cargo test --workspace` PASS
- Doctor honesty: workflow ready_base true; release_ready false
- Evidence LIMITATIONS honest; queue updated; no RELEASE_READY claim
