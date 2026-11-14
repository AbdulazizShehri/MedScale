# Feature Specification: Patient Workspace + Longitudinal UX

**Feature Branch**: `spec/061-patient-workspace-longitudinal-ux`
**Created**: 2026-09-15
**Status**: CLOSED_CANONICAL
**Promotion**: POST_SPEC_060_PRODUCT_SURFACE_SEQUENCE

## Goal
Turn the native Desktop shell into a useful patient workspace that presents trusted longitudinal projections with explicit provenance, coverage gaps, and synthetic-only demo state, without inventing clinical authority or bypassing the Core Host model.

## User Stories

### US1 — Patient workspace
A user can open Patients and see a calm patient header, overview, longitudinal timeline, labs, medications, documents, care-plan state, and source/provenance context.

### US2 — Trusted longitudinal presentation
Timeline, overview, and coverage semantics map to the existing `SubjectTimelineV1`, `SubjectBriefV1`, and `SubjectCoverageV1` contracts. Unsupported or absent concepts are shown honestly rather than filled with invented data.

### US3 — Source drill-down
A user can inspect where a displayed trusted fact came from and distinguish evidence/provenance from clinical authority.

### US4 — Safe incomplete states
Medication, document, and care-plan areas must render explicit unsupported/empty/deferred states when the current trusted projection does not support them. No UI placeholder may imply a capability is qualified when it is not.

### US5 — Keyboard-accessible navigation
Patient workspace tabs, list selection, source drill-down, and primary safe actions are keyboard reachable with native accessible roles and labels.

## Requirements
- **FR-001**: Preserve native Slint 1.16.1, Rust MSRV 1.88, femtovg 0.23.2, AccessKit, and no WebView/Tauri.
- **FR-002**: Patient workspace rendering MUST consume or faithfully model existing trusted presentation contracts; UI code MUST NOT open canonical storage, keys, or network clients directly.
- **FR-003**: Runtime demo content MUST be deterministic, synthetic, and visibly labeled as such.
- **FR-004**: Overview MUST expose identity/condition/coverage state without turning missing or conflicting evidence into affirmative facts.
- **FR-005**: Timeline MUST preserve source-aware longitudinal ordering semantics and expose source/provenance affordances.
- **FR-006**: Labs MUST present trusted Observation-derived values when available and surface unit/conflict/coverage state honestly.
- **FR-007**: Medications MUST show an explicit unsupported/empty state unless a trusted supported projection exists; no synthetic medication regimen may be presented as patient truth.
- **FR-008**: Documents MUST distinguish admitted/source evidence from unsupported document understanding and MUST NOT claim OCR/ASR completeness.
- **FR-009**: Care plan MUST remain review-first and non-authoritative; 061 may show state/entry points but MUST NOT commit an authority-changing workflow.
- **FR-010**: Source drill-down MUST expose provenance/evidence semantics and MUST NOT imply relevance equals authority.
- **FR-011**: All primary patient workspace controls require keyboard focus, accessible labels/roles, and non-color-only status language. No WCAG conformance claim is authorized.
- **FR-012**: `--smoke`, `--perf-idle-ms`, package qualification, three-OS CI, and release-honesty claims MUST remain intact.
- **FR-013**: Real PHI, production credentials, mobile, signing/notarization, MESC, and release-readiness claims remain out of scope.

## Success Criteria
- Patients route is a real native patient workspace rather than a scheduled placeholder.
- A deterministic synthetic patient model renders overview, timeline, labs, honest medication/document/care-plan states, and source/provenance context.
- Regression tests bind the patient surface to trusted projection vocabulary and honesty rules.
- Workspace fmt/clippy/tests and exact-head required CI are green.
- Portable package qualification remains successful on Windows, macOS, and Linux.
- No MESC changes and no change to Spec 012 status.

## Anti-scope
Population/cohort analytics and assistant reasoning (062); workflow execution/tasks/messages (063); audit/exports/settings/integration completion (064); CLI parity/polish (065); final hardening and WCAG/performance qualification (066-067); real PHI; production signing/notarization.
