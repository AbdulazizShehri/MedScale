# Feature Specification: H0-A Trusted Ingest + Durability

**Feature Branch**: `spec/003-h0a-trusted-ingest-durability`

**Created**: 2026-08-25

**Status**: Package complete; implementation `BLOCKED_BY_002` until Spec 002 is `CLOSED_CANONICAL`

**Input**: Deliver synthetic FHIR R4 trusted custody: lexical-safe ingest, exact source-byte preservation, validator and identity evidence (evidence only), blob-first canonical visibility, projection rebuild hooks, and durability proofs (blob integrity, GC/promotion races, restore completeness, migration interruption, claim-scoped filesystem, synthetic-scope backup/restore interface)—without real PHI, models, product network, OpenMed/MESC runtime, OCR/ASR, presentation UI, or production vault encryption.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Synthetic FHIR R4 Trusted Ingest (Priority: P1)

An engineer can submit synthetic FHIR R4 4.0.1 Bundle/resource bytes through the authority facade; MedScale stores exact source bytes as a SourceRecord, records lexical/parse outcomes, attaches validator EvaluationRecords as evidence only, and never treats FHIR JSON as the canonical database.

**Why this priority**: H0-A exit gate; all later presentation (004) and vault (005) work depends on trusted custody of synthetic interchange.

**Independent Test**: Fixture suite of synthetic FHIR resources exercises accept/reject paths, exact-byte round-trip, and EvaluationRecord evidence-only semantics without UI or encryption.

**Acceptance Scenarios**:

1. **Given** a synthetic FHIR R4 Patient JSON fixture, **When** ingest completes successfully, **Then** a SourceRecord holds byte-identical payload, content digest matches SHA-256 of those bytes, and media type is recorded (e.g. `application/fhir+json`).
2. **Given** the same bytes re-ingested, **When** content digest matches an existing SourceRecord in scope, **Then** the system records a typed duplicate/idempotent outcome without mutating prior source bytes.
3. **Given** an external validator sidecar/oracle result (or fixture stub of one), **When** attached to the ingest, **Then** it is stored only as EvaluationRecord evidence and NEVER as ClinicalAssertion authority.

---

### User Story 2 - Lexical Safety and Interchange Hardening (Priority: P1)

Hostile or malformed synthetic interchange is fail-closed: duplicate JSON keys, unsafe number/decimal forms, version mismatches, and lexical traps do not corrupt the store or silently coerce clinical meaning.

**Why this priority**: Roadmap exit explicitly requires lexical/duplicate-key/decimal/version tests; memory-safe bounded FHIR/JSON parsing is P0-class infrastructure, not a clinical engine.

**Independent Test**: Adversarial fixture corpus (duplicate keys, oversized numbers, wrong FHIR version markers, truncated JSON, BOM/encoding traps) proves reject-or-quarantine with typed errors and no durable corruption.

**Acceptance Scenarios**:

1. **Given** JSON with duplicate object keys in a clinically relevant path, **When** ingested, **Then** the pipeline fails closed (reject or quarantine) and does not pick an arbitrary “last key wins” clinical interpretation.
2. **Given** decimal/number forms that would lose precision or overflow under naive float parsing, **When** ingested, **Then** the system preserves exact lexical/decimal policy (reject or exact decimal type) and never silently float-coerces.
3. **Given** a resource declaring or implying a non-R4 4.0.1 interchange version outside the admitted H0-A pin, **When** ingested, **Then** version gate fails closed with typed error; no partial canonical visibility.
4. **Given** truncated or non-JSON bytes labeled as FHIR JSON, **When** ingested, **Then** reject; no SourceRecord promotion to visible canonical custody without successful lexical accept policy.

---

### User Story 3 - Identity Evidence Without Silent Merge (Priority: P1)

Ingest may extract identifier evidence into IdentityAssertion / Proposal records linked to sources; subjects remain distinct unless an explicit IdentityMergeDecision exists (Spec 002 rules).

**Why this priority**: Wrong-patient merge is a constitutional hazard; H0-A must prove identity evidence wiring on real durable paths.

**Independent Test**: Two synthetic Patients sharing an identifier string produce distinct subjects and IdentityAssertions; merge only via authorized DecideIdentityMerge.

