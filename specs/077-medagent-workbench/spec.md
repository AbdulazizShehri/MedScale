# Spec 077 — MedAgent Workbench

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promoted:** 2026-09-21
**Base SHA:** `80aefaecc513b504b89a8a6cf06b2a0bbd5e49ea`
**Target branch:** `spec/077-medagent-workbench`
**Dependency:** canonical MedScale through Spec 076 (`CLOSED_CANONICAL`)
**Promotion authority:** `docs/planning/SPEC_077_PROMOTION.md`

## 1. Problem

MedScale can organize research into Projects (074), govern data acquisition
(075), and let people collaborate around Project artifacts (076), but every
research workflow that wants a local model to read selected context, call a
bounded set of tools, and produce a reviewable draft would otherwise invent
its own ad hoc "run a model" glue: no typed context boundary, no receipted
tool calls, no cancellable run lifecycle, no structural guarantee that model
output stays evidence-only. That produces silent ambient data access,
untraceable tool side effects, and, worst case, a second authority plane
where a model's output is trusted as fact instead of proposal.

## 2. Goal

Add a local-first **MedAgent Workbench**: a governed, project-aware agent
IDE where an admitted local model Pack (from the existing
`medscale-pack`/Spec 008/069 runtime) reads only an explicit
`ContextManifest` of selected Project artifacts, may request a bounded set
of typed, Core-executed, receipted tools, runs inside a persisted,
cancellable state machine, and produces output that is structurally
evidence-only (`Proposal`-class, never `ClinicalAssertion`) — all built by
reusing Spec 074's Project/artifact/revision primitives, Spec 002's
`Proposal`/`ProducerKind`, and Spec 076's `ParticipantIdentity`, rather than
inventing a parallel authority, execution, or identity foundation.

At closure, a user can open a Project, register an `AgentIdentity` bound to
one admitted local model Pack, select explicit context artifacts into a
`ContextManifest`, submit a text prompt, watch the run execute through
typed tool invocations with visible receipts, cancel a running turn if
needed, see the completed run's `AgentProposal` and full `RunReceipt`
provenance, close/reopen MedScale, and see every fact resolve identically —
with zero network dependency and zero implicit clinical/research authority.

## 3. User and operational scenarios

1. An operator registers an `AgentIdentity` for one admitted local model
   Pack with an explicit `AgentCapabilityManifest` (which tool kinds it may
   request, nothing else); the identity persists across restart.
2. A researcher builds a `ContextManifest` by explicitly selecting Project
   artifacts (e.g., specific 075 snapshots, 074 documents); the agent run
   can read exactly those artifact revisions and nothing else in the vault.
3. A researcher submits a text prompt against that context; the run
   transitions `Pending -> Running`, issues zero or more typed
   `ToolInvocation`s (each policy-checked and Core-executed, never
   model-authored code), and transitions to `Completed` with a persisted
   `AgentProposal` and `RunReceipt`.
4. A researcher cancels a running turn; the run stops issuing further tool
   invocations and transitions to `Cancelled`, never silently continuing
   in the background.
5. A run's `RunReceipt` shows exact model Pack identity/version, exact
   `ContextManifest` revision, every tool invocation and its receipt, and
   resource/timing facts — inspectable after the fact, not just live.
6. A tool invocation that the current `AgentCapabilityManifest` does not
   grant is refused before execution, with the refusal visible in the run's
   history, not silently dropped.
7. An `AgentRun`'s `AgentProposal` output is never automatically promoted
   to `ClinicalAssertion`, and no code path in this spec calls
   `PromoteProposal`, `TransitionEffect`, or any controlled-action
   capability from agent output.
8. An agent participant registered in a Spec 076 collaboration Room
   (`ParticipantKind::Agent`) can have its `agent_profile_ref` resolved to
   a real `AgentIdentity`, so its Room activity is attributable to a real,
   inspectable agent, not an opaque placeholder.
9. The application restarts; every agent identity/context manifest/run/
   turn/tool invocation/receipt/proposal record resolves identically.
10. A vault created before Spec 077 opens and migrates (v5 -> v6) without
    changing old object IDs or breaking old CLI/Desktop workflows.
11. With network egress disabled, every 077 workflow remains fully useful;
    the one admitted local model Pack lane runs fully offline; there is no
    code path that requires network for any local-first agent operation.
12. A run that fails partway (model error, tool failure) is explicitly
    `Failed`, with whatever partial history exists still inspectable and
    never silently presented as a complete, successful run.

## 4. Required outcomes

1. Bounded contracts for `AgentIdentity`/`AgentProfile`/
   `AgentCapabilityManifest`, `ContextManifest`, `AgentRun`/
   `AgentRunState`/`AgentTurn`, `ToolManifest`/`ToolInvocation`/
   `ToolReceipt`, `RunReceipt`, `AgentProposal`.
2. Reuse of existing `OpaqueId`, `ObjectHeader`, `DigestSha256`, realm/
   scope, `ProjectRevision`/`check_revision`/`initial_revision`,
   `ArtifactDescriptor`/`ArtifactVersionBinding`/`ReferenceResolution`,
   `Proposal`/`ProducerKind`, and the existing `medscale-pack` runtime; no
   new ID, revision, provenance, or model-inference foundation.
3. Encrypted-vault persistence for all durable 077 rows inside the existing
   storage architecture (schema v5 -> v6, additive).
4. `AgentIdentity`/capability-manifest vertical slice: register an identity
   bound to one admitted local model Pack, reopen and see identical state.
5. `ContextManifest` vertical slice: explicit artifact selection bound to
   exact revisions; an agent run cannot read outside its named context.
