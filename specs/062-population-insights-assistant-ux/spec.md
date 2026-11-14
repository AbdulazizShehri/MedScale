# Feature Specification: Population Insights + Contextual Assistant UX

**Feature Branch**: `spec/062-population-insights-assistant-ux`
**Created**: 2026-09-15
**Status**: CLOSED_CANONICAL
**Promotion**: POST_SPEC_061_PRODUCT_SURFACE_SEQUENCE

## Goal
Add a native Population Insights surface and evidence-contextual assistant UX that summarize deterministic synthetic trusted presentation state across subjects without converting heuristics, retrieval relevance, or aggregate display into clinical authority.

## User Stories

### US1 — Population overview
A user can open Insights and see cohort size, evidence coverage, present-condition distribution, review-attention state, and evidence-gap summaries derived from synthetic trusted projections.

### US2 — Honest cohort exploration
A user can inspect cohort rows while Present, Absent, Unknown, Conflict, IncomparableUnits, UnhealthyEvidence, and UnsupportedResourceType remain semantically distinct.

### US3 — Evidence-contextual assistant
A user can ask a bounded contextual question and receive deterministic evidence-navigation text that preserves corpus/source metadata and `relevance_is_not_authority`.

### US4 — Risk/trend honesty
The product shows explicit non-claims where no qualified clinical risk-score or longitudinal population-trend contract exists. It must not fabricate analytics to fill the visual design.

### US5 — Review cues without action authority
Recommendations may highlight unresolved evidence or route users toward review, but MUST NOT diagnose, prescribe, clinically rank subjects, or commit a controlled action.

## Requirements
- **FR-001**: Preserve native Slint 1.16.1, Rust MSRV 1.88, femtovg 0.23.2, AccessKit, and no WebView/Tauri.
- **FR-002**: Population aggregation MUST consume `SubjectBriefV1` / `SubjectCoverageV1`; Desktop MUST NOT open canonical storage, keys, or network clients directly.
- **FR-003**: Coverage states MUST remain distinct rather than being flattened to a binary present/missing metric.
- **FR-004**: Present-condition distribution MUST count only Present condition facts; unresolved values remain visibly unresolved.
- **FR-005**: Evidence gaps MUST be labeled as evidence/coverage gaps, not clinical care-gap claims.
- **FR-006**: No clinical risk ranking is authorized without a qualified trusted contract; the synthetic demo MUST expose that boundary.
- **FR-007**: Assistant context MAY consume `LexicalRetrieveResult` / `RetrievalHit`, but MUST preserve `evidence_only`, `relevance_is_not_authority`, retraction, freshness, conflict, and corpus identity metadata.
- **FR-008**: Assistant output MUST be deterministic/bounded local UX in this unit; no provider/network/model authority is introduced.
- **FR-009**: Assistant and population recommendations MUST NOT commit authority-changing actions.
- **FR-010**: Synthetic demo content MUST be visibly labeled and deterministic.
- **FR-011**: Existing patient workspace, headless probes, three-OS package qualification, release honesty, and CLI behavior MUST remain intact.
- **FR-012**: Real PHI, MESC, mobile, production signing/notarization, clinical risk qualification, WCAG conformance, and release-readiness claims remain out of scope.

## Success Criteria
- Insights is a real native surface rather than a placeholder.
- Synthetic multi-subject trusted inputs render cohort, coverage, condition, and review-attention views without flattening uncertainty.
- Contextual assistant queries return bounded evidence-navigation text with visible authority non-claims.
- Evidence cards expose corpus identity plus lifecycle/freshness/conflict metadata.
- Local fmt/clippy/tests/MSRV and exact-head required CI pass, including portable package qualification on Windows/macOS/Linux.
- No MESC changes and no change to Spec 012 status.

## Anti-scope
Workflow Studio/tasks/messages (063); audit/exports/settings/integrations (064); CLI parity/polish (065); final hardening/qualification (066-067); provider-backed medical reasoning; diagnosis/treatment recommendation; real PHI; production signing/notarization.
