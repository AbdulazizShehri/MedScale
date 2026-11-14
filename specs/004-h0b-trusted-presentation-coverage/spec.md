# Feature Specification: H0-B Trusted Presentation + Coverage

**Feature Branch**: `spec/004-h0b-trusted-presentation-coverage`

**Created**: 2026-08-25

**Status**: Package complete; `QUALIFIED` for implementation (depends on Spec 003 `CLOSED_CANONICAL`)

**Input**: Deliver deterministic, LLM-free presentation over promoted canonical resources from Spec 003 vault/ingest: subject timeline, narrow Brief, honest coverage/unknown/absence/conflict/time-precision accounting, source drill-down, and deterministic Projection rebuild—using bounded typed per-resource extractors (no general FHIRPath engine), with UCUM/unit semantics where required, synthetic-only fixtures, and no models, real PHI, product network, OpenMed/MESC runtime, OCR/ASR, Desktop/CLI product shell, or production vault encryption.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Deterministic Subject Timeline (Priority: P1)

An engineer (or later CLI/Desktop client) can request a timeline for a synthetic subject; MedScale returns a deterministically ordered sequence of timeline events derived only from promoted ClinicalAssertions and their linked SourceRecords—never from unpromoted Proposals, validator EvaluationRecords as clinical truth, or LLM summarization.

**Why this priority**: Roadmap H0-B exit gate; Trusted Local Longitudinal Record wedge depends on inspectable longitudinal order with explicit time precision.

**Independent Test**: Golden fixture vault with known promoted assertions yields byte-stable / structure-stable timeline Projection; reorder of rebuild calls does not change ordering.

**Acceptance Scenarios**:

1. **Given** a subject with ≥3 promoted ClinicalAssertions with distinct effective/recorded times, **When** `GetTimeline` / rebuild timeline Projection runs, **Then** events appear in the documented deterministic order (effective time primary, recorded time secondary, assertion id tertiary) with each event’s `MedicalTime` + `TimePrecision` preserved (no invented Instant precision).
2. **Given** an assertion lacking effective time, **When** timeline is built, **Then** the event uses recorded time with an explicit precision/absence flag—never silent “now” or guessed causality.
3. **Given** only Proposals or EvaluationRecords for a subject (no ClinicalAssertions), **When** timeline is requested, **Then** the timeline is empty or explicitly “no promoted events,” and Proposals are not rendered as timeline clinical events.

---

### User Story 2 - Narrow LLM-Free Brief (Priority: P1)

A narrow Brief summarizes what the system can honestly say from promoted resources for a subject: structured sections filled only by typed extractors, with explicit unknown/absence where fields are missing—never free-text model generation.

**Why this priority**: Master plan H0-B; first product wedge requires a useful offline Brief without any model installed.

**Independent Test**: Fixture subject with known Patient/Observation/Condition assertions produces Brief matching golden JSON; mutating a source and rebuilding changes Brief only via extractor rules.

**Acceptance Scenarios**:

1. **Given** promoted Patient demographics + Observation vitals fixtures, **When** `GetBrief` runs, **Then** Brief contains only admitted section keys (identity, vitals subset, conditions subset, coverage summary) populated by typed extractors.
2. **Given** a missing optional field (e.g. no birthDate), **When** Brief is built, **Then** that slot is `Unknown` or `Absent` per coverage rules—not omitted silently as if complete.
3. **Given** any attempt to attach model/LLM text to Brief, **When** facade is invoked, **Then** no such capability exists; Brief remains rule/extractor-only.

---

### User Story 3 - Honest Coverage, Unknown, Absence, Conflict (Priority: P1)

Coverage accounting distinguishes **Unknown** (system did not look / extractor unsupported), **Absent** (looked; not present in admitted resources), and **Conflict** (multiple promoted values disagree under typed rules)—plus time-precision honesty—without silently resolving conflicts.

**Why this priority**: Constitution-level longitudinal truth; roadmap exit explicitly requires negative/absence/conflict tests.

