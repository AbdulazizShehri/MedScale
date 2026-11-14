# Spec 074 — Project + Artifact Graph Foundation

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`  
**Promoted:** 2026-09-17  
**Base SHA:** `a80c33307afc4577790282652e5b20911beb4bbe`  
**Target branch:** `spec/074-project-artifact-graph-foundation`  
**Dependency:** canonical MedScale through Spec 073 + merged Research OS planning packet PR #121  
**Promotion authority:** `docs/planning/SPEC_074_PROMOTION.md`

## 1. Problem

MedScale has authoritative patients, source/evidence objects, model Packs, workflows and product surfaces, but it lacks one durable organizing substrate for research work. Future MedAgent, Analytics, AudioFlow, collaboration, RAG and lab-scale workflows need a stable Project/Experiment/artifact relationship model. Building those features first would create feature-specific folders, IDs, context stores or duplicated objects.

## 2. Goal

Add a local-first **Project + Artifact Graph Foundation** that organizes existing and future MedScale artifacts without replacing their canonical authority, IDs, provenance or storage ownership.

At closure, a user can create a Project, create an Experiment, attach references to existing MedScale artifacts, create/remove typed organizational relationships, close/reopen the app, inspect the same graph through Core/CLI/Desktop, archive the Project, and prove that existing pre-074 workflows still behave correctly.

## 3. User and operational scenarios

1. A researcher creates an offline Project and gives it a name/description.
2. The researcher creates an Experiment under that Project.
3. Existing MedScale objects are attached by stable identity without copying payloads.
4. The user relates artifacts with explicit typed predicates such as `contains`, `derived_from`, `supports`, `discusses` or another bounded vocabulary approved during 074-A.
5. The application restarts and the Project/Experiment/edges resolve identically.
6. A stale mutation using an outdated revision/precondition returns an explicit conflict rather than overwriting newer state.
7. An existing vault created before Spec 074 opens and migrates without changing old object IDs or breaking old CLI/Desktop workflows.
8. A missing/deleted referenced object is displayed as a missing/stale reference; the graph does not fabricate payload or silently rebind another object.
9. A user archives a Project; its referenced canonical objects are not deleted.
10. An interrupted/crashed write does not leave a half-attached object/edge state.
11. A larger synthetic lab fixture remains usable and inspectable under declared performance measurements.

## 4. Required outcomes

1. Introduce bounded contracts for Project, Experiment, artifact descriptor/reference, typed graph edge, project context and summaries.
2. Reuse existing `OpaqueId`, `ObjectHeader`, digest/provenance/audit concepts where their semantics fit.
3. Preserve existing canonical domain objects; Project organization references them and never copies authority payload merely to join a Project.
4. Persist Project metadata, Experiment metadata, artifact references and graph edges inside the existing encrypted vault/storage architecture.
5. Provide explicit optimistic-concurrency/precondition semantics for mutable Project/Experiment/reference/edge state.
6. Provide transactional Core mutation paths so multi-record operations cannot partially commit authority.
7. Provide deterministic Core query paths for Project list/get/context and graph-neighbor traversal.
8. Provide a CLI vertical slice that uses Core and exposes stable machine-readable output plus honest error states.
9. Provide a native Slint Desktop Projects surface with create/open/archive, active Project context, Experiment view and artifact relationship inspection.
10. Preserve pre-074 CLI/Desktop behavior and current authority boundaries.
11. Provide migration/reopen/backup-recovery evidence from representative pre-074 vault fixtures.
12. Provide scale measurements without making unsupported performance claims.
13. Provide exact-head closure evidence mapping every acceptance criterion to proof.

## 5. Explicit non-goals

Spec 074 MUST NOT implement:

- team invitations, messages, shared rooms, sync or Hub;
- MedAgent or model Fleet;
- new PII/PHI de-identification engine;
- web browsing;
- AudioFlow;
- Analytics/DataFusion;
- vector search/RAG/Research Canvas;
- remote Compute;
- Research Packs;
- federation;
- real-PHI use;
- MESC;
- production cloud services;
- arbitrary plugin code;
- wholesale donor source adoption.

## 6. Existing-system compatibility

Implementation must inspect and preserve current live semantics for:

- `crates/medscale-contracts` object IDs/header/digest/evidence/audit types;
- `crates/medscale-storage` encrypted vault, SQLite metadata, blobs, migrations, backup/recovery and writer locking;
- `crates/medscale-core` authority/session/effect/query patterns;
- existing FHIR/patient/document/evidence/model/Pack identities;
- `medscale-cli` command behavior and exit/error conventions;
- `medscale-desktop` native Slint design/accessibility and existing routes.

No existing object is assigned a new identity just because it is attached to a Project.

## 7. Authority invariants

1. MedScale Core remains the sole product authority orchestrator.
2. A Project is an organizing authority object; it does not become the owner of clinical truth stored elsewhere.
3. Artifact references bind stable canonical object IDs and, when applicable, exact revision/digest metadata. They do not embed copied canonical payload by default.
4. Project Graph edges are explicit user/Core-created organizational relationships only.
5. Inferred/search/vector/model-produced relationships remain proposals/projections/evidence until a later authorized spec defines admission.
6. Archive of Project/Experiment/reference/edge never implicitly deletes referenced canonical objects.
7. No UI or CLI path writes storage directly.

## 8. Identity / authorization / capability rules

Spec 074 uses current actor/session/authority scope semantics. It MUST NOT invent team ReBAC or institutional roles.

Minimum capability operations to represent through the existing Core authority pattern:

```text
project.create
project.read
project.update
project.archive
experiment.create
experiment.read
experiment.update
experiment.archive
project_artifact.attach
project_artifact.detach
project_graph.read
project_graph.mutate
```

If the current repository has a different canonical capability vocabulary, 074 must map to it rather than introduce a parallel authorization engine.

## 9. Data classification / privacy rules

Spec 074 introduces no new global privacy taxonomy ahead of Spec 078.

- referenced artifacts retain their existing privacy/authority semantics;
- Project metadata must not downgrade or reclassify referenced content;
- Project summaries/counts must not leak content from objects the actor cannot inspect;
- logs contain IDs/statuses rather than sensitive payload by default;
- no network egress is added;
- no real PHI becomes authorized.

## 10. Contracts and types

Default owning location: `medscale-contracts`, using domain modules only when promoted implementation proves they fit dependency direction.

Required semantic contracts:

```text
Project
ProjectStatus
Experiment
ExperimentStatus
ArtifactDescriptor
ArtifactKind
ProjectArtifactRef
ProjectGraphEdge
ProjectGraphPredicate
ProjectContext
ProjectSummary
GraphNeighborQuery
GraphNeighborPage/Result
```

Minimum rules:

- IDs are existing `OpaqueId` values, not hashes;
- Project/Experiment durable headers reuse `ObjectHeader` or the nearest current canonical equivalent;
- mutable entities expose an explicit monotonic revision/version used for precondition checks;
- names/descriptions are bounded and validated;
- artifact descriptors identify the owning object class/kind without pretending a generic descriptor replaces the original contract;
- edges include stable edge identity, project scope, subject ref, predicate, object ref and revision/version metadata;
- unknown serialized enum/contract fields follow current repository strictness policy; do not silently accept unsafe unknown authority values.

The exact Rust field layout must be frozen in `contracts.md` after live code inspection and before 074-B storage implementation.

## 11. State machines

### Project

```text
ACTIVE -> ARCHIVED
ARCHIVED -> ACTIVE   (explicit restore only, if implementation proves current UX/authority supports it)
```

No destructive-delete state is authorized in 074.

### Experiment

Minimum allowed lifecycle:

```text
DRAFT -> ACTIVE -> COMPLETED -> ARCHIVED
DRAFT/ACTIVE/COMPLETED -> ARCHIVED
```

If a smaller lifecycle is sufficient, 074-A may minimize it before code; it must not silently expand after storage ships.

### Reference/edge

Creation is explicit. Removal is tombstone/detach semantics; no cascade into target canonical objects.

## 12. Core commands / queries

Required semantic operations:

```text
ProjectCreate
ProjectGet
ProjectList
ProjectUpdateMetadata
ProjectArchive
ProjectRestore          (only if admitted by frozen lifecycle)
ExperimentCreate
ExperimentGet/List
ExperimentUpdate
ExperimentArchive
ArtifactAttach
ArtifactDetachReference
GraphEdgeCreate
GraphEdgeRemove
GraphNeighborsQuery
ProjectContextResolve
ProjectSummaryQuery
```

Mutation requests must include actor/session/scope and expected revision/precondition where state already exists. Duplicate/idempotent requests must have deterministic behavior. Stale preconditions return explicit conflict.

Required result/error distinctions where applicable:

```text
Ok
NotFound
Invalid
Denied
Conflict
StaleReference
Corrupt
Unavailable
Internal
```

Do not collapse authorization/conflict/corruption into a generic error string.

## 13. Storage schema / indexes / transactions

Default schema families inside current encrypted metadata/storage architecture:

```text
projects
experiments
project_artifact_refs
project_graph_edges
```

Each table/record family must define:

- stable ID mapping;
- project/realm/authority scope;
- schema version;
- mutable revision/version;
- status/tombstone state;
- bounded metadata;
- indexes for Project list, Project artifact listing, graph subject/predicate/object queries and Experiment listing;
- uniqueness constraints preventing duplicate active attachment/edge identities where semantics require it.

Transaction rules:

1. Create/update of one durable entity is atomic.
2. Attach and any required graph/index metadata commit together or not at all.
3. Edge creation validates subject/object references and Project scope before commit.
4. Crash before commit yields no durable mutation; crash after commit yields fully reopenable state.
5. No foreign-key/cascade rule may delete canonical target artifacts.

The implementation must use existing writer lock/transaction patterns unless live evidence proves a change is required.

## 14. Migration and rollback

Migration must support:

```text
pre-074 vault
 -> backup/recovery checkpoint
 -> forward migration
 -> schema validation
 -> old workflow regression
 -> new 074 workflow
 -> close/reopen
 -> exact ID/revision/edge verification
 -> repeated migration/open safety
