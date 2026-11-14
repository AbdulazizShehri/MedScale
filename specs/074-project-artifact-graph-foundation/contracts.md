# Contracts — Spec 074 Project + Artifact Graph Foundation

**Status:** `CONTRACT_FREEZE_INPUT`  
**Rule:** Muse must reconcile these semantics against the exact live Rust types during T074-01, make only compatibility-minimizing adjustments, then mark the resulting field layout `FROZEN_FOR_074` before storage implementation begins.

This file closes semantic ambiguity. It does not authorize creation of a parallel ID, provenance, audit, privacy or authorization system.

## 1. Existing primitives are authoritative

Reuse the current MedScale primitives wherever their semantics fit:

```text
OpaqueId
ObjectHeader
DigestSha256
RealmId
AuthorityScopeId
existing audit/evidence/provenance contracts
existing serialization/error conventions
```

Do not introduce `ProjectId`, `ExperimentId`, `ArtifactId`, `GraphId` wrappers unless live repository conventions clearly require newtype safety and the active spec records why `OpaqueId` alone is insufficient. If newtypes are used, they must wrap/reuse `OpaqueId`; they must not create a second identity namespace.

System/audit timestamps must use the repository's existing system-time/audit convention. Do **not** reuse `MedicalTime` for repository metadata merely because it exists; `MedicalTime` retains its clinical/time-precision semantics.

## 2. Revision model

Project-graph mutable state requires one explicit monotonic concurrency token.

Default semantic type:

```text
Revision = u64
```

Rules:

- create starts at revision `1` unless current repository conventions dictate another deterministic initial value;
- every successful mutation increments by exactly one;
- mutation of existing state supplies `expected_revision`;
- mismatch returns `Conflict` and does not write;
- archive/restore is a normal revisioned mutation;
- revision is concurrency state, not content identity and not `DigestSha256`;
- a digest may additionally bind referenced immutable bytes where the owning artifact exposes one.

If the live repository already has a canonical revision/precondition contract, reuse it and document the mapping rather than creating `Revision` anew.

## 3. Project

Semantic contract:

```text
Project {
    header: ObjectHeader,
    revision: Revision,
    name: String,
    description: Option<String>,
    status: ProjectStatus,
}

ProjectStatus = Active | Archived
```

Invariants:

- `name` is non-empty after trimming;
- field byte/character limits are explicit constants and tested;
- description is bounded;
- Project header scope must match the actor/context scope admitted by Core;
- archived Project remains inspectable;
- archive never cascades to referenced canonical artifacts;
- restore, if retained after T074-01 live-code review, is explicit and revision-guarded.

No owner/team/member field is added in 074. Multi-user membership belongs to later collaboration/Hub work.

## 4. Experiment

Semantic contract:

```text
Experiment {
    header: ObjectHeader,
    project_id: OpaqueId,
    revision: Revision,
    name: String,
    description: Option<String>,
    status: ExperimentStatus,
}

ExperimentStatus = Draft | Active | Completed | Archived
```

T074-01 may reduce this lifecycle if current product needs only a smaller safe set. It must not add extra lifecycle states after storage is implemented without revisiting the contract freeze.

Invariants:

- Project must exist in the same admitted realm/authority scope;
- Project archive policy is checked before Experiment mutation;
- moving an Experiment between Projects is **not** authorized in 074; create a new Experiment/reference arrangement instead if needed;
- Experiment archive does not delete its referenced artifacts.

## 5. ArtifactKind

`ArtifactKind` identifies the owning domain contract. It is not an alternate `ObjectClass` and must not flatten existing types.

T074-01 must derive the smallest vocabulary from currently addressable MedScale object families. The semantic shape should support at least the current/future distinction between:

```text
Patient/record-related canonical object
Source/document
Evidence/evaluation
Model/Pack/run-related object
Dataset/derived artifact when such a canonical object exists
OtherExplicit(<versioned bounded identifier>) only if repository compatibility needs it
```

Prefer mapping to an existing `ObjectClass`/typed contract where possible. Never infer authority from display strings.

Unknown authority-bearing kinds fail closed. UI may display an unsupported/missing reference without silently treating it as a known kind.

## 6. Artifact version binding

A Project reference points to an owning artifact identity plus the strongest version binding the owning contract actually supports.

