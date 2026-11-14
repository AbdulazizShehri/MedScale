# Spec 020 evidence summary

**Status**: CLOSED_CANONICAL READY_BASE — FHIR / interchange qualification (Q08)  
**ADR**: ADR-020-001 honest support matrix + loss-aware export

## Delivered

- Typed `FhirSupportMatrix` / `ResourceSupportStatus` with seven axes (lexical / structural / profile / terminology / references / provenance / clinical_interpretation)
- READY_BASE honesty: Patient / Observation / Condition partial structural; profile + clinical interpretation unsupported; no full conformance
- Doctor `fhir_interchange` + embedded `fhir_support_matrix`; CLI text/JSON display
- Facade `GetFhirSupportMatrix` + `ExportFhirLossAware`
- Loss-aware export stub reuses `LossClass` and preserves unsupported field paths
- Tests: `fhir_interchange_020` (+ contracts unit tests)
- `validator_is_authority=false`; external validator remains evidence only

## Not claimed

See LIMITATIONS.md.
