# Spec 012 decoupling record (prospective canonical correction)

Historical records in this directory (GATE_CHECK.md, SUMMARY.md) remain
historical truth: at observation time, no qualifying TheHalfMoon/MESC
release existed and the admit lane was recorded as blocked. This file
records the prospective canonical correction; it does not rewrite history.

## Canonical decision

```text
SPEC_012 = DEFERRED_BY_CANONICAL_DESIGN (optional external integration)
MESC_REQUIRED_FOR_MEDSCALE = FALSE
MESC_RELEASE_BLOCKER = FALSE
MESC_ARTIFACT_AVAILABLE = false
MedScale core impact = none
MedScale release impact = none
```

## Disposition audit

- KEEP (generic MedScale capability): fail-closed artifact verification,
  hashes, provenance/rights/license verification, manifest schema validation,
  anti-rollback, local admission boundaries, doctor/status reporting.
- ISOLATE: MESC-specific naming stays behind the existing `mesc` contract
  module and `verify_mesc_release_dir`; it is one optional producer lane of
  the generic Pack/model admission boundary, never record authority.
- REMOVED FROM GATES: any reading of "MedScale cannot finish until MESC
  publishes assets."

## Proof

- `crates/medscale-core/tests/mesc_optional_decoupling.rs`: core starts,
  workflow READY_BASE, doctor `required=false` / `NOT_CONFIGURED`,
  `blocks_release()=false`, malformed artifacts fail closed, legacy doctor
  JSON still parses.
- Existing `mesc_admit_012` / `mesc_verifier_036` suites unchanged in force:
  user-attempted admission still returns
  `ExternalGateRequired(MESC_RELEASED_ARTIFACT)`.

## Security invariants preserved

Untrusted artifact != trusted model; presence != admission; hash != verified;
manifest != provenance; model output = proposal, never clinical fact.
