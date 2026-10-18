# Cursor — MedScale Autonomous Build Entry Point

This file is the single execution entry point for Cursor.

## Standing founder directive

```text
FOUNDER_STANDING_CURSOR_DIRECTIVE = AUTHORIZED
CURSOR_IMPLEMENTATION_AUTHORITY = YES_WITHIN_CANONICAL_PLAN
ASK_FOUNDER_FOR_ORDINARY_IMPLEMENTATION_DECISIONS = NO
CONTINUE_TO_NEXT_ELIGIBLE_UNIT = YES
STOP_AFTER_ONE_TASK_OR_PR = NO
```

Read, in order, before doing any material work:

1. `AGENTS.md`
2. `docs/planning/START_HERE.md`
3. `docs/planning/IMPLEMENTATION_AUTHORITY.md`
4. `docs/planning/BUILD_QUEUE.md`
5. `docs/planning/MASTER_BUILD_PLAN_V2.md`
6. `docs/planning/SPECKIT_MASTER_ROADMAP_V2.md`
7. `docs/planning/IMPLEMENTATION_DECISION_DEFAULTS.md`
8. `docs/planning/SOURCE_ACQUISITION_AND_COPY_PLAN.md`
9. `docs/planning/V0_UI_INTEGRATION_CONTRACT.md`
10. relevant source/OSS/OpenMed matrices
11. relevant current Spec Kit package or execution handoff

Spec 001 is historical and closed. Use the live queue and read
`docs/planning/TRUSTED_V1_DELIVERY_PLAN.md` and its bounded follow-on package.
Foundation closure does not establish product release readiness.

Then verify live GitHub/repository truth and execute the first `READY` unit in `BUILD_QUEUE.md`.

## Do not ask the founder routine questions

Resolve ordinary ambiguity from repository truth, frozen invariants, primary upstream evidence, tests, and the defaults in `IMPLEMENTATION_DECISION_DEFAULTS.md`. Prefer fail-closed, local-first, privacy-preserving, Rust-owned, minimal, reversible choices. Record architecture-impacting choices in the owning spec/research/ADR and continue.

If a genuinely external human gate is required, record it in `docs/planning/EXTERNAL_GATES.md`, continue every independent task, and do not turn the gate into a general project stop.

Historical planning-only `Implementation authorization: NO` metadata is superseded by the active standing authority document and must not trigger another founder question.

## Finish condition

Continue through every eligible V2 spec until all current roadmap work is `CLOSED_CANONICAL`, `DEFERRED_BY_CANONICAL_DESIGN`, or blocked only by an explicitly recorded external gate. Do not treat a passing demo as completion. Use `docs/planning/DEFINITION_OF_DONE.md`.

## UI

The latest Patients priority is recorded in `docs/planning/TAURI_PATIENTS_PRIORITY_2026-10-01.md`. Preserve the approved native shell and continue bounded Spec 097 preparation. Use English for communication and technical work. Native acceptance, Core wiring and predecessor/review gates remain open.

The 2026-09-30 founder decision selects staged Tauri 2 + React/TypeScript/Vite/Tailwind presentation migration. Read `docs/planning/TAURI_DESKTOP_MIGRATION_DECISION_2026-09-30.md` and the preparatory `specs/096-tauri-foundation/` package. Preserve 095 Slint history, current Slint functionality, Core authority, the open WebView privacy gate and 095 dependency/review blocks.

For the 2026-09-29 founder program, docs/planning/PRODUCTIZATION_PROGRAM.md is current visual authority and admits direct native implementation of the supplied approved identity. Earlier v0 ownership below is historical for this program. Core/integration boundaries remain.

The founder uses v0 for visual/UI creation. Follow `docs/planning/V0_UI_INTEGRATION_CONTRACT.md`. Cursor owns trusted core, contracts, integration, wiring, accessibility, tests, packaging, and privacy qualification. Cursor does not independently redesign the final UI and never accepts v0-generated server/database/network authority as MedScale authority.