```

Pre-074 objects are not automatically stuffed into a synthetic Project unless an explicit deterministic migration rule is justified. Default: existing objects remain valid outside Projects; Project attachment is opt-in.

If safe down-migration cannot preserve new state, rollback is restore-from-pre-migration backup. Do not pretend destructive down-migration is safe.

## 15. Network / external effects

`N/A` for new runtime effects. Spec 074 must work fully offline and adds no product runtime destination.

Development dependency/source retrieval remains governed by existing repository rules.

## 16. Worker / sandbox behavior

`N/A`. No new worker or arbitrary-code path is authorized.

## 17. Source/dependency/donor decisions

No donor is required for 074. Prefer current Rust/std/workspace dependencies.

Any proposed new dependency requires:

- exact crate/version/revision;
- reason existing workspace cannot satisfy the need;
- license/security/supply-chain review;
- dependency-direction check;
- removal/upgrade plan.

Buzz, Qdrat, Huly, Plane and other collaboration/project sources are references only in 074 unless an exact small component is separately qualified. Collaboration behavior belongs to 075.

## 18. CLI behavior

Minimum command family, adjusted to current CLI conventions:

```text
medscale project create
medscale project list
medscale project show
medscale project update
medscale project archive
medscale project context
medscale project attach
medscale project detach
medscale project graph neighbors
medscale experiment create
medscale experiment list
medscale experiment show
medscale experiment update/archive
```

Requirements:

- human and JSON outputs derive from typed Core results;
- IDs/revisions/status are inspectable;
- denied/conflict/not-found/corrupt states use current stable exit semantics;
- no CLI-only policy or storage path.

The first CLI slice must also evaluate whether the existing monolithic `medscale-cli/src/main.rs` needs a behavior-preserving command-module refactor; refactor only when required for bounded maintainability.

## 19. Desktop behavior

Add a native **Projects** workspace consistent with the current Slint design authority.

Minimum UI:

- Projects list/search/filter sufficient for local fixture scale;
- create Project;
- open Project;
- visible active Project context;
- edit bounded metadata;
- archive/restore if admitted;
- Experiment list/create/open;
- artifact-reference list with kind, stable identity and missing/stale state;
- relationship inspector/list/tree using real graph data;
- empty/loading/denied/conflict/corrupt/unavailable states;
- keyboard navigation/focus/accessible names/actions.

A decorative node graph is explicitly not required for 074 closure.

## 20. Failure and recovery table

| Condition | Required behavior |
|---|---|
| Invalid name/metadata/predicate | `Invalid`; no write |
| Unknown actor/session/scope | existing denial/session error; no write |
| Unauthorized mutation | `Denied`; no write |
| Missing Project/Experiment | `NotFound` |
| Stale expected revision | `Conflict`; preserve newer state |
| Missing referenced canonical object | reject attach, or expose an explicit stale/missing ref only if it became missing after prior valid attachment |
| Duplicate attach/edge request | deterministic idempotent result or explicit conflict; never duplicate silently |
| Cross-project edge violation | `Invalid`/`Denied`; no write |
| Crash before commit | no durable mutation |
| Crash after commit | full mutation survives reopen |
| Corrupt Project rows/refs | explicit `Corrupt`; do not fabricate object |
| Unsupported schema version | fail closed with migration/compatibility error |
| Archived Project mutation | deny unless operation is explicitly permitted by lifecycle |

## 21. Security / threat delta

New attack surfaces:

- graph/reference injection;
- ID spoofing/cross-scope reference;
- stale-write overwrite;
- metadata-based disclosure;
- pathological graph traversal/resource exhaustion;
- migration corruption;
- UI/CLI mismatch causing authority bypass.

Required mitigations/tests:

- validate all referenced IDs through Core and correct authority scope;
- parameterized/typed storage access; no SQL string construction from user metadata;
- bounded metadata and query page/limit controls;
- explicit traversal limits; 074 graph query must not recurse unbounded by default;
- optimistic concurrency/preconditions;
- access checks before summaries/counts reveal protected references;
- migration/recovery and corrupt-record tests.

## 22. Performance / resource envelope

No marketing budget is claimed by 074. Measure and record behavior on declared CI/development hardware.

Minimum synthetic fixtures:

```text
Personal: >= 10 Projects and >= 1,000 total artifact references.
Lab: >= 100 Projects OR one Project with >= 100,000 artifact/edge relations.
```

Measure at least:

- Project list/open latency;
- attachment/edge mutation latency;
- graph neighbor query latency with bounded page size;
- reopen/migration time;
- storage growth.

A measurement is evidence, not a universal performance claim.

## 23. Test plan

Apply `RESEARCH_OS_VERIFICATION_MATRIX.md` L0-L8 as applicable; L9 only if a new external dependency is actually admitted.

Minimum focused suites:

- contract serialization/strictness/invariants;
- storage migration/reopen/transaction/crash semantics;
- Core create/update/archive/attach/detach/edge/query authority;
- conflict/idempotency/cross-scope/adversarial reference tests;
- CLI integration/JSON/exit-state tests;
- Desktop state/accessibility adapter tests;
- pre-074 regression suite;
- personal + lab synthetic scale fixtures;
- full repository required gates.

Before closure, run the live repository equivalents of at least:

```text
cargo fmt --all -- --check
./scripts/check-dependency-direction.ps1
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo deny check --all-features
```

Plus all live required CI jobs. Do not rely on this list if main CI has changed.

## 24. Evidence plan

Evidence root:

```text
evidence/074-project-artifact-graph-foundation/
```

Required closure artifacts:

- base/live-truth record;
- contract/storage/Core/CLI/Desktop focused test logs;
- migration/reopen/recovery evidence;
- scale measurement report;
- accessibility/UX state evidence appropriate to current UI qualification practice;
- exact-range diff review;
- exact-head CI run IDs/results;
- final closure statement;
- post-merge main verification.

## 25. Implementation slices

```text
074-A Contracts
  Freeze field-level contracts, predicates, lifecycle, validation, errors and serialization tests.