6. `AgentRun` vertical slice: full state machine including cancel/
   interrupt, persisted across restart.
7. Tool vertical slice: typed `ToolManifest`/`ToolInvocation`/
   `ToolReceipt`, policy-checked and Core-executed only.
8. Model Pack lane integration: a genuine text-prompt-in/proposal-out run
   against the existing `medscale-pack` runtime, producing a real
   `AgentProposal` (`Proposal`-class, evidence-only).
9. `RunReceipt` + run history persisted as inspectable Project-scoped
   artifacts.
10. CLI inspect/mutation paths through Core with stable machine-readable
    output, matching Spec 074/075/076 conventions.
11. Native Slint Desktop split-pane workbench backed exclusively by Core
    for at minimum a run list/detail view, context selection, and cancel/
    interrupt controls, reusing the current design system.
12. Migration/reopen/backup-recovery evidence from representative pre-077
    vault fixtures (including populated 074/075/076 vaults).
13. Exact-head closure evidence mapping every acceptance criterion to
    proof.

## 5. Explicit non-goals

Spec 077 MUST NOT implement:

- Model Fleet + Compare, multiple simultaneous lanes, or any comparison
  reporting (Spec 078);
- Privacy Gate expansion, de-identification, or data-class policy beyond
  reusing existing classification (Spec 079);
- Governed Browse, web search, or any browser/external-provider tool for
  an agent (Spec 080) — external/browser providers remain denied;
- AudioFlow, Analytics Gate, Clinical Graph/Research Canvas, Hub, Compute,
  R Workspace, Community Extensions, Research/Evidence Packs,
  Institutional Adapters, Federation (Specs 081-092);
- a new general-purpose local-model inference engine; 077 uses the
  `medscale-pack` runtime as it already exists (`FixtureRuntime`/
  `OnnxTokenClassifierRuntime` or whatever admitted runtime the qualified
  Pack targets) rather than building a second inference stack;
- multi-agent orchestration or agent-to-agent delegation;
- agent self-registration or self-escalation: an `AgentIdentity` is created
  only by an explicit human-initiated Core command;
- any mechanism by which agent output automatically performs a clinical,
  research-authority, or external effect;
- a second database, vault, ID namespace, revision model, or authority
  plane;
- remote/bounded-worker execution of agent runs (Spec 085 Compute);
- real PHI fixtures or any real-PHI authorization claim;
- MESC coupling of any kind;
- wholesale adoption of Buzz's agent/workflow UI, relay protocol, or
  execution model; Buzz patterns may be selectively studied for shape
  only, never imported as running code.

## 6. Compatibility and reliance

- Pre-077 CLI/Desktop behavior is preserved; pre-077 vaults migrate
  without identity rewrite.
- Spec 074 Project/artifact-ref APIs are reused for `ContextManifest`
  artifact selection; 077 adds no parallel project or artifact system.
- Spec 002's `Proposal`/`ProducerKind` is reused for `AgentProposal`
  output; 077 adds no parallel proposal/authority mechanism.
- Spec 076's `ParticipantIdentity`/`ParticipantKind::Agent`/
  `AgentParticipantIdentity.agent_profile_ref` is the integration point: an
  `AgentIdentity` this spec creates may be referenced from an existing
  Spec 076 agent participant, resolving the previously-opaque forward
  reference. 077 does not modify Spec 076's closed contracts; it only
  provides a real object for the reference to point at.
- The `medscale-pack` runtime (`PackRuntimeAdapter` trait,
  `FixtureRuntime`, `OnnxTokenClassifierRuntime`) is reused as the model
  execution surface; 077 adds no parallel model-loading/inference path.
- Existing `SessionRegistry`/Core `Capability` mechanism governs all
  authority; an `AgentRun`'s `ContextManifest` adds a data-visibility
  boundary on top and never substitutes for it.

## 7. Missing-domain answers (frozen at T077-01)

- **Identity:** an `AgentIdentity` has no patient-identity semantics; it is
  an execution-actor record bound to one admitted model Pack. No
  cross-agent entity merge is performed or implied.
- **Time:** 077 durable rows use monotonic `seq`/`revision` for ordering,
  not wall-clock time or `MedicalTime`; `MedicalTime` keeps its clinical/
  time-precision semantics and is not reused for repository metadata
  merely because it exists.
- **Rights:** agent output inherits the classification/policy of the
  Project it runs against; 077 introduces no new rights/license model.
- **Privacy:** no new egress is introduced; `ContextManifest` bodies,
  prompts, and proposal payloads never leave the local vault in 077;
  logging conventions bound free-text exposure, mirroring `security.md`
  T11 from Spec 076.
- **Provenance:** every run binds exact model Pack identity/version, exact
  `ContextManifest` revision, and every tool invocation/receipt in a
  `RunReceipt` committed atomically with the run's terminal state.
- **Failure:** denied, unavailable, stale, missing, cancelled, and failed
  states are distinct typed states end to end, reusing `AuthorityError` and
  `ReferenceResolution` rather than inventing a parallel taxonomy.
- **Recovery:** restart, migration, backup, rollback via pre-migration
  backup restore, and interrupted-run fail-closed rules are defined in
  `migration.md` and tested.

## 8. Acceptance criteria

The frozen acceptance requirements are listed in
`docs/planning/SPEC_077_PROMOTION.md` ("Frozen acceptance requirements")
and mapped to proof in `plan.md` and the `evidence/077-medagent-workbench/`
packet. They are not restated here to avoid divergence; the promotion
document is authoritative for acceptance wording.
