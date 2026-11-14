# Feature Specification: Evidence Corpus Lifecycle (Q10)

**Feature Branch**: `spec/025-evidence-corpus-lifecycle`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 011 CLOSED; Specs 016/019 (Q02/Q06) CLOSED  
**Does not**: claim `RELEASE_READY`, clinical evidence quality, OpenMed superiority, licensed corpora, vector/GraphRAG, REAL_PHI.

## User Stories

### US1 — Versioned synthetic corpus admission (P1)
Operator/tests admit a Pack-like local corpus manifest with corpus source identity, version, per-doc digests, corpus content digest, and `rights=synthetic_owned`.

### US2 — Lexical retrieval over admitted corpus (P1)
Lexical search ranks active documents; EvaluationRecords remain `evidence_only` with `relevance_is_not_authority`.

### US3 — Freshness / retraction / conflict markers (P1)
Document markers travel as evidence metadata on hits and evaluation payloads. Retracted docs are excluded by default (or marked when explicitly included). Markers never become ClinicalAssertion.

### US4 — Version bump changes identity (P1)
Bumping corpus version changes `source_identity` (`corpus_id@version`) and content digest; source identity remains distinct from content hash.

### US5 — Doctor honesty (P1)
Doctor `evidence_corpus` reports READY_BASE versioned synthetic corpus without clinical-quality or RELEASE_READY claims.

## Requirements

- **FR-001**: Replace hardcoded three-note `SYNTHETIC_CORPUS` with admitted versioned corpus (`synthetic-lexical@1.0.0`).
- **FR-002**: Corpus manifest: source identity, version, digests, rights URI, document lifecycle status.
- **FR-003**: Lexical retrieval + filters (`include_retracted` default false).
- **FR-004**: Freshness/retraction/conflict as evidence metadata only.
- **FR-005**: EvaluationRecords `evidence_only=true`; Proposal ≠ ClinicalAssertion preserved.
- **FR-006**: Doctor axis honesty; no RELEASE_READY / clinical quality / OpenMed superiority.
- **FR-007**: Tests: load, search hits, retracted excluded/marked, version bump identity, relevance ≠ authority.
- **FR-008**: Update BUILD_QUEUE / roadmap / START_HERE: Spec 025 CLOSED; deferred advanced **026+**.
- **FR-009**: Synthetic/owned fixtures only; no licensed corpora.

## Out of scope

Vector search, GraphRAG, licensed literature corpora, OpenMed absorption claims, clinical guideline authority, RELEASE_READY, PRIVATE_DATA_READY, MULTI_CLIENT_RELEASE_READY, REAL_PHI, MESC mutation.

## Success Criteria

- Workspace `--locked` green; focused Spec 025 tests PASS
- Doctor honesty flags as above
- Evidence LIMITATIONS honest; no release/clinical-quality claims