**Independent Test**: Negative fixtures (missing Observation, conflicting Conditions, year-only dates) produce CoverageReport with expected statuses; no silent winner.

**Acceptance Scenarios**:

1. **Given** an admitted extractor path with no matching promoted resource, **When** coverage is computed, **Then** status is `Absent` (not `Unknown`).
2. **Given** a resource type outside the H0-B admitted extractor set, **When** coverage is queried for that concept, **Then** status is `Unknown` / `UnsupportedExtractor`—not fake `Absent`.
3. **Given** two promoted ClinicalAssertions that conflict on the same typed claim key (e.g. two different active Condition codes for the same coded slot under conflict rules), **When** coverage/Brief is built, **Then** both values are retained under `Conflict` with evidence refs; neither is dropped as “latest wins” unless an explicit later Spec defines authorized resolution (out of H0-B scope).
4. **Given** times with `Year` or `Month` precision, **When** ordering or display fields are produced, **Then** precision is preserved and approximate/false-causality flags follow Spec 002 MedicalTime rules.

---

### User Story 4 - Source Drill-Down (Priority: P1)

From any timeline event, Brief field, or coverage slot, the user can drill down to exact SourceRecord identity, content digest, and tagged TextSpan (prefer raw-byte coordinates on source bytes) proving where the extracted value came from.

**Why this priority**: Provenance is constitutive of trusted presentation; GLM F-08 span model.

**Independent Test**: Every extracted Brief/timeline field in the golden suite resolves to ≥1 evidence ref with valid span or whole-resource citation; corrupt/missing source fails closed.

**Acceptance Scenarios**:

1. **Given** a Brief field populated from an Observation value, **When** `DrillDown` is called, **Then** response includes source_id, content_digest, media_type, and TextSpan (or whole-resource citation when field is structural) into the exact SourceRecord bytes.
2. **Given** a Projection body field, **When** its `built_from` / evidence refs are followed, **Then** they resolve only to durable SourceRecord / ClinicalAssertion ids—not ephemeral parse caches as authority.
3. **Given** a quarantined or missing blob for a cited source, **When** drill-down runs, **Then** fail closed with typed error; do not invent span text.

---

### User Story 5 - Deterministic Projection Rebuild (Priority: P1)

Timeline, Brief, and Coverage views are rebuildable non-authoritative Projections. Deleting and rebuilding with the same durable inputs yields identical structure (golden rebuild). Projections never become ClinicalAssertion or SourceRecord substitutes.

**Why this priority**: Roadmap exit; Projection ≠ truth; Spec 003 provided rebuild hooks—004 owns real presentation kinds.

**Independent Test**: Golden rebuild suite: rebuild twice → equal Projection bodies; after adding one assertion, rebuild differs only in expected deltas.

**Acceptance Scenarios**:

1. **Given** durable promoted inputs for subject S, **When** `RebuildProjection` runs for `SubjectTimelineV1`, `SubjectBriefV1`, and `SubjectCoverageV1`, **Then** each Projection has `authoritative = false` and `built_from` listing input ids.
2. **Given** those Projections deleted, **When** rebuilt again with same inputs, **Then** bodies are deterministically equal (canonical JSON encoding rules documented).
3. **Given** a client that treats Projection as clinical authority, **When** facade reads are typed, **Then** API surface keeps Projection distinct; no promote-from-projection path in H0-B.

---

### User Story 6 - Bounded Typed Extractors (No General FHIRPath) (Priority: P1)

Presentation derives structured fields via a closed set of typed per-resource extractors (Patient, Observation, Condition—and only other types explicitly admitted in research). No general FHIRPath engine, expression language, or arbitrary path query is admitted in H0-B.

**Why this priority**: MASTER_BUILD_PLAN §16 / GLM F-14; roadmap exit gate.

**Independent Test**: Extractor unit tests per resource type; suite asserts no FHIRPath crate/API surface; unsupported resource → `Unknown`/`Unsupported`.

**Acceptance Scenarios**:

