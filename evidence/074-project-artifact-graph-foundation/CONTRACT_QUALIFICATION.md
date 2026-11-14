# CONTRACT QUALIFICATION — Spec 074 (074-A, T074-01)

## Binding

```text
BASE_SHA=ee8daef3a2782bbdbcb3324766a5d6b95c09fa09
BRANCH=spec/074-project-artifact-graph-foundation
CANDIDATE_SCOPE=contracts only (no storage, no Core paths, no CLI, no Desktop)
HOST=Windows 11 x64, Rust 1.97.1, Strawberry Perl 5.42.3.1 portable (TEMP-only) + VS2022 BuildTools 14.44
REAL_PHI_USED=false
```

## Frozen contract files

```text
crates/medscale-contracts/src/project_graph.rs (new, ~1000 lines incl. 16 unit tests)
crates/medscale-contracts/src/lib.rs (module registration)
crates/medscale-contracts/src/envelopes/mod.rs (+12 Capability variants, +6 AuthorityError variants)
specs/074-project-artifact-graph-foundation/contracts.md (section 17 freeze record)
```

Freeze: `CONTRACT_FREEZE=FROZEN_FOR_074` (see contracts.md section 17).

## Type map (semantic -> Rust)

```text
Project                 -> project_graph::Project { header: ObjectHeader, revision, name, description?, status }
ProjectStatus           -> Active | Archived
Experiment              -> project_graph::Experiment { header, project_id: OpaqueId, revision, name, description?, status }
ExperimentStatus        -> Draft | Active | Completed | Archived (+ can_transition_to)
ArtifactDescriptor      -> { object_id: OpaqueId, kind: ArtifactKind, binding: ArtifactVersionBinding }
ArtifactKind            -> SourceRecord | DerivedSourceArtifact | Proposal | ClinicalAssertion
                           | EvaluationRecord | IdentityAssertion | AmendmentRecord
                           | EvidenceDocument | PackManifest | OtherExplicit(String)
ArtifactVersionBinding  -> IdentityOnly | Digest(DigestSha256) | Revision(String) | DigestAndRevision
ProjectArtifactRef      -> { header, project_id, experiment_id?, artifact, revision, status: Active|Detached }
ProjectGraphPredicate   -> Contains | References | AssociatedWith | ExperimentInput | ExperimentOutput
GraphEndpoint           -> Artifact(ArtifactDescriptor) | Experiment(OpaqueId)
ProjectGraphEdge        -> { header, project_id, subject, predicate, object, revision, status: Active|Removed }
ReferenceResolution     -> Current | Stale | Missing | UnsupportedKind | Denied | Corrupt
ProjectContext          -> { project, experiment?, authorized_artifact_refs, graph_slice, totals, cursors? }
ProjectSummary          -> { project_id, name, status, revision, experiment_count, active_ref_count, active_edge_count }
ExperimentSummary       -> { experiment_id, project_id, name, status, revision }
GraphNeighborQuery      -> { project_id, start, predicates?, direction?, limit?, cursor? } (single hop, edge-id order)
GraphNeighborPage       -> { edges, next_cursor? }
Revision                -> ProjectRevision = u64, initial 1, +1 per mutation, check_revision()
Errors                  -> envelopes::AuthorityError (+Conflict/StaleReference/Corrupt/
                           UnsupportedSchema/Unavailable/Internal { message })
Capabilities            -> ProjectCreate/Read/Update/Archive, ExperimentCreate/Read/Update/Archive,
                           ProjectArtifactAttach/Detach, ProjectGraphRead/Mutate
```

Reuse: `OpaqueId` (all ids incl. header.id; no ProjectId newtypes), `ObjectHeader`,
`DigestSha256`, `RealmId`, `AuthorityScopeId`. No `MedicalTime`. No durable timestamps.

## Invariant proof (cargo test -p medscale-contracts)

```text
RESULT=PASS (38 lib tests incl. 16 project_graph tests; 0 failed)
- revision starts at 1, increments by exactly one, stale conflicts without write
- name/description bounds enforced (128 / 2048 chars)
- experiment lifecycle admits only 074 transitions; archive terminal (no restore)
- predicate vocabulary closed: proves/diagnoses/derived_from/empty/uppercase rejected
- OtherExplicit cannot shadow known kinds; uppercase rejected
- binding revision bounds enforced
- self-edges denied
- duplicate attachment/edge identity deterministic (same_attachment_as/same_edge_as)
- neighbor query bounds (default 25, max 100, cursor 256B); limit 0 => default
- context page bounds (100 refs / 100 edges)
- deny_unknown_fields rejects injected fields on Project
- predicate snake_case round-trip
- scope match exact
```

## Workspace safety proof (additive-only change)

```text
cargo fmt --all -- --check => PASS
cargo clippy --workspace --all-targets --locked -- -D warnings => PASS (exit 0, ~15m, full native build)
```

No existing match was exhaustive over `Capability`/`AuthorityError`/`RequestBody`
(all use wildcards or match on other types); facade dispatch untouched because no
`RequestBody`/`ResponseBody` variants were added in 074-A. The 19 typed Core
envelope bodies + 19 `capability_matches` pairs landed in 074-C
(`core/authority/project_graph.rs` + facade arms) using exactly the frozen
vocabulary; no parallel capabilities or second error taxonomy were invented.

## Exact-head CI binding

Pushed-head CI is recorded in EXACT_HEAD_QUALIFICATION.md at closure time.
Pre-074-A pushed head `bbeab3d` CI run `35214900525` => completed/success.
Pending CI is never claimed as PASS.
