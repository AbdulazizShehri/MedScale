# Security and Adversarial Qualification — Spec 079

Threats from `specs/079-privacy-gate/security.md`, each mapped to the test
that proves the control. Test files:
Core `crates/medscale-core/tests/privacy_gate_079.rs` (C),
storage `crates/medscale-storage/tests/privacy_gate_079.rs` (S),
contracts unit tests in `crates/medscale-contracts/src/privacy_gate.rs` (K),
recognizer unit tests in `crates/medscale-core/src/authority/privacy_recognizers.rs` (R),
CLI `crates/medscale-cli/src/privacy_gate.rs` (L).

| # | Threat | Proof | Result |
|---|---|---|---|
| T1 | Unclassified artifact leaves | C `unclassified_is_local_phi_and_declarations_are_revision_safe_and_durable`; C `egress_fails_closed_and_every_decision_is_persisted` (every boundary: `denied_unclassified`); K `egress_policy_matrix_fails_closed` | pass (run 35802759307) |
| T2 | Source overwritten | C `corpus_transforms_write_new_artifacts_and_bind_receipts_without_leaking_values` (source bytes identical after transform; output id differs; digests bound); Core re-verifies the source digest after the commit | pass (run 35802759307) |
| T3 | Plaintext in persisted rows | C `corpus_...` scans every `privacy_*` row body for all 45 corpus values; C `reidentification_...` checks audit rows never hold the resolved value; K `limitation_vocabulary_makes_no_absence_claim` | pass (run 35802759307) |
| T4 | Key in metadata/backup | C `reidentification_...` searches the metadata file for raw and hex key bytes; backups serialize metadata rows only (S `backup_restore_roundtrips_every_079_row_exactly`) | pass (run 35802759307) |
| T5 | Re-identification without audit or grant | C `reidentification_...` (operator session refused; separate single-capability session; audit row per request, including denials); L `privacy_commands_run_through_core_across_fresh_sessions` | pass (run 35802759307) |
| T6 | Revoked objects still usable | C `egress_...` (revoked receipt and revoked profile deny); C `reidentification_...` (revoked map denies, key destroyed, new transforms refused); S `profile_and_receipt_revocation_are_terminal_and_stale_safe` | pass (run 35802759307) |
| T7 | Output tampered after receipt | C `a_receipt_whose_output_digest_no_longer_matches_denies`; vault reload also refuses a derived artifact whose blob digest differs (existing Spec 016 check) | pass (run 35802759307) |
| T8 | Unavailable recognizer reported clean | C `unavailable_or_failed_recognizers_never_yield_a_clean_residual` (no Pack, non-admitting Pack path, malformed FHIR); K `residual_scan_never_reports_clean_when_a_recognizer_did_not_complete` | pass (run 35802759307) |
| T9 | Cross-Project / cross-scope access | C `unclassified_...` (foreign scope refused; other Project sees its own absent row); C `egress_...` (output classification does not carry into another Project) | pass (run 35802759307) |
| T10 | Stale revision / replay | S `classification_is_unique_per_artifact_and_revision_safe`; C `unclassified_...` (stale expected revision refused) | pass (run 35802759307) |
| T11 | Malformed / hostile input | C `real_phi_malformed_and_bounded_inputs_are_refused_without_writes` (invalid UTF-8, input bound, real-PHI gate, no receipt written); C `unavailable_...` (malformed FHIR) | pass (run 35802759307) |
| T12 | Tampered backup | S `restore_rejects_hand_edited_079_snapshots` (duplicate row, entry-count mismatch, receipt without classification, invalid profile, outcome/reason mismatch); S `edited_row_bodies_fail_closed_on_read`; S `consistency_check_detects_every_invariant_break` | pass (run 35802759307) |
| T13 | Network use | Deterministic grep in `EXACT_RANGE_REVIEW.md` | none found |
| T14 | False completeness claim | K `limitation_vocabulary_makes_no_absence_claim`; every receipt must include `automated_recognition_is_incomplete` (K `receipt_invariants_hold`) | pass (run 35802759307) |

## Honest limits

- Log-capture leakage (stdout/stderr of the whole process) is not captured
  by an automated test; the proof is that no 079 Core or storage module
  contains a print or logging call (`EXACT_RANGE_REVIEW.md`) and that errors
  use fixed strings.
- The OS keyring path (`select_keystore`) is exercised only where the CI
  host provides one; tests use in-memory stores.
