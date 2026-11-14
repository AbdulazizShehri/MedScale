# Feature Specification: Trusted Object / Source / Authority + Process/Text Foundation

**Feature Branch**: `spec/002-trusted-object-source-authority-foundation`

**Created**: 2026-08-25

**Status**: Ready after Spec 001 closes (`BLOCKED_BY_001` until then)

**Input**: Establish MedScale’s durable object classes, source/text identity, realm/authority scope, proposal/promotion semantics, effect/retry vocabulary, process topology (single-writer Core Host), tagged text coordinates, native/FFI hardening contract, and worker supervision policy stubs—without H0 ingest, presentation, or vault encryption.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Durable Object Classes Are Explicit and Distinct (Priority: P1)

An engineer working in the trusted core can create, serialize, and round-trip the durable object classes without collapsing Proposal into ClinicalAssertion, Projection into truth, or derived text into source bytes.

**Why this priority**: All later H0 and product specs depend on class separation; silent authority collapse is an architectural failure.

**Independent Test**: Property and serialization tests prove each class retains distinct type identity, required fields, and forbidden promotions without storage or FHIR ingest.

**Acceptance Scenarios**:

1. **Given** in-memory object instances for each durable class, **When** they are serialized and deserialized, **Then** type tags and required identity fields round-trip without loss or silent coercion across classes.
2. **Given** a Proposal produced by a synthetic “capability” stub, **When** an unauthorized caller attempts to treat it as a ClinicalAssertion, **Then** the authority facade rejects the promotion and records a typed denial.
3. **Given** a Projection rebuilt from assertions, **When** inspected, **Then** it is marked non-authoritative and never substitutes for SourceRecord or ClinicalAssertion.

---

### User Story 2 - Source Bytes Stay Distinct From Derived Text (Priority: P1)

An engineer can model immutable source identity separately from derived representations and attach tagged text spans with explicit coordinate systems.

**Why this priority**: Source drill-down, OCR/ASR later, and Unicode safety all require tagged coordinates from day one.

**Independent Test**: Unit/property tests create SourceRecord + DerivedSourceArtifact pairs and validate span tagging, forbidden silent mutation of source bytes, and conversion helpers between declared coordinate systems.

**Acceptance Scenarios**:

1. **Given** a SourceRecord with immutable bytes and a content digest, **When** a DerivedSourceArtifact is created (e.g., normalized text), **Then** source identity remains independent of content hash and derived bytes never overwrite source bytes.
2. **Given** a span tagged `RawByte` on a SourceRecord, **When** mapped to a Unicode-scalar span on a derived text artifact, **Then** the conversion is explicit, versioned, and fails closed on invalid offsets.
3. **Given** a request for “silent normalize source,” **When** submitted, **Then** the system refuses and requires a DerivedSourceArtifact with transform metadata.

---

### User Story 3 - Single-Writer Core Host and Authority Facade (Priority: P1)

CLI/Desktop/SDK clients share one authority facade; desktop/headless topology is one Core Host per open vault with a single-writer lease; no client opens the canonical store directly.

**Why this priority**: Process topology is a V2 exit gate for Spec 002 and blocks unsafe multi-writer designs later.

**Independent Test**: In-process host/lease simulator tests prove exclusive writer acquisition, denial of second writer, and facade methods that never expose DB/key handles. Real OS IPC may be stubbed; contract messages are versioned.

**Acceptance Scenarios**:

1. **Given** a vault identity with no active host, **When** a client acquires the Core Host lease, **Then** it becomes the sole writer and subsequent acquire attempts fail closed until release.
2. **Given** an active Core Host, **When** a client requests a write via the authority facade, **Then** the request is capability-scoped, deadline-bounded, and never transfers canonical DB or key handles.
3. **Given** no host, **When** a CLI-equivalent client needs a write, **Then** it may spawn/own a transient host under the same authorization path without creating a second independent writer.

---

### User Story 4 - Identity, Realm, Time, and Promotion Rules (Priority: P1)

Identity merges are explicit; realm and opaque `authority_scope_id` are first-class; time fields carry precision; promotion from Proposal to ClinicalAssertion requires an authorized path.

**Why this priority**: Wrong-patient merges and false causality are constitutional hazards.

