# Spec 078 — Model Fleet + Compare

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promoted:** 2026-09-22
**Base SHA:** `ff2e677294b1ea8704bbeefa605d129c1a99e8f2`
**Target branch:** `spec/078-model-fleet-compare`
**Dependency:** canonical MedScale through Spec 077 (`CLOSED_CANONICAL`)
**Promotion authority:** `docs/planning/SPEC_078_PROMOTION.md`

## 1. Problem

Spec 077 gives MedScale a governed MedAgent Workbench, but it proves exactly
**one** admitted local model Pack lane per run. A researcher who wants to
see how two independently-configured agent lanes handle the *same* task —
whether they agree, where they contradict, what evidence each cites, what
each declines to claim — has no way to do that without manually running two
separate `AgentRun`s and eyeballing the difference themselves, with no
structural guarantee that the comparison stays honest (no invented
consensus, no silent winner, no leaked permission between the two runs).

## 2. Goal

Add a local-first **Model Fleet + Compare** layer on top of Spec 077: an
`AgentLane` wraps an existing `AgentIdentity`/`ContextManifest` with
explicit, lane-scoped policy; a `FleetRun` binds two or more lanes to the
same task and drives each lane's own independent, unmodified Spec 077
`AgentRun` to completion (or explicit partial failure); a
`ComparisonReport` then observes the completed lanes' `AgentProposal`/
`RunReceipt` output and records agreement/disagreement, contradiction
candidates, evidence/citation overlap, unsupported-claim candidates,
abstention, schema validity, and resource/runtime facts — **never** a
quality score, a ranking, or a declared winner.

At closure, a user can open a Project, create two or more `AgentLane`s
against existing admitted `AgentIdentity`/`ContextManifest` records, start
a `FleetRun` that dispatches the same task to every lane, watch each lane's
run execute independently and cancel individually if needed, see one lane
fail while another completes without the whole fleet silently collapsing,
view a `ComparisonReport` that is honest about overlap/disagreement/
unknowns without inventing a verdict, close/reopen MedScale, and see every
fact resolve identically — with zero network dependency, zero permission
leakage between lanes, and zero modification to Spec 077's own contracts.

## 3. User and operational scenarios

1. An operator creates two `AgentLane`s in a Project, each bound to an
   already-registered `AgentIdentity` and `ContextManifest`, each carrying
   an explicit role label and lane policy (which tool kinds/context this
   lane's run may use, never wider than the underlying identity/manifest
   already allow); both lanes persist across restart.
2. A researcher starts a `FleetRun` naming those two lanes and one task
   (prompt); each lane's own real `AgentRun` begins independently through
   Spec 077's unmodified `execute_agent_run` path.
3. Both lanes complete; the researcher requests a `ComparisonReport` over
   the `FleetRun`; the report shows where the two lanes' `AgentProposal`
   outputs agree, where they contradict, which evidence references overlap
   or diverge, which claims either lane makes without supporting evidence,
   and where either lane abstained — with no score and no winner.
4. One lane's run fails (model/tool error) while the other completes; the
   `FleetRun` reaches an explicit partial-failure state; the surviving
   lane's output and the failure detail are both inspectable; no fabricated
   comparison is produced pretending both lanes contributed.
5. A researcher cancels one lane's run mid-flight; that lane transitions to
   `Cancelled` through Spec 077's existing cancel path; the other lane's run
   is unaffected and continues independently; the `FleetRun` reflects the
   mixed state honestly.
6. Lane A's run cannot read Lane B's context, tools, or in-flight results at
   any point, even though both belong to the same `FleetRun` — refused
   structurally, not merely untested.
