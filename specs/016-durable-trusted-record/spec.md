# Feature Specification: Durable Trusted Record Persistence (Q02)

**Feature Branch**: `spec/016-durable-trusted-record`

**Created**: 2026-09-09

**Status**: Package complete; `QUALIFIED` for implementation after analyze PASS (T01–T03)

**Input**: Close the Q02 gap identified by the 2026-09-09 whole-product review: `CoreFacade` / `InMemoryAuthorityStore` authority objects do not survive process restart. Deliver cross-process durable persistence of the full trusted-record object graph through the existing Rust authority path, using synthetic data only. Historical planning inputs are preserved in the private maintainer archive. This unit promotes **one bounded** Spec 016 successor; remaining advanced/deferred capabilities stay `017+` / `DEFERRED_BY_CANONICAL_DESIGN`.

**Depends on**: Specs 002–006 `CLOSED_CANONICAL` (object model, ingest durability, presentation, vault, CLI facade).

**Does not**: admit real PHI; claim `PRIVATE_DATA_READY`; implement Q03 encrypted-metadata platform qualification; implement Q04 authenticated multi-client sessions; mutate MESC; enable live network/actions; redesign UI.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Restart Retains Admitted Record (Priority: P1)

A synthetic-record user opens a vault, imports supported FHIR through the trusted authority path, accepts promotion of assertions, closes MedScale, and reopens the same vault in a **separate process**. They receive identical original source bytes, stable identifiers, assertions, identity decisions, provenance, evaluations, audit history, and deterministic Timeline / Brief / coverage.

**Why this priority**: Without restart durability, MedScale is not a longitudinal record product.

**Independent Test**: Process A imports/promotes/rebuilds projections and exits; Process B opens the same vault root and asserts byte-identical sources, same IDs, same audit/projection digests. An empty in-memory store must not pass.

**Acceptance Scenarios**:

1. **Given** Process A admitted synthetic FHIR and promoted assertions into vault V, **When** Process A exits and Process B opens V, **Then** source digests, source bytes, object IDs, assertions, evaluations, identity objects, and audit rows match Process A’s final snapshot.
2. **Given** Process B after reopen, **When** Timeline / Brief / coverage are requested, **Then** deterministic presentation matches Process A’s last rebuild (or an explicitly rebuilt projection with identical body digest when rebuild-on-open is selected).
3. **Given** no vault open, **When** a fresh process constructs `CoreFacade` with empty memory only, **Then** it cannot satisfy the restart equality fixture.

---

### User Story 2 - Atomic Canonical Promotion (Priority: P1)

Rejected or interrupted writes never become partially visible canonical state. Blob stage and metadata commit follow a documented protocol. Interrupted migrations refuse or recover explicitly.

**Why this priority**: Partial promotion destroys trust in the record.

**Independent Test**: Inject failures after blob stage / mid-metadata transaction / mid-migration; assert no partial canonical visibility and typed recovery or refusal.

**Acceptance Scenarios**:

1. **Given** a staged blob whose metadata transaction aborts, **When** the vault is reopened, **Then** the source is not canonically visible; staging is distinguishable from committed state.
2. **Given** committed metadata referencing a missing/corrupt blob, **When** read/verify runs, **Then** the operation fails closed with a typed integrity error (no silent empty reset).
3. **Given** migration interrupted at `started`, **When** open runs, **Then** open recovers per ADR or refuses with typed `MigrationIncomplete`—never silently resets to empty.

---

### User Story 3 - Scope, Realm, and Writer Ownership (Priority: P1)

Every durable lookup enforces realm/scope. Independent second writers are refused via OS-exclusive ownership (Q02 minimum). Lock-file presence alone is not ownership. Backup/restore includes the full object graph + blobs; wrong-key/corruption preserve destination.

**Why this priority**: Cross-process integrity and isolation are part of Q02 store qualification.

**Independent Test**: Scope mismatch denied; second process open fails while first holds exclusive lock; backup restore to fresh destination validates closure; wrong restore target preserved.

**Acceptance Scenarios**:

1. **Given** object O in scope S1, **When** read is requested under S2, **Then** typed scope denial.
2. **Given** Process A holds the vault writer lock, **When** Process B attempts open for write, **Then** typed `WriterHeld` / equivalent.
3. **Given** a complete backup of vault V, **When** restore runs into a fresh path, **Then** restored objects and blobs match; wrong-key or corrupt backup does not destroy an existing valid destination.

---

### User Story 4 - Authority Class Discipline Survives Persistence (Priority: P1)

Persisted storage preserves `SOURCE != ASSERTION`, `ASSERTION != PROPOSAL`, `PROPOSAL != ACTION`, `PROJECTION != AUTHORITY`, `AI_OUTPUT != CLINICAL_FACT`. Schema version is not clinical effective time. Corrections append lineage; history is not overwritten.

**Why this priority**: Constitutional invariants must not be collapsed by a single JSON blob design mistake.

**Independent Test**: Round-trip each `StoredObject` variant; assert class discriminator and typed deserialization; attempt overwrite of source history fails.

