# ADR-036-001 — Synthetic MESC verifier READY_BASE

## Decision
Implement an independent MedScale synthetic release-dir verifier with deny-unknown-fields manifests, size/digest checks, rights/SBOM presence, and producer-epoch anti-rollback/replay — without authorizing product Pack admit.

## Consequences
`MESC_RELEASED_ARTIFACT` remains `NOT_AVAILABLE` until a real immutable upstream release qualifies. Spec 012 stays blocked on assets; Spec 036 closes the in-repo verifier gap.
