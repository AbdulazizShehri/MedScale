# Contracts: Spec 022

Primary typed surface: `medscale_contracts::doctor::ReleaseQualificationDoctorStatus` on `DoctorReport.release_qualification`.

Serialization: serde snake_case JSON for `medscale doctor --json`.

Honesty invariants (tests):

- `release_ready == false`
- `macos_qualified == false`
- `mobile_release_qualified == false`
- `branch_protection_configured == false`
- `missing_evidence_classes` non-empty
- `prep_ready_base == true` after Spec 022 ships
