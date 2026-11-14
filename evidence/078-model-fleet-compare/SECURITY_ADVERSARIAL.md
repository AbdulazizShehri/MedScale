# Security / Adversarial Qualification — Spec 078

Each `specs/078-model-fleet-compare/security.md` threat maps to the exact
control and the test that proves it. "Core test" = `crates/medscale-core/
tests/model_fleet_078.rs`, "storage test" = `crates/medscale-storage/tests/
model_fleet_078.rs`, "compare unit" = `crates/medscale-core/src/authority/
model_fleet_compare.rs` tests. The CI run that qualifies these results is
recorded in `EXACT_HEAD_QUALIFICATION.md`.

| Threat | Control | Proof |
|---|---|---|
| T1 effect escalation | Comparison is a pure read plus one report insert; no path to promote/amend/actions/effects. | Core unit `module_has_no_path_to_promotion_amendment_effects_or_actions` (source scan of `model_fleet.rs` + `model_fleet_compare.rs`); Core test `recompute_appends_a_new_report_and_never_touches_lane_runs` (lane runs and fleet byte-equal before/after two computes). |
| T1 no score/rank/winner | `ComparisonObservation` has no numeric field (frozen shape, T078-01); the computation never reads `Proposal.confidence` and has no ordering/selection. | Compare unit `computation_has_no_score_rank_or_winner_logic` (source scan: no `score`, `rank`, `winner`, `max_by`, `min_by`, `sort_by`, `f32`, `f64`, `.confidence`); Core test `two_real_lanes_produce_a_factual_grounded_report` asserts the serialized report contains none of `score`/`rank`/`winner`/`confidence`/`best`. |
| T2 lane policy escalation | `LanePolicy::validate_within` at creation and again at every dispatch against live-resolved Spec 077 manifests; dispatched run binds exactly the lane's identity/context. | Core tests `lane_policy_naming_an_ungranted_tool_kind_is_refused_before_any_write`, `lane_policy_naming_an_out_of_manifest_artifact_is_refused_before_any_write`, `dispatch_binds_and_starts_one_independent_run_per_lane`, `dispatch_refusals_write_nothing` (revoked identity re-checked at dispatch). |
| T2 on Spec 077's direct tool path | Facade `AgentToolInvoke` arm runs `require_lane_policy_allows_tool` before Spec 077's own checks for any lane-bound run. | Core test `lanes_cannot_reach_each_others_context_and_lane_policy_holds_on_the_direct_tool_path` (sibling artifact, excluded in-manifest artifact, excluded tool kind, malformed args all `Unauthorized`; in-policy read `Executed`). |
| T3 cross-lane leakage | Each lane runs its own Spec 077 run against its own context; no 078 function passes one lane's data into another's dispatch. | Core test above (lane b cannot read lane a's artifact: Spec 077 records `Refused`); storage invariant 3 (bound run must carry the lane's own identity/context), storage test `verify_model_fleet_consistency_detects_every_invariant_break`. |
| T4 fake diversity | UNIQUE `agent_run_id` index + explicit Core check; closure fixture uses two identities, disjoint contexts, different policies. | Storage test `an_agent_run_cannot_be_bound_to_two_lane_run_refs`; restore edit "second binding of one agent run" rejected; Core test asserts `run_a != run_b` and a `ModelOutput` turn on each. See `LANE_PLURALITY_QUALIFICATION.md`. |
| T5 stale write | CAS on every mutable 078 row; frozen transition table. | Storage `lane_retire_and_fleet_transition_are_stale_revision_safe`; Core `lane_retire_is_stale_revision_safe_and_not_repeatable`, stale dispatch/cancel cases, second-dispatch conflict, terminal-fleet execute/cancel conflicts. |
| T6 cancel race / zombie lane | Fleet cancel only through Spec 077 `cancel_agent_run`; a lane that finished first keeps its state; `Cancelled` only when every lane ended `Cancelled`; restore invariant "a Cancelled fleet binds only Cancelled runs". | Core `fleet_cancel_semantics_follow_the_frozen_contract` (completed lane survives cancel as `Completed`, fleet `PartiallyFailed`; directly-failed lane kept `Failed`, fleet `Failed`); storage tamper "cancelled fleet over completed/failed lanes". |
| T7 hostile metadata | Bounded text on role label, task prompt, observation detail; parameterized SQL; CLI escapes role labels. | Core `lane_role_label_is_bounded_and_stored_as_inert_text` (empty, blank, over-max refused; SQL-like/markup/bidi label stored verbatim); compare unit `details_are_bounded_and_the_report_is_capped`; restore edit "oversized role label" rejected. |
| T8 history rewrite | No update API for refs/reports; restore re-verifies every cross-row invariant. | Storage `verify_model_fleet_consistency_detects_every_invariant_break` (ten tampers), `restore_rejects_hand_edited_078_snapshots` (seven edits with recomputed outer digest, each asserted by its failure message). |
| T9 content leakage | Failed-lane `RunReceipt.failure_reason` is a fixed classification, never the raw error; CLI prints prompt length, not prompt text, for fleet runs; no new cache. | Code: `bounded_failure_reason` in `authority/model_fleet.rs`. Honest limit: as in Specs 076/077, log-capture leakage is proven by inspection (078 adds no logging call), not by an automated log-capture test. |
| T10 half-committed state | Per-row atomic inserts; dispatch order create -> bind -> start (a run never starts unbound); migration journal fail-closed. | Storage `uncommitted_078_writes_are_absent_after_reopen`, `failed_lane_binding_leaves_sibling_intact_and_only_an_unstarted_unbound_run`, `crash_mid_migration_fails_closed_and_pre_migration_backup_recovers`; Core `dispatch_refusals_write_nothing` (non-admitted Pack refused before the fleet leaves `Pending`). |
| T11 supply chain | No new dependency. | `git diff ff2e677..HEAD -- '**/Cargo.toml' Cargo.lock` is empty (recorded in `EXACT_HEAD_QUALIFICATION.md`); `cargo-deny` job green. |

## Findings fixed during implementation (not hidden)

1. **Dispatch could strand a fleet in `Running` with no lanes** if a lane's
   Pack was not admitted in the current Core process: Spec 077's
   `create_agent_run` re-checks admission, but the preflight did not. Found
   by reading Spec 077's `require_active_identity` before the first CI run;
   fixed by checking admission in preflight; regression in
   `dispatch_refusals_write_nothing`.
2. **Lane policy was decorative on Spec 077's direct tool path.** A
   lane-bound run could use `AgentToolInvoke` with the identity's full grant.
   Fixed by the facade guard (T2 row above).
3. **Spec 077 cites the whole bound manifest as proposal evidence**, even
   for a lane whose policy narrows the context. 078 cannot change Spec 077,
   so the comparison reports it as an `UnsupportedClaim` ("cites N
   artifact(s) outside its lane context"), and the closure-fixture test
   asserts that it is surfaced.
4. **Atomic per-lane dispatch conflicted with "unmodified Spec 077"**:
   reconciled in `migration.md` section 5 (see
   `STORAGE_MIGRATION_RECOVERY.md`).
