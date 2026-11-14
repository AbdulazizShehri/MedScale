# Security and Threat Delta — Spec 074

**Status:** implementation contract

Spec 074 adds durable Project/Experiment organization and a typed Project Graph. It adds no runtime network service, no remote worker, no model execution authority, no team sync and no real-PHI authorization.

## 1. Trust boundary

The authority path remains:

```text
CLI/Desktop
   -> typed Core command/query
      -> actor/session/scope checks
      -> input + revision validation
      -> canonical target-reference validation
      -> encrypted storage transaction
      -> audit/result
```

No CLI/Desktop component may access Project tables directly.

## 2. Threats and required controls

### T1 — ID spoofing / cross-scope reference

Attack: provide an `OpaqueId` belonging to another realm/authority scope or an object type different from the declared ArtifactKind.

Controls:

- resolve target through canonical owner/Core path;
- verify realm/authority scope before attach/edge creation;
- verify declared kind against resolved contract/class where possible;
- deny before storage write;
- do not reveal protected target metadata in denial response.

Tests: same-scope valid, wrong scope, unknown ID, mismatched kind, denied actor.

### T2 — Stale-write overwrite

Attack: two callers update Project/Experiment/ref/edge from the same old state and the later request silently overwrites the newer result.

Controls:

- explicit expected revision/precondition;
- transaction verifies revision atomically with write;
- mismatch -> `Conflict`;
- no generic last-write-wins.

Tests: concurrent/stale metadata update, archive/update race, detach/update race, edge remove/update race if applicable.

### T3 — Graph injection / semantic escalation

Attack: create arbitrary string predicates such as `diagnoses`, `trusted`, or `proves` and later treat them as authority.

Controls:

- closed/versioned predicate enum for canonical 074 edges;
- organizational/workflow semantics only;
- unknown canonical predicate fails closed;
- inferred/model/search relations are not admitted as ProjectGraphEdge in 074.

Tests: invalid/unknown predicate serialization and Core request denial.

### T4 — Graph traversal resource exhaustion

Attack: build dense/cyclic graph then issue unbounded traversal.

Controls:

- foundation API is bounded neighbor query, not arbitrary recursive traversal;
- explicit page size/max limit;
- deterministic ordering/cursor if pagination is supported;
- indexes for subject/object/predicate queries;
- no recursive SQL/DFS exposed by default.

Tests: dense cyclic fixture, maximum page size, over-limit request, repeated pagination.

### T5 — Metadata injection

Attack: oversized/control-character/markup/SQL-like Project names/descriptions cause storage/query/UI issues.

Controls:

- bounded UTF-8 validation;
- parameterized storage APIs;
- UI renders text as text, not executable markup;
- logging safely escapes/bounds user metadata.

Tests: empty/whitespace, max boundary, over max, unusual Unicode, control characters according to chosen policy, SQL-like strings.

### T6 — Information leak through summaries/counts

Attack: actor cannot read an artifact but learns existence/type/count from Project summary/context.

Controls:

- authorization/scope filtering before returning reference metadata/counts;
- denied target resolution does not reveal sensitive display metadata;
- ProjectContext contains only authorized refs;
- cache, if any, is scoped to the same authorization context; 074 should prefer no new sensitive shared cache.

Tests: mixed visible/denied targets, summary counts, context resolution.

### T7 — Cascade deletion / ownership confusion

Attack: archiving Project or detaching last reference deletes canonical patient/document/evidence/model data.

Controls:

- Project references are non-owning;
- no cascade from Project tables into target canonical stores;
- GC cannot treat absence of Project refs as proof target is unused;
- destructive Project deletion is not a 074 feature.

Tests: archive/detach/edge removal leaves target object intact and resolvable through original workflow.

### T8 — Half-committed authority after crash

Attack: Project row commits but required associated state does not, or an edge/index partially persists.

Controls:

- define atomic transaction boundaries;
- use current writer lock/transaction mechanism;
- crash/reopen tests;
- migration transactions fail closed.

Tests: simulated/deterministic failure before/after commit boundaries.

### T9 — Corrupt/stale reference confusion

Attack: malformed storage or deleted target causes UI to display another object or fabricate success.

Controls:

- exact ID + kind + version binding;
- typed `Missing`/`Stale`/`Corrupt`/`UnsupportedKind` states;
- no fuzzy rebinding;
- corrupt rows do not become valid defaults.

Tests: missing target, digest/version mismatch, invalid serialized row where test harness permits, unsupported kind/version.

### T10 — Alternate authority path in CLI/Desktop

Attack: surface code reimplements checks or writes DB directly for convenience.

Controls:

- Core-only mutation/query path;
- dependency-direction gate;
- code review/search for storage imports in surface modules where forbidden;
- parity tests across CLI/Desktop for representative operations.

### T11 — Migration identity loss

Attack: migration rewrites current IDs or creates Project copies of canonical objects.

Controls:

- additive migration;
- before/after identity/digest fixture comparison;
- existing objects remain valid outside Projects;
- no automatic synthetic Project population by default.

### T12 — Log disclosure

Attack: full sensitive artifact metadata/body enters logs while resolving Project context.

Controls:

- operational logs prefer opaque IDs, state, kind, revision/digest where safe;
- no artifact bodies/patient fields/document text by default;
- bounded error messages;
- existing log/privacy rules remain authoritative.

## 3. Authorization posture

074 does not invent team roles/ReBAC. It uses current actor/session/scope rules and introduces only the minimum operation semantics required by the Core implementation.

A future 075/083 may add collaboration/membership, but 074 data structures must not hard-code assumptions that every Project is globally visible or single-user forever.

## 4. Network posture

No new product runtime egress. A Project operation must be fully functional in offline/local mode.

Do not add remote project sync, WebSocket, HTTP client, cloud DB or browser dependency in 074.

## 5. Privacy posture

No new universal Privacy Gate taxonomy is implemented before 078. Existing sensitivity/authority controls remain in force.

Project metadata itself should be treated conservatively because names/descriptions may contain sensitive information; do not emit them to logs/telemetry. MedScale has no hidden telemetry authority.

## 6. Supply-chain posture

No new dependency is expected. If implementation proposes one, stop that mutation until the active spec records exact need, version, license/security review, dependency direction, tests and removal/update strategy.

## 7. Security closure gate

Before 074 closure, evidence must show:

```text
cross_scope_reference_denied = true
kind_mismatch_denied_or_explicit = true
stale_write_conflict_proven = true
unknown_predicate_fail_closed = true
graph_query_bounded = true
metadata_validation_proven = true
summary_context_permission_filtering_proven = true
project_archive_does_not_delete_target = true
crash_reopen_atomicity_proven = true
migration_preserves_existing_identity = true
surface_bypass_not_present = true
new_runtime_network_path = false
real_phi_used = false
```

Any false item is a Spec 074 closure blocker unless canonical governance explicitly narrows the claim/scope.