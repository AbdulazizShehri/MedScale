# T077-07 Implementation — Model Pack lane + AgentProposal

## Scope

Wire a real, local, zero-network model execution against an
`AgentRun`'s bound `AgentIdentity`/Pack, and persist the output as an
`AgentProposal` linking to a real `Proposal` row with
`producer: ProducerKind::Agent(agent_identity_id)`.

## Which `PackRuntimeAdapter`, and why

**`medscale_pack::OnnxTokenClassifierRuntime`** (`runtime_requirements:
tract_onnx_token_classification_v1`). This is the only `PackRuntimeAdapter`
implementation in this repository genuinely qualified for real local model
execution:

- `FixtureRuntime` (`medscale-pack/src/runtime.rs`) is explicitly
  documented in its own doc comment as "Spec 008 default: no native model
  engine" -- it is a deterministic stub whose `run_fixture` never touches
  any actual model weights. Using it would not satisfy "a real ... local
  model execution."
- `OnnxTokenClassifierRuntime` (`medscale-pack/src/onnx_runtime.rs`,
  Spec 069) is a real `tract_onnx`-backed CPU inference engine, already
  wired into Core for interactive pack evaluation
  (`RequestBody::PacksEvaluateLocal` in `facade.rs`), and already exercised
  end-to-end by `crates/medscale-core/tests/real_local_model_runtime_069.rs`.

**Fixture pack used:** `evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0`
(`pack_id: pack-tiny-token-classifier-v0`), *not* the
`pack-fixture-ner-v0` fixture used throughout T077-03 through T077-06.
Cross-checked by reading `pack-fixture-ner-v0`'s own manifest: it declares
`"runtime_requirements": "fixture_runtime_v0"` and carries exactly one
`fixture_bytes` artifact (`payload.txt`) -- no ONNX model, tokenizer, or
labels files at all. It remains admissible (and was used) for identity
registration in earlier tasks because those tasks never actually executed
a model; T077-07 is the first task that does, so it is the first to need
the real ONNX-compatible fixture.

## Design decisions

**`local_path` is caller-supplied on every call**, exactly mirroring
`RequestBody::PacksEvaluateLocal`. `PackManifestV0` is purely
content-addressed (relative paths + digests inside the pack directory) and
never stores an on-disk location, so there is nothing to cache -- this is
an existing architectural fact, not a new design choice.

**Two independent identity checks before any model code runs:**

1. The admitted manifest read from `local_path` must match this run's
   bound `AgentIdentity.pack_id`/`pack_version` exactly (not merely "a"
   pack with a matching id) -- "exact model Pack identity."
2. That same manifest must also match what is *actually admitted* right
   now in this vault's `PackStore` (content digest, epoch, version) --
   "exact runtime identity," so a caller cannot point `local_path` at an
   unadmitted or since-superseded directory that merely claims the same
   identity.

Either mismatch fails closed with `AuthorityError::DigestMismatch` before
`OnnxTokenClassifierRuntime` is even constructed.

**`synthetic_only: true` is mandatory**, mirroring
`PackEvaluationRequest.synthetic_only` exactly (`facade.rs`'s
`PacksEvaluateLocal` arm). Real PHI flowing through a local model runtime
via MedAgent requires a later, explicit gate this spec does not grant;
`synthetic_only: false` refuses with `ExternalGateRequired { gate:
"REAL_PHI_MODEL_RUNTIME" }` before touching the run, the pack, or the
runtime at all. This keeps MedAgent's actual behavior consistent with the
founder's standing constraint against `PRIVATE_DATA_READY`/`RELEASE_READY`
claims.

**The `Proposal` is constructed directly in `medagent.rs`, not through
`RequestBody::CreateProposal`.** `contracts.md` section 6/7 describes
submission "through the existing `CreateProposal` capability," but that
capability's frozen request shape (`facade.rs`'s `CreateProposal` arm)
always sets `producer: ProducerKind::Rule` with no field to override it --
modifying that shape was not authorized by this promotion (only the
additive `ProducerKind::Agent(OpaqueId)` variant was). `execute_agent_run`
therefore mirrors that same arm's exact construction pattern
(`store.alloc_id("proposal")`, the same `Proposal` struct, inserted into
the same `store`) with the one field this spec's own contracts.md
addition exists to carry. The claim content and object type are still
fully reused, as the design decision requires; only the request-routing
mechanism differs from a literal reading of "through the capability."