**Independent Test**: Tests attempt silent identity merge (must fail), verify realm/scope on objects, exercise time precision enums, and verify authorized vs unauthorized promotion.

**Acceptance Scenarios**:

1. **Given** two IdentityAssertions that could match, **When** no explicit merge decision exists, **Then** records remain distinct and no automatic merge occurs.
2. **Given** objects with realm and `authority_scope_id`, **When** a cross-scope write is attempted without authorization, **Then** the facade denies it.
3. **Given** a Proposal with evidence links, **When** an authorized promotion command runs, **Then** a ClinicalAssertion is created with provenance to the Proposal and an AuditRecord; unauthorized promotion fails.

---

### User Story 5 - Effect/Retry Vocabulary and Audit Stubs (Priority: P2)

Effect and external-action state vocabulary exists so later specs (014) do not invent blind retries; UNKNOWN is never auto-retried.

**Why this priority**: Fail-closed retry policy must be frozen before action systems exist.

**Independent Test**: State-machine unit tests enforce legal transitions and forbid auto-retry from UNKNOWN.

**Acceptance Scenarios**:

1. **Given** an Action/AuditRecord in state `UNKNOWN`, **When** a retry is requested without reconciliation, **Then** the system refuses.
2. **Given** states `PENDING → SENT → CONFIRMED | FAILED | UNKNOWN`, **When** illegal transitions are attempted, **Then** they are rejected with typed errors.

---

### User Story 6 - Native/FFI Hardening and Worker Supervision Contracts (Priority: P2)

Native/FFI admission and worker supervision policies are documented and expressed as typed contracts/stubs so later specs (005/008/010) wire real engines without inventing authority.

**Why this priority**: Roadmap exit gate; prevents “FFI equals confinement” regressions.

**Independent Test**: Contract tests and compile-time types encode P0–P3 placement, FFI checklist fields, and worker capability denials (no ambient DB/keys/network/FS/authority). No real workers required.

**Acceptance Scenarios**:

1. **Given** a proposed native dependency record missing ABI/ownership/unwind rules, **When** validated against the FFI contract, **Then** admission fails closed.
2. **Given** a WorkerSupervisionPolicy stub, **When** inspected, **Then** it denies ambient canonical DB, master keys, unrestricted filesystem, network, secrets, and authority by default.
3. **Given** engine placement classes P0–P3, **When** a complex hostile parser is classified, **Then** it cannot be labeled P0 solely because it is wrapped in Rust FFI.

### Edge Cases

- Second process attempts vault write while lease held: deny; do not corrupt or share writer connection.
- Span offset past end of representation: fail closed; never clamp silently.
- Content-hash collision or hash-as-identity misuse: source identity remains distinct; hash is evidence only.
- Proposal confidence/score present: never grants ClinicalAssertion authority.
- Mobile topology note: Spec 002 records app-owned in-process core as the mobile pattern; does not implement mobile FFI.
- Serialization of unknown future fields: versioned envelopes reject or ignore only per explicit compatibility rules (fail closed on authority-bearing unknowns).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST define distinct durable object classes: SourceRecord, DerivedSourceArtifact, Proposal, ClinicalAssertion, EvaluationRecord, Action/AuditRecord, Projection, IdentityAssertion, plus Realm and opaque `authority_scope_id`.
- **FR-002**: System MUST enforce `Proposal != ClinicalAssertion`, `Projection != truth`, and `AI/worker output != authority` at type and facade boundaries.
- **FR-003**: System MUST treat source bytes as immutable evidence; source identity MUST NOT equal content hash; derived transforms MUST create DerivedSourceArtifact records with transform/loss/version metadata.
- **FR-004**: System MUST represent text spans with tagged representation and coordinate system (`RawByte`, `UnicodeScalar`, and explicit conversion points for UTF-16/JS/Swift/Kotlin later).
- **FR-005**: System MUST implement (in-process for this spec) the single-writer Core Host decision: one writer per vault lease; clients use the authority facade; no DB/key handle transfer.
- **FR-006**: System MUST provide a versioned authority-facade message/API sketch covering acquire/release lease, capability-scoped requests, deadlines/limits, and typed errors.
- **FR-007**: System MUST make identity merge explicit via IdentityAssertion / merge decision records; silent merge MUST be impossible through the facade.
- **FR-008**: System MUST model time with explicit effective/recorded/acquired (as applicable) and precision/approximation without inventing false causality.
- **FR-009**: System MUST define promotion rules from Proposal to ClinicalAssertion that require authorization and produce AuditRecord provenance.
- **FR-010**: System MUST define effect/retry vocabulary with states `PENDING`, `SENT`, `CONFIRMED`, `FAILED`, `UNKNOWN` and MUST forbid blind retry from `UNKNOWN`.
- **FR-011**: System MUST publish a native/FFI hardening contract (ABI, ownership, unwind, thread affinity, typed errors, placement class, evidence fields).
- **FR-012**: System MUST publish worker supervision policy stubs denying ambient DB/keys/network/unrestricted FS/secrets/authority; real OS sandbox wiring is deferred.
- **FR-013**: System MUST provide property and serialization tests proving round-trip and class-separation invariants.
- **FR-014**: Spec 002 MUST NOT implement H0 ingest (003), presentation/Brief/coverage (004), vault encryption/key lifecycle (005), product UI, models, product runtime network, real PHI, or MESC mutation.
- **FR-015**: Implementation MUST target `medscale-contracts` and `medscale-core` modules first; further crate splits only when compile-time ownership pressure justifies them (may be noted, not required in 002).

