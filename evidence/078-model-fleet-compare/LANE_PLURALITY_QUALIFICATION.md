# Lane Plurality Qualification — Spec 078

## Real model inventory

The repository admits exactly one Pack for real local execution:
`pack-tiny-token-classifier-v0`
(`evidence/069-real-local-model-runtime-hf-pack-path/fixtures/`), runtime
`tract_onnx_token_classification_v1`, labels `["O", "ENTITY"]`, fixed
sequence length 4, deterministic. Spec 008's `pack-fixture-ner-v0` admits for
identity registration but declares `fixture_runtime_v0` and cannot execute
(`execute_agent_run` refuses it with `DigestMismatch` against an ONNX-bound
identity, which the tests use as a *real* execution failure).

Per `SPEC_078_PROMOTION.md` and `contracts.md` section 5, 078 does not build a
second inference engine and does not fabricate a second model.

## How the two-lane closure fixture proves independence

`crates/medscale-core/tests/model_fleet_078.rs::fleet_fixture`:

| | lane a | lane b |
|---|---|---|
| AgentIdentity | agent a (grants read + search) | agent b (grants read only) |
| Pack | `pack-tiny-token-classifier-v0` | `pack-tiny-token-classifier-v0` |
| ContextManifest | {a1, a2} (real source records) | {b1} (real source record) |
| LanePolicy | tools {read}, artifacts {a1} | inherit |

`two_real_lanes_produce_a_factual_grounded_report` then asserts:
- two distinct `AgentRun` ids (`security.md` T4), each with its own
  `ModelOutput` turn from real ONNX execution and its own `RunReceipt`
  (the report's two `ResourceRuntimeFact` observations are grounded in two
  distinct receipt ids);
- lane b cannot read lane a's artifact, and lane a's policy blocks a2, b1
  and search on the direct tool path (T2/T3);
- the report's observations, computed over the committed rows.

## What the real run can and cannot show (honest scope)

Both lanes run the same prompt through the same deterministic Pack, so
their token labels are identical. Over the real run:

| Kind | Real two-lane run | Where else proven |
|---|---|---|
| Agreement | yes: one observation over both lanes | compare unit tests |
| SchemaValidity | yes: both lanes, well-formed | unit: malformed payload |
| ResourceRuntimeFact | yes: both lanes, from each lane's own receipt | unit |
| UnsupportedClaim | yes: lane a's proposal cites a2, outside its lane policy (Spec 077 cites the whole bound manifest) | unit: entity with no in-context evidence |
| EvidenceOverlap | no, by design: the fixture contexts are disjoint | unit: overlapping citations |
| Abstention | emitted only if the fixture model labels every token `O` for the fixture prompt; the real-run test does not assert either way | unit: all-`O` output; completed run without a proposal |
| Disagreement | **cannot arise** from one deterministic Pack on one prompt | unit: differing labels; differing tokenization |
| ContradictionCandidate | **cannot arise** (only one entity label exists) | unit: two different entity labels on one token |

The comparison engine evaluates every kind on every report and emits only
what the committed data supports. It never invents a Disagreement to show
the feature working; the real-run test asserts that no Disagreement
appears. Showing a genuine Disagreement or ContradictionCandidate between
real lanes needs a second admitted Pack with a different model or label
set, which is outside this spec's authority (no new Pack admission).

Partial failure is real: in `partial_failure_reports_name_the_excluded_lane_and_bad_states_are_refused`,
lane b executes against a mismatched Pack directory, Spec 077 refuses the
execution, the lane run closes `Failed` with a fixed reason, the fleet lands
`PartiallyFailed`, and the report names lane b in `excluded_lane_ids`.