**Acceptance Scenarios**:

1. **Given** two synthetic Patient resources with the same identifier system/value, **When** both are ingested, **Then** two subjects (or two unmerged identity graphs) remain distinct and IdentityAssertions cite source evidence.
2. **Given** those IdentityAssertions, **When** no merge decision exists, **Then** facade reads never collapse them into one subject.
3. **Given** an authorized IdentityMergeDecision, **When** applied, **Then** merge is explicit, audited, and reversible only via further authorized identity operations (no silent undo).

---

### User Story 4 - Blob-First Canonical Visibility + Projection Rebuild Hooks (Priority: P1)

Canonical visibility is blob-first: metadata references content-addressed blobs; visible custody requires digest-verified blob presence. Projection rebuild hooks exist so Spec 004 can rebuild non-authoritative views deterministically from durable inputs.

**Why this priority**: Master plan H0-A + GLM durability rules; projections must never become truth.

**Independent Test**: After ingest, blob read verifies digest/size; missing/corrupt blob fails closed and is quarantined with evidence; rebuild hook API regenerates a stub Projection from assertion/source ids.

**Acceptance Scenarios**:

1. **Given** a successfully ingested SourceRecord, **When** canonical visibility is queried, **Then** the blob is readable only after digest+size verification against stored metadata.
2. **Given** a corrupted or truncated blob file, **When** read is attempted, **Then** the system quarantines, records Evaluation/Audit evidence, and does not present the resource as healthy custody.
3. **Given** durable sources/assertions for a synthetic subject, **When** `RebuildProjection` (stub kind allowed in 003) runs, **Then** a Projection is produced with `authoritative = false` and `built_from` provenance; deleting the Projection and rebuilding yields deterministic structure for the same inputs.

---

### User Story 5 - Durability: Integrity, GC/Promotion Races, Crash/Fault (Priority: P1)

Crash/fault injection and concurrency tests prove blob digest integrity, crash-safe blob GC (mark/tombstone/sweep) racing with promotion, and no orphaned visible metadata without blobs (and no premature GC of live blobs).

**Why this priority**: Roadmap exit gate; durability is constitutive of “trusted custody.”

**Independent Test**: Fault-injection harness (process kill mid-write, mid-GC, mid-promotion) plus race tests; post-recovery invariants hold.

**Acceptance Scenarios**:

1. **Given** a blob write interrupted mid-fsync/rename path, **When** the vault is reopened, **Then** either the ingest is absent (no partial visible custody) or fully present with matching digest—never half-visible corrupt state.
2. **Given** GC running while a concurrent promotion references a blob, **When** both complete, **Then** the live blob is retained; GC never deletes a blob still referenced by visible metadata.
3. **Given** a blob tombstoned after all references drop, **When** sweep completes across a crash boundary, **Then** unmarked live blobs remain and tombstones are not resurrected as healthy custody without re-ingest.

---

### User Story 6 - Restore Completeness, Migration Interruption, Claim-Scoped Filesystem (Priority: P1)

Backup/restore interface for **synthetic scope** proves restore completeness; schema/migration interruption leaves the store upgrade-safe or rolled back; filesystem durability claims are scoped to tested OS/filesystem classes.

**Why this priority**: GLM F-10 (003 proves synthetic backup/restore interface; 005 owns production encryption/recovery UX); F-03 filesystem claim discipline.

**Independent Test**: Backup → wipe → restore round-trip; kill during migration; document claim scope for local app-data filesystem class used in tests.

**Acceptance Scenarios**:

1. **Given** a synthetic vault with N sources/blobs/metadata, **When** backup then restore into a clean location runs, **Then** every visible metadata reference has its required blob and matching digest (restore closure).
2. **Given** a metadata schema migration in progress, **When** the process is killed, **Then** reopen either completes migration idempotently or remains on the prior schema without torn authoritative state.
3. **Given** durability tests, **When** evidence is archived, **Then** claim scope names the OS + filesystem class under test; unsupported remote/sync roots are outside the release claim (detect/refuse or non-claim—not waved through).

---