7. **Amended at T078-01:** no data-classification/privacy-sensitivity
   primitive exists anywhere in this repository yet (`DataClass` is Spec
   079's own future contract); a `ComparisonReport` therefore carries no
   classification field in this spec's v1 shape at all, rather than a
   fabricated or misapplied one (`contracts.md` section 4's reconciliation
   note). This scenario is retained here as a forward-looking statement of
   intent for whichever later spec adds a real classification primitive,
   not as a v1 requirement.
8. No code path in this spec calls `PromoteProposal`, `TransitionEffect`,
   or any controlled-action capability from `FleetRun`/`ComparisonReport`
   output, and no code path automatically resolves a lane disagreement into
   a single "correct" answer.
9. The application restarts; every lane/fleet-run/comparison record
   resolves identically, and every pre-078 (074/075/076/077) object and
   workflow is unaffected.
10. A vault created before Spec 078 opens and migrates (v6 -> v7) without
    changing old object IDs or breaking old CLI/Desktop workflows.
11. With network egress disabled, every 078 workflow remains fully useful;
    every lane in a fleet run executes fully offline, exactly as a Spec 077
    `AgentRun` already does.
12. Because this repository currently has exactly one real, qualified local
    model Pack (`pack-tiny-token-classifier-v0` via
    `OnnxTokenClassifierRuntime`), a two-lane closure-gate fixture binds
    both lanes to that same admitted Pack through two distinct
    `AgentIdentity` registrations (or one identity reused by two lanes with
    distinct `ContextManifest`/policy — the exact shape is an implementation
    decision recorded in `contracts.md`), with deliberately different lane
    policy so each lane's run is genuinely independent, not a duplicated
    result (`docs/planning/SPEC_078_PROMOTION.md`, "Lane-plurality scope
    decision").

## 4. Required outcomes

1. Bounded contracts for `AgentLane`/`LanePolicy`/`LaneTransform`,
   `FleetRun`/`FleetRunState`/`LaneRunRef`, `ComparisonRequest`/
   `ComparisonObservation`/`ComparisonReport`.
2. Reuse of Spec 077's `AgentIdentity`, `AgentCapabilityManifest`,
   `ContextManifest`, `AgentRun`/`AgentRunState`/`AgentTurn`,
   `ToolInvocation`/`ToolReceipt`, `RunReceipt`, `AgentProposal` exactly as
   frozen; no modification to any Spec 077 contract, storage table, or Core
   authority function.
3. Encrypted-vault persistence for all durable 078 rows inside the existing
   storage architecture (schema v6 -> v7, additive).
4. `AgentLane`/`LanePolicy` vertical slice: create a lane bound to an
   existing identity/context with explicit policy, reopen and see identical
   state.
5. `FleetRun` vertical slice: bind two or more lanes to one task, dispatch
   each lane's own independent `AgentRun` through the unmodified Spec 077
   path, track per-lane and overall fleet state including explicit
   partial-failure states.
6. Comparison vertical slice: a real `ComparisonReport` computed over two or
   more completed (or partially-failed) lanes' actual `AgentProposal`/
   `RunReceipt` output — agreement/disagreement, contradiction candidates,
   evidence/citation overlap, unsupported-claim candidates, abstention,
   schema validity, resource/runtime facts; structurally no score/winner
   field.
7. Fleet/comparison history persisted as inspectable Project-scoped
   artifacts.
8. CLI inspect/mutation paths through Core with stable machine-readable
   output, matching Spec 074/075/076/077 conventions.
9. Native Slint Desktop fleet/compare panel backed exclusively by Core for
   at minimum a fleet list/detail view, per-lane status, and comparison
   report view, reusing the current design system.
10. Migration/reopen/backup-recovery evidence from representative pre-078
    vault fixtures (including populated 074/075/076/077 vaults).
11. Exact-head closure evidence mapping every acceptance criterion to proof.

## 5. Explicit non-goals

Spec 078 MUST NOT implement:

- Privacy Gate expansion, de-identification, or a new data-class/egress
  policy engine beyond reusing existing classification inheritance
  (Spec 079);
- Governed Browse, web search, or any browser/external-provider delegate
  adapter for a lane (Spec 080) — external/browser providers remain denied;
  "optional delegate adapters" named in the roadmap text remain a future
  extension point this spec does not build;
- AudioFlow, Analytics Gate, Clinical Graph/Research Canvas, Hub, Compute,
  R Workspace, Community Extensions, Research/Evidence Packs, Institutional
  Adapters, Federation (Specs 081-092);
- a new general-purpose local-model inference engine or a second Pack
  admission/qualification pipeline; 078 compares output from Packs/runtimes
  `medscale-pack` already qualifies (Spec 008/069), it does not admit new
  models;
- any modification to a Spec 077 contract, storage table, or Core authority
  function; `FleetRun` orchestrates existing `AgentRun`s, it never forks or
  re-implements run execution, tool dispatch, or proposal submission;
- any quality score, confidence ranking, or declared "winner" across lanes,
  anywhere in contracts, storage, Core, CLI, or Desktop;
- permission union across lanes, or any code path letting one lane's
  in-flight run read another lane's context, tools, or results;
- multi-agent orchestration where lanes communicate with or influence each
  other's execution; a `FleetRun` only aggregates already-independent
  completed (or failed) results for comparison after the fact;
- any mechanism by which fleet/comparison output automatically performs a
  clinical, research-authority, or external effect, or automatically
  resolves a disagreement into a single answer;
- remote/bounded-worker execution of lane runs (Spec 085 Compute) — every
  lane run in this spec is local-process only, exactly as Spec 077;
- real PHI fixtures or any real-PHI authorization claim;
- MESC coupling of any kind;
- wholesale adoption of Buzz's agent/workflow UI, relay protocol, or
  execution model; Buzz patterns may be selectively studied for shape only,
  never imported as running code.

## 6. Compatibility and reliance

- Pre-078 CLI/Desktop behavior is preserved; pre-078 vaults migrate without
  identity rewrite.
- Spec 077's `AgentIdentity`/`AgentCapabilityManifest`/`ContextManifest`/
  `AgentRun`/`AgentTurn`/`ToolInvocation`/`ToolReceipt`/`RunReceipt`/
  `AgentProposal` are reused exactly as frozen; 078 adds no parallel agent
  identity, run, tool, receipt, or proposal system. Every lane's actual
  model execution happens through Spec 077's existing
  `MedAgent::execute_agent_run`, never a second execution path.
- The `medscale-pack` runtime (`PackRuntimeAdapter` trait, `FixtureRuntime`,
  `OnnxTokenClassifierRuntime`) is reused as-is; 078 adds no parallel
  model-loading/inference path and admits no new Pack.
- Existing `SessionRegistry`/Core `Capability` mechanism governs all
  authority; a lane's policy adds a data/tool-scope boundary on top of, and
  never wider than, Spec 077's own `AgentCapabilityManifest`/
  `ContextManifest` checks, and never substitutes for them.

## 7. Missing-domain answers (frozen at T078-01)

- **Identity:** an `AgentLane` has no patient-identity semantics; it is a
  policy wrapper around an existing `AgentIdentity`. No cross-lane entity
  merge is performed or implied.
- **Time:** 078 durable rows use monotonic `seq`/`revision` for ordering,
  not wall-clock time or `MedicalTime`, mirroring Spec 077's own answer.
- **Rights:** fleet/lane/comparison output inherits the classification/
  policy of the Project and lanes it runs against; 078 introduces no new
  rights/license model.
- **Privacy:** no new egress is introduced; lane policy, fleet task text,
  and comparison payloads never leave the local vault in 078. No
  classification-inheritance behavior is implemented in v1: no
  classification primitive exists yet in this repository (Spec 079's
  future `DataClass`), so there is nothing for a `ComparisonReport` to
  inherit; see the T078-01 amendment to scenario 7 and `contracts.md`
  section 4.
- **Provenance:** every `FleetRun` binds exact lane identities/policies and
  every `LaneRunRef` to the real underlying `AgentRun`/`RunReceipt` it
  drove; a `ComparisonReport` binds exact source `AgentProposal`/
  `RunReceipt` references for every observation it records.
- **Failure:** denied, unavailable, stale, missing, cancelled, and failed
  states are distinct typed states end to end at both the lane level (reuse
  of `AgentRunState`) and the fleet level (`FleetRunState`, including
  explicit partial-failure), reusing `AuthorityError` and
  `ReferenceResolution` rather than inventing a parallel taxonomy.
- **Recovery:** restart, migration, backup, rollback via pre-migration
  backup restore, and interrupted-fleet fail-closed rules are defined in
  `migration.md` and tested.

## 8. Acceptance criteria

The frozen acceptance requirements are listed in
`docs/planning/SPEC_078_PROMOTION.md` ("Frozen acceptance requirements")
and mapped to proof in `plan.md` and the `evidence/078-model-fleet-compare/`
packet. They are not restated here to avoid divergence; the promotion
document is authoritative for acceptance wording.
