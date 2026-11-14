# T077-03 Implementation — AgentIdentity + AgentCapabilityManifest

## Scope

Vertical slice: register/get/list/revoke for `AgentIdentity` +
`AgentCapabilityManifest`, through Core authority exactly (contracts ->
envelopes -> facade -> CLI), mirroring the `pg`/`ds`/`collab` pattern this
codebase already established in Specs 074/075/076.

## Files changed

```text
crates/medscale-contracts/src/envelopes/mod.rs
  - Capability::AgentIdentityRegister/Read/Revoke (+ is_read_or_health,
    operator_grants entries)
  - RequestBody::AgentIdentityRegister/Get/List/Revoke
  - ResponseBody::MedAgentIdentity/MedAgentIdentityList

crates/medscale-core/src/authority/medagent.rs (new)
  - MedAgent<'a> authority view (store/meta/packs/sessions/leases/vault_id/
    realm/scope/session_id), mirrors Collab<'a> field-for-field
  - register_agent_identity/get_agent_identity/list_agent_identities/
    revoke_agent_identity
  - actor()/audit() duplicated from collaboration.rs per this spec's own
    established convention (storage/medagent.rs's own header comment:
    "so this module never depends on Spec 076's closed file")

crates/medscale-core/src/authority/mod.rs
  - mod medagent;

crates/medscale-core/src/authority/facade.rs
  - medagent<R>() helper (mirrors collab<R>()/ds<R>()/pg<R>() exactly)
  - 4 dispatch arms in dispatch_inner
  - 4 capability_matches pairs

crates/medscale-core/src/cli_session.rs
  - CliSession::medagent_identity_register/get/list/revoke

crates/medscale-cli/src/medagent.rs (new)
  - MedAgentCmd::{IdentityRegister,IdentityShow,IdentityList,IdentityRevoke}
  - run_medagent(), human + --json output, error mapping mirroring
    collaboration.rs's collab_fail

crates/medscale-cli/src/main.rs
  - mod medagent; Commands::MedAgent { action: Box<medagent::MedAgentCmd> }

crates/medscale-core/tests/medagent_077.rs (new, 4 tests)
specs/077-medagent-workbench/{tasks.md,contracts.md}
evidence/077-medagent-workbench/T077-03_IMPLEMENTATION.md (this file)
```

## Design decision: `pack_version` is Core-derived, never caller-supplied

`RequestBody::AgentIdentityRegister` carries only `pack_id`, not
`pack_version`. `MedAgent::register_agent_identity` resolves the
currently admitted `PackManifestV0` for that `pack_id` via
`PackStore::get(&pack_id)` and captures its real `.version` field onto the
new `AgentIdentity`. A `pack_id` the local `PackStore` does not recognize
(never admitted, or since-removed) returns `AuthorityError::InvalidArgument`
before any row is written -- fail-closed per `security.md` T5. This closes
an otherwise-open spoofing vector where a caller could otherwise claim an
arbitrary `pack_version` string unrelated to what is actually admitted.
Recorded in `contracts.md`'s `AgentIdentity` section.

## Tests

`crates/medscale-core/tests/medagent_077.rs`, using a single-actor
`CoreFacade` harness (trimmed from `collaboration_076.rs`'s multi-actor
one, since agent-identity lifecycle is scope-level, not
membership-gated) and the real Spec 008 fixture pack
(`evidence/008-local-ai-capability-fabric/fixtures/pack-fixture-ner-v0`):

1. `register_get_list_revoke_roundtrip_through_core` -- full lifecycle
   through real `CoreFacade::dispatch`: register (asserts `pack_version`
   was captured, non-empty, and matches the admitted pack), get, list,
   revoke, then a second revoke attempt with the now-stale
   `expected_revision` fails closed with `Conflict`.
2. `registration_against_non_admitted_pack_fails_closed` -- registers
   against a `pack_id` that was never installed; asserts
   `AuthorityError::InvalidArgument`, proving `security.md` T5's fail-closed
   requirement end-to-end through the facade, not just at the storage
   layer.
3. `capability_manifest_is_immutable_and_registration_is_scoped_to_its_project` --
   two identities in two different Projects (same admitted pack) get
   distinct ids and independent capability manifests; the first identity's
   manifest is unaffected by the second registration's wider grant set.
4. `agent_identity_survives_vault_reopen` -- registers an identity, drops
   the `Harness` (closing the in-process facade), opens a brand new
   `CoreFacade` against the same on-disk synthetic vault root, and reads
   the identity back with an identical `pack_version`/`revision` --
   matching `collaboration_076.rs`'s own reopen-durability precedent.

No local compile/test run was possible on this workstation (MSVC linker
absent, the same constraint every prior spec in this repository has
recorded); `cargo fmt --check` is clean across the whole workspace after
this change.

## Exact-head CI qualification

Three CI iterations were required before green (all findings genuine
compiler/clippy errors, not flakiness -- each fixed forward with a new
commit, never by suppressing the check):

```text
HEAD 925b6f6 -> FAILED: matches!() does not accept a third format-string
  argument (that belongs to assert!()) in medagent_077.rs storage test
  (run 35705130450).
HEAD 8b097fe -> not separately CI-qualified (superseded before its run
  completed by the next fix, pushed after local review caught two more
  issues: redundant .clone() calls and an unproven two-facade drop-timing
  assumption in the reopen test).
HEAD 970c395 -> FAILED: clippy::enum_variant_names -- all 4 MedAgentCmd
  variants shared the "Identity" prefix (run 35705694772).
HEAD af00d34 -> SUCCESS, run 35706622805, 6/6
  (supply-chain policy present, cargo-deny, perf delivery-plan scale
  (windows), rust (ubuntu-latest), rust (macos-latest),
  rust (windows-latest)).
```

T077-03 is qualified at exact head `af00d344360adc4f3e6a26aecdb1590e9ed34d06`.