**`evidence_refs` link to the run's bound `ContextManifest` artifacts.**
If that context manifest cannot be read (should be unreachable in
practice -- `create_agent_run` already validated its existence and
`ContextManifest` is immutable once created), `execute_agent_run` now
propagates the real error via `?` rather than silently defaulting to an
empty evidence list; a proposal with misleadingly empty evidence would be
a worse failure mode than an outright error.

**No prepared-model cache reuse.** `PacksEvaluateLocal` caches a prepared
ONNX session (`CoreFacade.prepared_models`, keyed by content digest)
because interactive pack evaluation is repeated, latency-sensitive
traffic. An `AgentRun`'s model execution happens once per run, not in a
hot loop, so `OnnxTokenClassifierRuntime::run`'s own documented
"convenience one-shot path" is used instead -- a deliberate scope
simplification, not an oversight; the cache can be added later if
profiling shows a real need.

## Files changed

```text
crates/medscale-contracts/src/envelopes/mod.rs
  - Capability::AgentRunExecute
  - RequestBody::AgentRunExecute{run_id,local_path,max_tokens,synthetic_only}
  - ResponseBody::MedAgentRunExecuted{turn,proposal}

crates/medscale-core/src/authority/medagent.rs
  - MedAgent::execute_agent_run

crates/medscale-core/src/authority/facade.rs
  - 1 dispatch arm (u32 -> usize max_tokens conversion) + capability_matches pair

crates/medscale-core/src/cli_session.rs
  - CliSession::medagent_run_execute

crates/medscale-cli/src/medagent.rs
  - MedAgentCmd::RunExecute

crates/medscale-core/tests/medagent_077.rs
  - onnx_fixture_pack()/Harness::install_onnx_fixture_pack helpers,
    running_run_on_onnx_pack setup helper
  - 4 new tests (see below)

specs/077-medagent-workbench/tasks.md
evidence/077-medagent-workbench/T077-07_IMPLEMENTATION.md (this file)
```

## Tests

`crates/medscale-core/tests/medagent_077.rs`, all through real
`CoreFacade::dispatch` against the real Spec 069 ONNX fixture pack
(input `"alice visited clinic today"`, `max_tokens: 4`, matching
`real_local_model_runtime_069.rs`'s own known-good values for this
fixture's tokenizer/`fixed_sequence_length`):

1. `real_local_model_execution_produces_agent_proposal` -- executes for
   real, asserts the `ModelOutput` turn's payload names the exact runtime
   id, reads the underlying `Proposal` object back via
   `Capability::ReadObject` and asserts `producer: agent(..)` (note:
   `ProducerKind` carries `#[serde(rename_all = "snake_case")]`, so the
   JSON key is lowercase `agent`, not `Agent`) and non-empty
   `evidence_refs`, and asserts the run's turn sequence is exactly
   `prompt_submitted`/`model_output`.
2. `non_synthetic_execution_is_refused_closed` -- `synthetic_only: false`
   refuses with `ExternalGateRequired`.
3. `execution_against_mismatched_pack_directory_fails_closed` -- a
   `local_path` pointing at a second, really-admitted pack (the Spec 008
   fixture) that is not the one this identity is bound to fails closed
   with `DigestMismatch`.
4. `execution_on_a_non_running_run_is_refused_closed` -- a `Pending`
   (never started) run refuses execution with `Conflict`.

No local compile/test run was possible (MSVC linker absent, same
constraint as every prior spec). `cargo fmt --check` is clean across the
whole workspace after this change.

## Exact-head CI qualification

Green on the first push, no fixes needed -- including the real ONNX
inference test actually running and passing in CI on all three platforms:

```text
HEAD 376b8d1 -> SUCCESS, run 35721534239, 6/6.
```

T077-07 is qualified at exact head `376b8d1b9ac4283176b61f35f2a97d6c2f790ea6`.
