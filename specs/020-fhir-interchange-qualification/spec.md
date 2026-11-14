# Feature Specification: FHIR / Interchange Qualification (Trusted V1 Q08)

**Feature Branch**: `spec/020-fhir-support-matrix`  
**Created**: 2026-09-09  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 019 record semantics; Spec 003/004 FHIR ingest/presentation; Spec 013 validator evidence path  
**Does not**: claim RELEASE_READY, full FHIR conformance, SMART server capability, live partner profiles, real PHI, or MESC mutation.

## User Stories

### US1 — Honest support matrix (P1)
An operator runs `medscale doctor` (JSON or text) and sees a machine-readable FHIR support matrix that separates lexical, structural, profile, terminology, references, provenance, and clinical-interpretation axes. Only Patient / Observation / Condition structural extraction is marked partial/qualified where tests exist; everything else remains unsupported or evidence-only.

### US2 — Validator evidence is not authority (P1)
External validator outcomes attached via the existing evidence path remain EvaluationRecord evidence. Doctor and matrix fields explicitly state `validator_is_authority=false`. No PASS/CONFORMANT claim is derived from validator fixtures alone.

### US3 — Loss-aware export stub (P1)
A synthetic FHIR resource round-tripped through the loss-aware export stub preserves unsupported field paths in a loss report that reuses `LossClass`, so export never silently drops interchange information without documentation.

### US4 — No false conformance (P1)
Matrix and doctor axes keep `full_conformance_claimed=false` and `release_ready=false`. Narrow subset support never advertises FHIR R4 full conformance.

## Requirements

- **FR-001**: Typed `FhirSupportMatrix` / `ResourceSupportStatus` in `medscale-contracts` with seven axes and honest READY_BASE defaults for R4 4.0.1 synthetic interchange.
- **FR-002**: Doctor axis + embedded matrix so CLI can display support posture without inventing a CapabilityStatement server.
- **FR-003**: Facade read capability `GetFhirSupportMatrix` returning the canonical matrix.
- **FR-004**: Loss-aware export stub documents per-field `LossClass` for unsupported paths and preserves unsupported field inventory.
- **FR-005**: Facade `ExportFhirLossAware` synthetic-only stub for the export path.
- **FR-006**: Tests prove matrix honesty (no full conformance; axes separated; Patient/Observation/Condition partial structural; profile/clinical unsupported) and loss-aware export.
- **FR-007**: Evidence SUMMARY + LIMITATIONS; BUILD_QUEUE promotes 020 CLOSED_CANONICAL READY_BASE; next Spec 021 minimum lovable workflow (Q07) READY.

## Out of scope

Minimum lovable workflow journey (Q07 / Spec 021); live SMART/NPHIES; profile pack admission; licensed terminology; R5/openEHR/OMOP; full FHIRPath; GraphRAG/plugins/imaging; RELEASE_READY.

## Success Criteria

- Matrix + export + doctor/CLI/facade tests PASS
- Workspace fmt / clippy `-D warnings` / `cargo test --workspace` PASS
- Evidence LIMITATIONS honest; no RELEASE_READY claim
- Queue: 020 CLOSED_CANONICAL; 021 READY (Q07)
