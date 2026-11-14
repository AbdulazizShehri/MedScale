# Feature Specification: Fixture Accessibility Semantics Deepening (Q-a11y residual)

**Branch**: `spec/056-fixture-a11y-semantics`
**Status**: READY_BASE deepening (this package)
**Promotion**: EXISTING_A11Y_RESIDUAL (extends Spec 029 label honesty with role/keyboard/announcement semantics)
**Does not**: claim WCAG 2.x conformance, assistive-technology qualification, contrast measurement, final v0 UI, or release readiness.

## User Stories

### US1 Per-surface semantics (P1)
Every `FixtureUiViewModel` constructor assigns honest `FixtureA11ySemantics`:
a shell-agnostic role, a zero-based keyboard focus index, a human-readable
keyboard path, and the canonical announcement for its lifecycle state.

### US2 Full lifecycle announcements (P1)
All six lifecycle states (ready, loading, empty, error, conflict, recovery)
carry exact canonical announcement text. Error and conflict states disclose
that no action was taken, matching the architectural invariant
`ACTION_INTENT != ACTION_RESULT`.

### US3 Machine-checkable honesty (P1)
Unit and integration tests assert semantics honesty on every constructor,
reject tampered announcements, keep pre-056 JSON parsing (semantics absent
reads as not-honest, never as conformant), and assert no WCAG/release claim.

## Anti-scope
WCAG audit; assistive-technology product testing; visual/contrast design;
final v0 UI artifact (external gate FINAL_V0_UI_ARTIFACT); MESC.
