# Contract Qualification — Spec 079

Freeze record: `specs/079-privacy-gate/contracts.md` section 8.
Module: `crates/medscale-contracts/src/privacy_gate.rs`.

| Contract | Proving tests (all pass in run `35802759307`) |
|---|---|
| Closed vocabularies (15 enums) | `every_vocabulary_round_trips_and_is_closed` |
| `DataClass` order and broker mapping | `data_class_restrictiveness_is_strictly_ordered` |
| `ArtifactClassification` basis rules | `classification_basis_invariants_hold` |
| `PrivacyPolicyProfile` completeness | `profile_must_map_every_kind_exactly_once`, `profile_op_lookup_and_pseudonym_flag` |
| `ResidualScanResult` derivation | `residual_scan_never_reports_clean_when_a_recognizer_did_not_complete` |
| `DeidReceipt` invariants | `receipt_invariants_hold` |
| `ReceiptLimitation` (no absence claim) | `limitation_vocabulary_makes_no_absence_claim` |
| Pseudonym shape | `pseudonym_shape_is_exact` |
| `decide_egress` policy | `egress_policy_matrix_fails_closed` |
| `EgressDecision` outcome/reason | `egress_decision_outcome_must_match_reason` |
| `SensitiveSpan` offsets | `span_validation_rejects_bad_offsets` |
| `ReidentificationAudit` bounds | `reidentification_audit_is_bounded` |

13 contract tests. Envelope additions (10 capabilities, 18 request bodies, 13
response bodies) are exercised end to end by the Core, CLI and Desktop tests.