### User Story 7 - Synthetic-Scope Backup/Restore Interface Proof (Priority: P1)

A versioned Backup/Restore interface exists and is tested for synthetic H0-A vaults. It is **not** production KeyProvider/encryption/recovery UX (Spec 005).

**Why this priority**: Explicit roadmap bullet for Spec 003.

**Independent Test**: Contract tests + end-to-end synthetic backup/restore proof archived as evidence.

**Acceptance Scenarios**:

1. **Given** `BackupVault` capability, **When** invoked on a synthetic vault, **Then** a portable backup artifact (manifest + blobs + metadata snapshot) is produced with content digests and schema version.
2. **Given** that artifact, **When** `RestoreVault` runs to an empty target under single-writer lease, **Then** restore closure checks pass and facade reads match pre-backup visibility for synthetic fixtures.
3. **Given** a tampered backup blob digest, **When** restore runs, **Then** fail closed; partial restore does not become visible canonical custody.

### Edge Cases

- Empty Bundle / zero-entry ingest: typed accept or reject per policy; no panic; no empty “success” that invents subjects.
- Extremely large synthetic payload beyond configured max: reject with size limit; no unbounded memory growth claim without evidence.
- Concurrent second writer: denied by Spec 002 lease; durability layer never opens a second writable metadata connection.
- Validator oracle unavailable: ingest may still custody exact bytes with EvaluationRecord noting `validator_skipped` / fail policy—never invent ClinicalAssertion from absence.
- GC vs restore: restore must not race-delete newly restored blobs; restore holds lease / marks references before sweep.
- Path traversal in claim-scoped FS layout: reject; blobs only under vault-relative claim paths.
- Real PHI-looking strings in fixtures: fixtures remain labeled synthetic; REAL_PHI authority remains NO.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST accept synthetic FHIR R4 4.0.1 interchange (JSON resource/Bundle) only through the authority facade / Core Host; clients MUST NOT open the canonical store directly.
- **FR-002**: System MUST persist exact source bytes as SourceRecord; source identity MUST remain distinct from content digest; digests are SHA-256 evidence.
- **FR-003**: System MUST enforce lexical safety for FHIR JSON ingest: duplicate-key fail-closed policy, decimal/number precision policy, admitted FHIR version gate, and bounded parse limits.
- **FR-004**: External validator output (HL7 validator / HAPI / Inferno or fixture stubs) MUST be stored only as EvaluationRecord evidence; FHIR remains interchange, not canonical DB; validators NEVER grant ClinicalAssertion authority.
- **FR-005**: System MUST record identity evidence from ingest as IdentityAssertion and/or Proposal linked to sources; silent identity merge MUST remain impossible.
- **FR-006**: System MUST implement blob-first storage: metadata references blobs; canonical read verifies digest and size; corruption → quarantine + evidence.
- **FR-007**: System MUST provide projection rebuild hooks (API + stub projection kind) so projections are rebuildable and non-authoritative.
- **FR-008**: System MUST prove crash/fault durability for ingest writes, blob promotion, and GC (mark/tombstone/sweep) including GC↔promotion race tests.
- **FR-009**: System MUST verify blob digest integrity on read and on restore; restore closure MUST require every visible metadata reference to have its blob+digest.
- **FR-010**: System MUST handle metadata migration interruption safely (idempotent resume or clean prior schema).
- **FR-011**: System MUST confine vault files to a claim-scoped local filesystem layout; durability PASS claims MUST name OS/filesystem class; remote/sync roots are non-claim or refused by default.
- **FR-012**: System MUST provide and test a synthetic-scope Backup/Restore interface (manifest, digests, restore closure); MUST NOT claim production encryption, KeyProvider, or key-loss recovery (Spec 005).
- **FR-013**: Storage MUST sit behind a narrow DurableStore / BlobStore interface. Spec 003 MAY use an unencrypted SQLite candidate (one SQLite copy per process) for metadata. SQLCipher remains a Spec 005 candidate, not constitutional, and MUST NOT be required by Spec 003.
- **FR-014**: Spec 003 MUST NOT implement H0-B timeline/Brief/coverage UI (004), production vault encryption/recovery UX (005), Desktop/CLI product shell beyond optional debug commands (006), models/packs (008+), OpenMed/MESC runtime, OCR/ASR, product runtime network egress, real PHI, or NPHIES/actions.
- **FR-015**: Implementation SHOULD expand `medscale-storage` and/or `medscale-fhir` modules/crates only as justified by ownership boundaries; prefer modules in existing crates until compile-time pressure requires splits (roadmap §5).
- **FR-016**: All fixtures and evidence MUST be synthetic-only; REAL_PHI remains unauthorized.

