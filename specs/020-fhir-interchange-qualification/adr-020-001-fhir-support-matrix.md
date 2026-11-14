# ADR-020-001: Honest FHIR support matrix + loss-aware export

**Status**: Accepted  
**Context**: Q08 requires bounded FHIR interchange claims so a narrow R4 subset is not mistaken for conformance.

**Decision**:

1. Publish a typed `FhirSupportMatrix` / `ResourceSupportStatus` in `medscale-contracts` with seven explicit axes.
2. Surface the matrix through doctor and a read-only facade capability; do not imply an online FHIR server.
3. Implement a synthetic loss-aware export stub that reuses `LossClass` and preserves unsupported field paths in a loss inventory.
4. Keep external validator outcomes as evidence only (`validator_is_authority=false`).
5. Keep `full_conformance_claimed=false` and `release_ready=false`.

**Consequences**:

- Operators can inspect exact support honesty via CLI.
- Export callers must read field losses; silent drop is forbidden by contract.
- Workflow UX (Q07) remains Spec 021; this unit only qualifies interchange claims.
