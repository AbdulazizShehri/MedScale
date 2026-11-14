# T077-04 Implementation — ContextManifest

## Scope

`ContextManifest` create/get through Core authority, plus the single
Core-internal read-boundary check (`security.md` T4) every future
tool-dispatch path (T077-06) must consult.

## Design decisions

**Shape-only validation at write time; live resolution at read time.**
`migration.md` section 4 is explicit: "`ContextManifest`'s artifact
references (`ArtifactDescriptor`s) are validated for shape at write time;
their live resolution is always recomputed at read time through the
existing Project-graph resolution path, never cached as a foreign-key join
that could go stale silently." `create_context_manifest` therefore only
calls `ArtifactDescriptor::validate()` (binding shape) and
`ContextManifest::new`'s own bound checks (non-empty, max count) -- it does
NOT check that a named artifact exists. `get_context_manifest` recomputes
every artifact's `ReferenceResolution` fresh on every call via
`resolve_context_artifact`. A manifest may legitimately name an artifact
that does not exist yet (resolves `Missing`) or has since gone stale
(resolves `Stale`); this is the intended design, verified by a dedicated
test, not an oversight.

**`resolve_context_artifact` duplicates `project_graph::
ProjectGraph::resolve_descriptor`'s logic** rather than importing it,
matching `collaboration.rs`'s own precedent (it duplicates the same logic
as `resolve_anchor`) and `storage/medagent.rs`'s explicit module-
independence rationale ("so this module never depends on Spec 076's closed
file"). The duplicate is a deliberate simplification of the original for
non-`PackManifest` objects: it checks class-matching and returns
`Current` only for `IdentityOnly` bindings, `Stale` for anything else,
rather than replicating every object family's own digest-comparison logic
-- adequate for this spec's `ReadContextArtifact`/`SearchContextArtifacts`
tool vocabulary (source/derived text artifacts), which does not need
`Proposal`/`ClinicalAssertion`-grade digest pinning semantics.

**`require_artifact_in_context` is the single boundary-check primitive.**
It fetches the (scope-checked) `ContextManifest` and calls its own
`.allows(object_id)`, returning `Unauthorized` on refusal. It does not
consult storage a second time or take any other path to "is this object
readable" -- `security.md` T4's control ("there is exactly one
Core-internal function an agent run's tool dispatch may call to resolve
artifact content, and it always consults the bound `ContextManifest`
first") is satisfied by this function existing as the sole entry point;
T077-06's tool dispatch must call it before any artifact content read,
never invent a second path.

## Files changed

```text
crates/medscale-contracts/src/envelopes/mod.rs
  - Capability::ContextManifestCreate/Read
  - RequestBody::ContextManifestCreate{project_id,selected_artifacts}/
    ContextManifestGet{context_id}
  - ResponseBody::MedAgentContextManifest{manifest,resolutions}

crates/medscale-core/src/authority/medagent.rs
  - stored_class_matches (duplicated from project_graph.rs)
  - MedAgent::scoped_context_manifest/resolve_context_artifact/
    create_context_manifest/get_context_manifest/require_artifact_in_context
  - inline #[cfg(test)] mod tests (MedAgent is not exported for external
    integration tests, mirroring project_graph.rs's own inline-test
    precedent): resolve_context_artifact_matrix_matches_project_graph_semantics,
    create_and_get_context_manifest_resolves_live_never_cached,
    require_artifact_in_context_allows_named_denies_everything_else,
    context_manifest_is_scope_isolated

crates/medscale-core/src/authority/facade.rs
  - 2 dispatch arms (ContextManifestCreate re-fetches+resolves after
    create for a consistent response shape) + 2 capability_matches pairs

crates/medscale-core/src/cli_session.rs
  - CliSession::medagent_context_create/get

crates/medscale-cli/src/medagent.rs
  - MedAgentCmd::ContextCreate/ContextShow (repeatable --artifact
    object_id:kind spec, IdentityOnly binding -- CLI scope simplification
    mirroring collaboration.rs's own simple_anchor precedent)

specs/077-medagent-workbench/tasks.md
evidence/077-medagent-workbench/T077-04_IMPLEMENTATION.md (this file)
```

No local compile/test run was possible (MSVC linker absent, same
constraint as every prior spec). `cargo fmt --check` is clean across the
whole workspace after this change.

## Exact-head CI qualification

Three iterations to green, all genuine findings fixed forward:

```text
HEAD 334fbd4 -> FAILED (run 35709355030): clippy dead_code on
  require_artifact_in_context (no production caller yet -- T077-06 is its
  first one) and too_many_arguments (8/7) on the inline test harness()
  (realm/scope parameters needed for the cross-scope test, unlike
  project_graph.rs's simpler 6-arg harness).
HEAD 787deb6 -> FAILED (run 35709884342): clippy clean, but 3 inline tests
  panicked with LeaseRequired -- create_context_manifest's audit() call
  resolves the actor via session or lease holder, and the tests never
  acquired a lease.
HEAD b331477 -> SUCCESS, run 35710883996, 6/6.
```

T077-04 is qualified at exact head `b331477808212a490b5bafdc0df54f62a81b3a17`.