074-B Storage + migration
  Add encrypted metadata schema/migration, transactions, indexes, reopen/recovery tests.

074-C Core authority
  Implement typed commands/queries, reference validation, concurrency/preconditions, graph limits and receipts/audit integration.

074-D CLI
  Add project/experiment/artifact/graph command vertical slice through Core; preserve existing command behavior.

074-E Desktop
  Add native Projects workspace through Core with real states and accessibility.

074-F Qualification + closure
  Run compatibility/adversarial/scale/recovery/full CI, exact-range review, PR merge and post-main verification.
```

## 26. Task rules

`tasks.md` is the execution cursor. A task is complete only when its code/docs/tests/evidence are actually present. Do not mark a later slice complete to hide an earlier missing invariant.

## 27. Completion criteria

Spec 074 may become `CLOSED_CANONICAL` only when:

1. all required outcomes are implemented on the exact reviewed head;
2. no unauthorized 075+ capability is present;
3. existing object IDs/authority semantics remain preserved;
4. migration/reopen/recovery evidence passes;
5. Core/CLI/Desktop parity is proven;
6. required security/conflict/cross-scope tests pass;
7. scale evidence is recorded honestly;
8. all live required CI passes on exact head;
9. PR is merged normally without bypass/force/rebase;
10. post-merge main verification is green and recorded;
11. canonical queue/status is updated to close 074 and recompute the next eligible unit.

## 28. Completion truth

After closure, MedScale may claim that it has a durable local Project/Experiment/Artifact Graph organizing foundation for the qualified scope.

It may **not** claim team collaboration, MedAgent, Fleet, Privacy Gate expansion, Browse, AudioFlow, Analytics, RAG, Hub, Compute, Research Packs, federation, real-PHI readiness or Research OS completion merely because 074 is closed.