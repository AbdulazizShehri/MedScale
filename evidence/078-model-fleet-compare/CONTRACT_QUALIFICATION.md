# Contract Qualification — Spec 078

Frozen contracts (`specs/078-model-fleet-compare/contracts.md`) mapped to
Rust source and tests. The freeze record itself is section 6 of
`contracts.md` (T078-01).

| Contract | Rust path | Proving tests |
|---|---|---|
| `AgentLaneStatus` {Active, Retired} | `crates/medscale-contracts/src/model_fleet.rs` | `agent_lane_status_round_trips` |
| `LaneTransform` (closed-empty) | same, `pub enum LaneTransform {}` | `lane_transform_is_closed_empty` |
| `LanePolicy` (narrowing only) | same, `LanePolicy::validate_within` | `lane_policy_validate_within_rejects_superset_tool_kind`, `lane_policy_validate_within_rejects_out_of_manifest_artifact`; Core `lane_policy_naming_an_ungranted_tool_kind_is_refused_before_any_write`, `lane_policy_naming_an_out_of_manifest_artifact_is_refused_before_any_write`, `lanes_cannot_reach_each_others_context_and_lane_policy_holds_on_the_direct_tool_path` |
| `AgentLane` | same, `AgentLane::new`, `check_mutation` | `agent_lane_new_validates_role_label`; Core `lane_create_get_list_retire_roundtrip_and_survives_reopen`, `lane_binding_to_revoked_missing_or_foreign_objects_is_refused`, `lane_retire_is_stale_revision_safe_and_not_repeatable`, `lane_role_label_is_bounded_and_stored_as_inert_text` |
| `FleetRunState` + transition table | same, `can_transition_to`, `is_terminal`, `FleetRun::aggregate_state` | `fleet_run_state_transition_table_is_frozen`, `fleet_run_state_terminal_classification`, `fleet_run_aggregate_state_covers_every_case`; Core `fleet_reaches_completed_and_partially_failed_from_real_lane_outcomes`, `fleet_cancel_semantics_follow_the_frozen_contract` |
| `FleetRun` | same, `FleetRun::new` | `fleet_run_new_rejects_empty_and_oversized_prompt`; Core `dispatch_refusals_write_nothing`, `fleet_state_and_bindings_survive_reopen` |
| `LaneRunRef` | same | Core `dispatch_binds_and_starts_one_independent_run_per_lane`; storage `an_agent_run_cannot_be_bound_to_two_lane_run_refs` |
| `ComparisonObservationKind` / `ComparisonObservation` | same, `requires_multiple_lanes`, `validate` | `comparison_observation_kind_requires_multiple_lanes_is_correct`, `comparison_observation_validate_enforces_multi_lane_kinds` |
| `ComparisonRequest` / `ComparisonReport` (no score, no winner) | same, `ComparisonReport::validate` | `comparison_report_validate_rejects_lane_in_both_lists`; Core `two_real_lanes_produce_a_factual_grounded_report`, `partial_failure_reports_name_the_excluded_lane_and_bad_states_are_refused`, `recompute_appends_a_new_report_and_never_touches_lane_runs`; Core unit `computation_has_no_score_rank_or_winner_logic` |
| Lane-plurality decision (section 5) | no uniqueness constraint on `agent_identity_id` | `LANE_PLURALITY_QUALIFICATION.md` |
| Envelope operations | `crates/medscale-contracts/src/envelopes/mod.rs` (+117) | exercised end to end by Core `tests/model_fleet_078.rs` and CLI `lane_commands_run_through_core_across_fresh_sessions` |

## Result

All tests named above passed on code head `547fb41` in CI run `35768404980`
(contracts lib: 81 passed including the 12 model_fleet tests; Core
`model_fleet_078`: 18 passed; storage `model_fleet_078`: 14 passed; see
`EXACT_HEAD_QUALIFICATION.md`). The final candidate head is qualified by the
run recorded there.
