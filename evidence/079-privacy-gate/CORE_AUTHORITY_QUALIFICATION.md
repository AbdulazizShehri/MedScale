# Core Authority Qualification — Spec 079

`crates/medscale-core/tests/privacy_gate_079.rs`: 10 passed, 0 failed in
run `35802759307`. Every call goes through `CoreFacade::dispatch` with a real
session, lease and synthetic vault.

| Behavior | Test |
|---|---|
| Unclassified -> `local_phi`/`default_unclassified`; declare, CAS update, reopen; other Project and foreign scope isolated | `unclassified_is_local_phi_and_declarations_are_revision_safe_and_durable` |
| Incomplete profile refused before any write | `incomplete_profiles_are_refused_before_any_write` |
| All 6 corpus documents: new output artifact, source bytes unchanged, digests and profile revision bound, no expected value in output, FHIR output stays valid JSON, residual clean by admitted recognizers, no corpus value in any 079 row | `corpus_transforms_write_new_artifacts_and_bind_receipts_without_leaking_values` |
| Pseudonyms stable within a map, distinct across maps; map required for pseudonymizing profiles | `pseudonyms_are_stable_within_a_map_and_distinct_across_maps` |
| Re-identification: operator session refused; separate capability; every request audited (returned, unknown, key unavailable, returned, revoked); key never in metadata; revocation destroys key | `reidentification_needs_its_own_capability_is_audited_and_ends_at_revocation` |
| Egress matrix over all 11 boundaries; revoked receipt/profile deny; cross-Project deny; all decisions persisted | `egress_fails_closed_and_every_decision_is_persisted` |
| Output digest mismatch denies | `a_receipt_whose_output_digest_no_longer_matches_denies` |
| Missing Pack, non-admitting Pack, malformed FHIR -> residual `unavailable`, egress denies | `unavailable_or_failed_recognizers_never_yield_a_clean_residual` |
| Admitted ONNX Pack runs as the model recognizer and is recorded with its Pack id and the not-qualified limitation | `admitted_local_model_recognizer_runs_and_is_recorded` |
| Real-PHI gate, invalid UTF-8, input bound refused with no receipt written | `real_phi_malformed_and_bounded_inputs_are_refused_without_writes` |

Unit tests in Core (run `35802759307`): 12 recognizer tests including RFC 4231
HMAC vectors and the corpus benchmark (`RECOGNIZER_BENCHMARK.md`), and 2
`privacy_gate` helper tests.