1. **Given** a promoted Observation with quantity+unit, **When** the Observation extractor runs, **Then** it emits typed fields (code, value, unit, time) via fixed paths—not a FHIRPath expression string.
2. **Given** a FHIR resource type with no admitted extractor, **When** presentation rebuild scans sources, **Then** it records `UnsupportedResourceType` coverage and does not attempt dynamic path evaluation.
3. **Given** dependency audit / crate graph, **When** Spec 004 implement closes, **Then** no general FHIRPath engine dependency is admitted for H0-B.

---

### User Story 7 - UCUM / Unit Semantics Where Required (Priority: P1)

Where Observation (or other admitted) quantities carry units, H0-B applies bounded UCUM/unit semantics sufficient for honest comparison and conflict detection in the admitted vitals/quantity set—without claiming a full terminology server or bundling licensed code systems.

**Why this priority**: Roadmap exit; F-04 narrowing; avoid silent unit-ignorant conflicts.

**Independent Test**: Same quantity in `kg` vs `g` (or admitted equivalent pair) either normalize under pinned UCUM subset rules or report `Conflict`/`IncomparableUnits`—never treat unequal units as equal numbers.

**Acceptance Scenarios**:

1. **Given** two Observations with numerically equal values but incompatible units under the admitted UCUM subset, **When** conflict/coverage runs, **Then** they are not silently equal; report `IncomparableUnits` or normalized compare only when rules admit conversion.
2. **Given** a unit string outside the admitted UCUM subset, **When** extracted, **Then** value is shown with `UnitUnrecognized` / unknown comparability—not invented conversion.
3. **Given** Spec 004 closeout, **When** rights/provenance are reviewed, **Then** any UCUM tables/code used are pinned with STANDARD/dependency admission—no silent OpenMed terminology table copy.

### Edge Cases

- Subject with zero sources: empty timeline/Brief with explicit empty coverage; no panic.
- Unmerged subjects sharing identifier strings: presentation scoped to explicit subject_ref; no silent merge in timeline.
- Proposal-only claim: visible in optional “non-authoritative candidates” only if explicitly requested; default Brief/timeline exclude Proposals as clinical events.
- Quarantined SourceRecord cited by assertion: fail closed on drill-down; coverage marks evidence unhealthy.
- Clock skew / timezone: use stored MedicalTime strings + precision; do not reinterpret with local TZ heuristics in H0-B.
- Extremely large subject (many assertions): bounded result pages or documented max for H0-B tests; no unbounded memory claim without evidence.
- Real PHI-looking strings in fixtures: remain labeled synthetic; REAL_PHI authority remains NO.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide deterministic subject Timeline from promoted ClinicalAssertions linked to SourceRecords, ordered by documented total order with preserved TimePrecision.
- **FR-002**: System MUST provide a narrow LLM-free Brief composed solely of typed extractor outputs and coverage honesty fields; MUST NOT load models or generate free-text via LLM.
- **FR-003**: System MUST compute Coverage distinguishing at least `Present`, `Absent`, `Unknown`/`Unsupported`, and `Conflict`, plus time-precision and unit-comparability honesty where applicable.
- **FR-004**: System MUST support source drill-down from presentation fields to SourceRecord id, digest, and tagged TextSpan (raw-byte preferred) or whole-resource citation.
- **FR-005**: Timeline, Brief, and Coverage MUST be rebuildable Projections (`authoritative = false`) with golden deterministic rebuild tests.
- **FR-006**: Field extraction MUST use a closed set of bounded typed per-resource extractors; MUST NOT admit a general FHIRPath engine in H0-B.
- **FR-007**: Where quantities require unit semantics, system MUST apply a pinned admitted UCUM/unit subset with fail-closed unrecognized units; MUST NOT bundle licensed SNOMED/LOINC tables as a side effect.
- **FR-008**: System MUST include negative tests for absence, conflict, unrecognized units, unsupported resource types, and golden rebuild equality.
- **FR-009**: Presentation MUST be served only through the Spec 002 authority facade / Core Host; clients MUST NOT open the canonical store or Projection tables directly.
- **FR-010**: Spec 004 MUST consume Spec 003 vault/ingest/visibility/rebuild hooks; MUST NOT reimplement ingest lexical custody or durability GC/backup.
- **FR-011**: Spec 004 MUST NOT implement production vault encryption/KeyProvider (005), Desktop/CLI product shell beyond optional debug/facade tests (006), models/packs (008+), OpenMed/MESC runtime, OCR/ASR, product runtime network egress, real PHI, NPHIES/actions, or general FHIRPath.
- **FR-012**: All fixtures and evidence MUST be synthetic-only; REAL_PHI remains unauthorized.
- **FR-013**: Implementation SHOULD place extractors in `medscale-fhir` (or thin presentation module) and orchestration in `medscale-core`, extending `medscale-contracts` envelopes; prefer existing crates until ownership pressure requires a split.
- **FR-014**: Proposals and EvaluationRecords MUST NOT be treated as ClinicalAssertion when building default Timeline/Brief clinical content.