**Acceptance Scenarios**:

1. **Given** a proposal and a promoted assertion, **When** both are persisted and reloaded, **Then** classes remain distinct and `promoted_from_proposal_id` lineage is intact.
2. **Given** an external-action intent audit row, **When** reloaded, **Then** it remains an audit/intent object—not an assertion.
3. **Given** a projection, **When** reloaded, **Then** `authoritative` remains false and rebuild remains possible.

---

## Edge Cases

- Interrupted blob stage before metadata commit
- Metadata commit succeeds but blob finalize fails
- Orphaned staging content vs committed missing content
- Corrupted SQLite page / invalid JSON body
- Digest mismatch on source/derived byte hydrate
- Clock rollback between recorded_time fields (detect/refuse where ADR requires)
- ID allocator counter loss or collision
- Stale lock file after crash (must not block forever; must not allow two live writers)
- Backup missing object row or blob file
- Opening Spec 003 vault (schema v1) upgrades once to schema v2 without data loss of existing sources

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST persist every `StoredObject` class used by `InMemoryAuthorityStore` (Source, Derived, Proposal, Assertion, Audit, Identity, Merge, Evaluation, Projection) through the Core Host authority path.
- **FR-002**: System MUST persist the durable ID allocator sequence so reopened vaults never reuse IDs for new objects.
- **FR-003**: System MUST reload the complete object graph into the facade store on vault open (or equivalent durable-backed store) so facade reads work without a second authority service.
- **FR-004**: System MUST keep blob content-addressed storage; source/derived large bytes MUST verify digest on read.
- **FR-005**: System MUST commit blob+metadata with a staged protocol equivalent to: stage blob → verify digest → transactional metadata commit → finalize canonical visibility.
- **FR-006**: System MUST enforce realm and authority-scope on every durable lookup.
- **FR-007**: System MUST provide OS-exclusive writer ownership for an open vault (Q02 minimum); lock-file presence alone MUST NOT grant ownership.
- **FR-008**: System MUST extend backup/restore to include authority objects + blobs with versioned manifest closure.
- **FR-009**: System MUST refuse silent empty-vault reset, silent history overwrite, and silent acceptance of corruption.
- **FR-010**: System MUST migrate schema from Spec 003/005 v1 sources-only metadata to v2 authority-object schema with journaled migration.
- **FR-011**: CLI/UI/workers/Pack MUST NOT open the canonical DB directly; all writes remain behind `CoreFacade` capabilities.
- **FR-012**: Spec 016 MUST document privacy limitations: open synthetic plaintext metadata and Spec 005 sealed-at-close behavior are **not** `PRIVATE_DATA_READY` (Q03 owns that qualification).
- **FR-013**: Action/outbox intents that share the audit object class MUST persist with the same transaction boundary when touched.
- **FR-014**: All fixtures and tests MUST use synthetic owned data only.

### Non-Functional / Invariants

- IMMUTABLE_ORIGINAL_SOURCE_BYTES
- CONTENT_DIGEST_VERIFIED_ON_READ
- STABLE_UNIQUE_IDS
- REALM_AND_SCOPE_ENFORCEMENT
- ATOMIC_CANONICAL_PROMOTION
- AUDIT_WITH_PROMOTION
- NO_SILENT_EMPTY_VAULT_RESET
- NO_HISTORY_OVERWRITE
- SCHEMA_VERSION != CLINICAL_EFFECTIVE_TIME
- TIME_PRECISION_PRESERVED
- UNKNOWN_PRESERVED
- CORRECTIONS_APPEND_LINEAGE

### Key Entities

See `data-model.md` and `contracts/field-mapping.md`.

## Success Criteria *(mandatory)*

- **SC-001**: Two-process restart fixture PASS on Windows and Linux CI for synthetic admitted record.
- **SC-002**: Failure-injection suite covers interrupted metadata, blob stage, migration, corrupt digest—each recovers or refuses explicitly.
- **SC-003**: Existing Spec 003/004/005/014 tests remain green; no API bypass introduced.
- **SC-004**: Exact-head evidence under `evidence/016-durable-trusted-record/` records commit/tree/commands/results/limitations.
- **SC-005**: `BUILD_QUEUE.md` marks Spec 016 `CLOSED_CANONICAL` only after T08; next Trusted V1 unit (Q03) becomes READY for specification.

## Assumptions

- One SQLite dependency family (`rusqlite`) remains the metadata store.
- SyntheticVault is the primary development path; EncryptedVault receives the same schema via working-copy open/seal without claiming private-data readiness.
- Q03/Q04 follow as separate numbered specs after 016 closeout.
- Process leases remain logical; Q02 adds OS writer lock only.

## Out of Scope

- Real PHI; MESC admission; live SMART/NPHIES; HF online; mobile apps; GraphRAG/plugins; authenticated multi-client capability sessions (Q04); full open-vault metadata encryption qualification (Q03); precision-time model redesign (Q06); final v0 UI.
