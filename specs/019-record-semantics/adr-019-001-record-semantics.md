# ADR-019-001: Append-only record semantics READY_BASE

**Status**: Accepted  
**Context**: Q06 requires precision-aware time, amendments, explicit identity reconciliation, and missingness without inventing Instant or overwriting history.

**Decision**:

1. Extend `MedicalTime` in place (serde-default timezone field) rather than a parallel type.
2. Ship `AmendmentRecord` as a first-class StoredObject class persisted through Spec 016 durable snapshot.
3. Expose one facade op `AmendAssertion`; identity unresolved listing remains a pure library helper.
4. Timeline excludes superseded priors but retains prior objects for provenance ReadObject.

**Consequences**:

- READY_BASE is synthetic-only; RELEASE_READY stays false.
- FHIR interchange precision matrix deferred to Spec 020.
- Callers must not upgrade Year→Instant; helpers fail closed.