### Key Entities

- **TimelineEvent**: Ordered presentation event (assertion ref, times, claim summary, evidence refs).
- **TimelineProjection**: Projection kind `SubjectTimelineV1` body.
- **BriefSection / BriefProjection**: Narrow structured Brief (`SubjectBriefV1`).
- **CoverageSlot / CoverageReport / CoverageProjection**: Per-concept honesty (`SubjectCoverageV1`).
- **TypedResourceExtractor**: Closed per-resource extractor interface (Patient, Observation, Condition, …).
- **ExtractedField**: Typed field + evidence refs + span citations.
- **ConflictSet**: Multiple disagreeing promoted values for one claim key.
- **DrillDownResult**: Source + digest + TextSpan / citation.
- **UnitSemanticResult**: Normalized or `Incomparable` / `Unrecognized` under admitted UCUM subset.
- **PresentationRebuildRequest**: Facade rebuild for presentation Projection kinds.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Golden timeline suite (≥1 subject, ≥3 promoted events) rebuilds twice with identical canonical Projection body encoding.
- **SC-002**: Golden Brief suite matches expected section population; missing fields → `Absent`/`Unknown` as specified (100% of declared fixtures).
- **SC-003**: Absence fixture yields `Absent`; unsupported-type fixture yields `Unknown`/`Unsupported` (zero silent Present).
- **SC-004**: Conflict fixture retains ≥2 values under `Conflict` with evidence refs (zero silent single-winner resolution).
- **SC-005**: Drill-down suite: 100% of populated Brief/timeline fields resolve to valid source citation or fail closed on quarantine.
- **SC-006**: Unit suite: incompatible units never compare equal; unrecognized units never silently convert.
- **SC-007**: Dependency/evidence check: no general FHIRPath engine admitted; no model crates; synthetic-only fixtures.
- **SC-008**: Deliverable introduces no real PHI, product network clients, OpenMed/MESC runtime, OCR/ASR, Spec 005 encryption requirement, or Spec 006 product UI shell as authority.

## Assumptions

- Spec 003 is `CLOSED_CANONICAL` with durable ingest, blob-first visibility, and `RebuildProjection` hooks available.
- Spec 002 MedicalTime / TimePrecision / Projection / TextSpan contracts remain binding.
- “Promoted canonical resources” means ClinicalAssertions authorized via Spec 002 promote path, evidence-linked to Spec 003 SourceRecords (synthetic FHIR R4).
- H0-B admitted extractors start with Patient, Observation, Condition; additional resource types require research.md amendment before Present claims.
- UCUM subset is engineering-pinned STANDARD input, not a full terminology Pack product (007 owns broader terminology rights program).
- Ordinary engineering choices follow `docs/planning/IMPLEMENTATION_DECISION_DEFAULTS.md` and are recorded in `research.md` / `clarifications.md`.
- Product runtime network remains DEFAULT_DENY by absence.
