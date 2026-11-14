# Feature Specification: Documents + OCR + Voice

**Feature Branch**: `spec/010-documents-ocr-voice`  
**Created**: 2026-08-25  
**Status**: QUALIFIED for implementation (005+008 CLOSED)  
**Input**: Hostile-document MIME/quarantine fail-closed intake; P1 worker ambient-deny; OCR/ASR FixtureStubs producing DerivedSourceArtifact + Proposal-only outputs. No real engines, REAL_PHI, or unsandboxed parse.

## User Stories

### US1 - MIME Quarantine (P1)
Synthetic document bytes declare MIME; deny matrix quarantines forbidden types; allowed types admit as SourceRecord under claim path.

### US2 - Hostile Worker Policy (P1)
Document/voice workers use WorkerSupervisionPolicy deny-by-default; no ambient DB/keys/network/authority.

### US3 - OCR Stub (P1)
OCR FixtureStub maps image/PDF fixture → DerivedSourceArtifact + Proposal (never ClinicalAssertion).

### US4 - ASR Stub (P1)
ASR FixtureStub maps audio fixture → transcript DerivedSourceArtifact + Proposal with synthetic source spans.

### US5 - Critical-number / Arabic design hooks (P1)
Evidence registers trap classes for Spec 008/007 corpora handoff; measurement deferred.

## Anti-scope
Real sherpa/whisper/docling/PaddleOCR; REAL_PHI; online download; OS PLATFORM_QUALIFIED as exit; unsandboxed office parse.