Semantic model:

```text
ArtifactVersionBinding =
    IdentityOnly
  | Digest(DigestSha256)
  | Revision(StringOrExistingTypedRevision)
  | DigestAndRevision { ... }
```

This is a semantic requirement, not permission to invent a generic string revision if a stronger existing typed binding exists.

Rules:

- identity is mandatory;
- binding cannot claim immutability the owner does not provide;
- a later change to the underlying object does not silently rewrite a pinned binding;
- `IdentityOnly` is allowed only when the owner genuinely lacks a stronger stable version identity;
- resolution reports current/pinned/missing/stale explicitly.

## 7. ArtifactDescriptor

Semantic contract:

```text
ArtifactDescriptor {
    object_id: OpaqueId,
    kind: ArtifactKind,
    binding: ArtifactVersionBinding,
}
```

Optional safe display metadata may be returned by queries, but durable attachment authority must not depend on copied title/body/patient/model payload.

## 8. ProjectArtifactRef

Semantic contract:

```text
ProjectArtifactRef {
    header_or_id: existing durable identity form,
    project_id: OpaqueId,
    experiment_id: Option<OpaqueId>,
    artifact: ArtifactDescriptor,
    revision: Revision,
    status: Active | Detached,
}
```

Invariants:

- Project exists and scope matches;
- optional Experiment belongs to the same Project;
- target canonical object resolves successfully at attach time;
- duplicate active attachment under the same Project/Experiment/target/binding is idempotent or explicitly conflicts according to the frozen Core API; it never creates silent duplicates;
- detach tombstones/removes the relationship only; target artifact remains untouched;
- if the target disappears after valid attachment, the reference remains inspectable as `Missing`/`Stale` rather than rebinding.

## 9. Project Graph predicates

Spec 074 owns **organizational/workflow relationships only**. It must not duplicate clinical truth, provenance authority, evidence validation or model inference.

Initial bounded predicate vocabulary:

```text
Contains
References
AssociatedWith
ExperimentInput
ExperimentOutput
```

Interpretation:

- `Contains`: organizational containment inside a Project/Experiment view; not storage ownership.
- `References`: explicit user/Core reference with no stronger semantic claim.
- `AssociatedWith`: explicit organizational association; deliberately weak semantics.
- `ExperimentInput`: artifact is selected as an input for the Experiment; does not claim scientific causality/provenance beyond that recorded workflow role.
- `ExperimentOutput`: artifact is selected as an output/result reference for the Experiment; does not supersede the artifact's canonical provenance.

Do **not** add predicates such as `Proves`, `Diagnoses`, `ClinicallySupports`, `TruthOf`, or model-inferred relationship labels in 074.

Do not add `DerivedFrom` if it would duplicate an existing canonical provenance relation. If the live code shows no equivalent and an actual 074 workflow cannot be expressed without it, record a spec amendment explaining ownership and validation before adding it.

## 10. ProjectGraphEdge

Semantic contract:

```text
ProjectGraphEdge {
    id: OpaqueId,
    project_id: OpaqueId,
    subject: ArtifactDescriptorOrExperimentRef,
    predicate: ProjectGraphPredicate,
    object: ArtifactDescriptorOrExperimentRef,
    revision: Revision,
    status: Active | Removed,
}
```

T074-01 must choose a single typed endpoint representation rather than a JSON/string union.

Invariants:

- both endpoints are resolvable/admitted within the same Project context;
- no edge may imply cross-realm/cross-authority visibility;
- no self-edge unless the predicate explicitly allows it (default deny);
- duplicate active edge identity/semantic tuple is deterministic/idempotent or conflicts;
- removal does not delete endpoints;
- cycle presence is not inherently invalid for weak organizational relationships, but graph traversal is always bounded and non-recursive by default;
- graph queries never become an implicit recursive authority expansion.

## 11. Resolution state

Reference/query APIs need a typed resolution state:

```text
ReferenceResolution =
    Current
  | Stale
  | Missing
  | UnsupportedKind
  | Denied
  | Corrupt
```

`Denied` must not reveal sensitive target metadata. The UI can show that a reference cannot be resolved without exposing why beyond the permitted boundary.

## 12. ProjectContext

`ProjectContext` is a bounded, inspectable manifest for future consumers; it is **not** an ambient vault handle.