### Key Entities

- **SourceRecord**: Immutable raw bytes + identity metadata (not content-hash identity).
- **DerivedSourceArtifact**: Versioned transform of a source with loss/transform metadata.
- **Proposal**: Non-authoritative candidate claim or extraction with evidence links.
- **ClinicalAssertion**: Authorized clinical truth claim with promotion provenance.
- **EvaluationRecord**: Assessment/evidence about proposals, coverage, or quality—not authority by itself.
- **Action/AuditRecord**: Durable intent/effect/audit trail including effect state machine fields.
- **Projection**: Rebuildable, non-authoritative derived view.
- **IdentityAssertion**: Explicit identity statement; merge only via explicit decision.
- **Realm / authority_scope_id**: Explicit tenancy/authorization scope boundaries.
- **TextSpan**: Offset range tagged by representation + coordinate system.
- **CoreHostLease**: Single-writer lease for a vault identity.
- **WorkerSupervisionPolicy**: Capability denials and placement class for future workers.
- **FfiAdmissionRecord**: Checklist-backed native dependency admission metadata.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All durable object classes serialize/deserialize with 100% round-trip success on a synthetic fixture suite (≥1 fixture per class).
- **SC-002**: Property tests demonstrate zero successful silent coercions of Proposal→ClinicalAssertion or Projection→SourceRecord.
- **SC-003**: Single-writer lease tests show second writer acquire fails in 100% of contested cases in the in-process simulator.
- **SC-004**: Tagged span conversion tests cover RawByte↔UnicodeScalar happy path and ≥3 invalid-offset failure cases (fail closed).
- **SC-005**: Effect state machine tests reject UNKNOWN auto-retry and illegal transitions in 100% of adversarial cases in suite.
- **SC-006**: FFI and worker policy contract validators reject incomplete admission records and ambient-capability grants.
- **SC-007**: Spec deliverable introduces no H0 ingest, presentation UI, encryption vault, model load, product network client, or real PHI fixture.

## Assumptions

- Spec 001 has closed (or is closing) with `medscale-contracts` and `medscale-core` crates available for module expansion.
- In-process Core Host / lease simulator satisfies Spec 002 topology requirements; OS-local IPC transport lands in Spec 006 (or earlier if needed) using these contracts.
- ICU4X or equivalent Unicode helpers may be admitted later behind MedScale’s text contract; Spec 002 may implement minimal conversion tests without full ICU if decision defaults prefer smallest reversible path.
- Synthetic-only fixtures; no real PHI; no models; product runtime network remains DEFAULT_DENY by absence.
- Ordinary engineering choices follow `docs/planning/IMPLEMENTATION_DECISION_DEFAULTS.md` and are recorded in `research.md`.
- Purpose/legal basis/consent remain versioned flow decision placeholders; Spec 002 does not invent law in code.