### Key Entities

- **IngestReceipt**: Facade result of an ingest attempt (accepted / rejected / quarantined) with source_id, digests, evaluation refs.
- **BlobRef**: Content-addressed blob pointer (digest, size, storage locator within claim scope).
- **BlobRecord**: Durable blob metadata + lifecycle state (`Live`, `Tombstoned`, `Quarantined`).
- **CanonicalVisibilityRecord**: Scope-checked metadata that a source/resource is visible only when blob verify succeeds.
- **ValidatorEvidence**: EvaluationRecord subtype/payload for external validator/oracle output.
- **IdentityEvidenceBundle**: IdentityAssertions / Proposals emitted from ingest, evidence-linked to SourceRecord.
- **ProjectionRebuildHook**: Command/API to rebuild a named Projection kind from durable inputs.
- **GcMarkState / Tombstone**: Crash-safe GC bookkeeping.
- **BackupArtifact / BackupManifest**: Synthetic-scope backup package with schema version and digests.
- **FilesystemClaimScope**: Declared OS + filesystem class for which durability evidence applies.
- **MigrationJournal**: Records in-flight schema migration for interrupt-safe resume.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Synthetic FHIR fixture suite (≥1 Patient, ≥1 Observation, ≥1 Bundle, ≥3 adversarial lexical cases) shows 100% exact-byte custody on accepts and 100% fail-closed on declared reject fixtures.
- **SC-002**: Duplicate-key, decimal/precision, and version-gate tests each have ≥1 failing adversarial fixture that is rejected/quarantined (zero silent coerce passes).
- **SC-003**: Validator evidence tests prove EvaluationRecord attachment without any ClinicalAssertion auto-creation (100% of suite).
- **SC-004**: Identity suite proves no silent merge across ≥2 subjects sharing identifier strings; authorized merge path remains the only unification.
- **SC-005**: Blob integrity tests: digests mismatch → quarantine in 100% of injected corruption cases; healthy reads verify digest+size.
- **SC-006**: GC↔promotion race suite completes with zero live-reference deletions and zero visible metadata lacking blobs after recovery.
- **SC-007**: Crash/fault suite (ingest write, GC sweep, migration) recovers to invariant-holding state in 100% of scripted kill points.
- **SC-008**: Backup → restore closure check passes for a multi-blob synthetic vault; tampered digest restore fails closed.
- **SC-009**: Evidence archive names filesystem claim scope; no PASS claim for untested remote/sync roots.
- **SC-010**: Deliverable introduces no real PHI fixtures, model loads, product network clients, OpenMed/MESC runtime, OCR/ASR, H0-B UI, or SQLCipher requirement.

## Assumptions

- Spec 002 is `CLOSED_CANONICAL` (or equivalent) with durable object classes, Core Host single-writer lease, authority facade, and identity/promotion rules available before Spec 003 implement starts.
- FHIR R4 4.0.1 is the sole admitted H0-A interchange pin; other versions fail closed.
- External validators are sidecar/oracle or recorded fixtures; they are not product authority and need not run in every CI job if fixture evidence substitutes—but the EvaluationRecord shape and evidence-only rule are mandatory.
- Unencrypted local SQLite (or equivalent) behind DurableStore is acceptable for synthetic H0-A; encryption-at-rest is Spec 005.
- One SQLite implementation per process (OSS matrix / master plan supply-chain rule).
- Ordinary engineering choices follow `docs/planning/IMPLEMENTATION_DECISION_DEFAULTS.md` and are recorded in `research.md` / `clarifications.md`.
- Product runtime network remains DEFAULT_DENY by absence; development-time fetch of public FHIR fixtures/docs is not product egress.