Semantic shape:

```text
ProjectContext {
    project: ProjectSummary,
    experiment: Option<ExperimentSummary>,
    authorized_artifact_refs: Vec<ResolvedArtifactRef>,
    graph_slice: Vec<ProjectGraphEdge>,
    cursor/page metadata,
}
```

Rules:

- query is paginated/bounded;
- every returned ref has passed existing authorization/scope checks;
- no raw artifact payload is included unless a separate typed query explicitly asks for and authorizes it;
- no future MedAgent assumption is implemented here; 074 only proves a safe context manifest can be resolved.

## 13. ProjectSummary

Summary may contain only non-sensitive Project metadata plus counts computed after authorization filtering.

Never expose a count that proves the existence of inaccessible protected artifacts if current policy treats existence itself as sensitive.

## 14. Error contract

Use the current repository error/result framework if one exists. Semantic distinctions required by 074:

```text
Invalid
NotFound
Denied
Conflict
StaleReference
Corrupt
UnsupportedSchema
Unavailable
Internal
```

The exact Rust enum naming may follow existing conventions, but these states must not be collapsed where they affect user action or safety.

## 15. Serialization/versioning

- follow current serde naming/`deny_unknown_fields` policy for authority-bearing contracts;
- every durable schema has explicit schema version ownership;
- unknown future authority values fail closed;
- old supported versions migrate or return explicit unsupported-schema state;
- JSON CLI output is versionable and tested.

## 16. Contract freeze gate

Before T074-02 starts, Muse must update this file with:

```text
CONTRACT_FREEZE = FROZEN_FOR_074
LIVE_BASE_SHA = <verified canonical ancestor>
CONTRACT_FILES = <exact Rust paths>
REVISION_MODEL = <exact reused/new type>
PREDICATE_VOCABULARY = <final bounded set>
ARTIFACT_KIND_MAPPING = <exact current object-family mapping>
ERROR_TYPE = <exact Rust type/path>
```

Any later change to those frozen items requires an explicit reason, test impact review, and migration impact review before code proceeds.

## 17. Freeze record (T074-01, FROZEN_FOR_074)

```text
CONTRACT_FREEZE = FROZEN_FOR_074
LIVE_BASE_SHA = ee8daef3a2782bbdbcb3324766a5d6b95c09fa09
CONTRACT_FILES = crates/medscale-contracts/src/project_graph.rs,
                 crates/medscale-contracts/src/lib.rs,
                 crates/medscale-contracts/src/envelopes/mod.rs
REVISION_MODEL = medscale_contracts::project_graph::ProjectRevision (u64 alias)
                 + check_revision/initial_revision; no canonical precondition
                 contract existed, so this is the single definition for 074.
PREDICATE_VOCABULARY = contains, references, associated_with,
                       experiment_input, experiment_output
ARTIFACT_KIND_MAPPING = SourceRecord, DerivedSourceArtifact, Proposal,
                        ClinicalAssertion, EvaluationRecord, IdentityAssertion,
                        AmendmentRecord, EvidenceDocument, PackManifest,
                        OtherExplicit(<[a-z0-9._-]{1,128}>);
                        ActionAuditRecord/Projection/IdentityMergeDecision are
                        intentionally NOT attachable (stay in owning trails).
ERROR_TYPE = medscale_contracts::envelopes::AuthorityError, extended additively
             with Conflict/StaleReference/Corrupt/UnsupportedSchema/Unavailable/
             Internal { message }; Invalid->InvalidArgument, NotFound->NotFound,
             Denied->Unauthorized|SessionDenied|WrongScope (most precise applies).
```

Sequencing note: typed Core request/response envelope bodies and their
`capability_matches` pairs land in 074-C with the authority paths so the
facade dispatch match stays exhaustive and every commit builds. The 12 new
`Capability` variants and the 6 new `AuthorityError` variants above are the
frozen vocabulary 074-C must use; 074-C may not invent parallel capabilities
or a second error taxonomy. Experiment restore is NOT admitted (archive is
terminal for experiments). No `ProjectId`/`ExperimentId` newtypes: identity
is `header.id: OpaqueId`. No durable timestamps in 074 contracts: ordering
travels through the existing audit trail; `MedicalTime` keeps clinical
semantics.