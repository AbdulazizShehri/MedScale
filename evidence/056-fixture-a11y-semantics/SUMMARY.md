# Evidence SUMMARY - Spec 056 Fixture A11y Semantics (READY_BASE)

- Every `FixtureUiViewModel` constructor assigns role + keyboard focus order +
  keyboard path + canonical state announcement.
- All six lifecycle states (ready/loading/empty/error/conflict/recovery) have
  exact canonical text; error/conflict disclose no action was taken.
- Doctor `fixture_state_semantics_checked=true`; `wcag_conformance_claimed`
  stays false; `final_v0_ui_present` stays false; release not claimed.
- Pre-056 JSON without `semantics` still parses; reads as not-honest.
- Tests: `fixture_a11y_semantics_056.rs` (6 tests) + extended CLI fixture test
  + existing 029 tests.
