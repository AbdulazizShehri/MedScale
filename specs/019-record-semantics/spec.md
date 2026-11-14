# Feature Specification: Record Semantics (Trusted V1 Q06)

**Feature Branch**: `spec/019-record-semantics-q06`  
**Created**: 2026-09-09  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 016 durable path; Spec 004 presentation; Spec 002 MedicalTime  
**Does not**: claim RELEASE_READY or PRIVATE_DATA_READY; touch MESC; admit real PHI; redesign FHIR matrix (Spec 020).

## User Stories

### US1 — Precision-aware time (P1)
A synthetic record carries year-only and day-precision effective times. Timeline ordering treats partial times as ranges, never invents Instant, and refuses Year→Instant upgrades.

### US2 — Append-only amendments (P1)
A correction creates a superseding ClinicalAssertion plus AmendmentRecord lineage and audit. Prior assertion bytes remain readable; no in-place overwrite.

### US3 — Explicit identity reconciliation (P1)
Shared identifiers across distinct subjects surface as IdentityUnresolvedSet candidates. Merge remains an explicit DecideIdentityMerge decision.

### US4 — Missingness taxonomy (P1)
Unknown, explicit absence, not observed, not applicable, withheld, and conflict remain distinct MissingnessKind values through serde round-trip.

## Requirements

- **FR-001**: Extend `MedicalTime` with optional `timezone_offset_minutes`, `TimeRole`, parse/validate helpers, refuse false precision upgrades, precision-aware ordering bounds.
- **FR-002**: `MissingnessKind` enum as specified.
- **FR-003**: Append-only `AmendmentRecord` linking OpaqueIds; `AmendAssertion` facade op.
- **FR-004**: `IdentityReconciliationCandidate` + `IdentityUnresolvedSet` + library helper.
- **FR-005**: Timeline sorting must not treat unknown/partial as Instant; exclude superseded priors from active timeline.
- **FR-006**: Persist amendments via Spec 016 `StoredObject` / authority_objects path.
- **FR-007**: Synthetic-only tests; doctor axis honest (`release_ready=false`).

## Out of scope

FHIR support matrix (Spec 020); workflow journey (Q07); GraphRAG/plugins/imaging; real PHI; MESC; OS multi-client release.

## Success Criteria

- Precision/upgrade/amendment/missingness/identity tests PASS
- Workspace fmt/clippy/test PASS
- Evidence LIMITATIONS honest; BUILD_QUEUE promotes 019 CLOSED_CANONICAL; next = Spec 020 READY
