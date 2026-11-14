# DESKTOP_QUALIFICATION — Spec 075

## Surface

Native Slint `Data` route (`crates/medscale-desktop/ui/app.slint`):
sources (active project), snapshots, typed schema header, row grid,
provenance detail, import/refresh/schema-ack/grid-view/transform actions,
explicit empty/loading/error/denied/conflict/corrupt/partial/unavailable
states via `data-status`. Light/dark through the shared `Theme` tokens;
keyboard/focus through shared components (`FocusableClickArea`,
`AdaptiveLineEdit`, `ToolbarAction`, `QuickAction`); no color-only truth
states (`StatusPill` carries text labels).

## Authority

All state arrives through `data_workbench.rs` view-models backed exclusively
by `CliSession` (Core). No direct storage/driver/network access from UI.
Covered by in-module session tests (`workbench_flows_through_real_core_session`,
`workbench_input_parsers_reject_gracefully`).

## Renders

`DATA_WORKBENCH_LIGHT.png` / `DATA_WORKBENCH_DARK.png`: deterministic
renders of the Data route. Local rendering is unavailable on this
workstation (no display/link toolchain), and `.github/workflows/ci.yml`
has no rendering/screenshot step either, so these renders were never
captured through any path used to qualify this spec. This is an honest,
recorded residual, not a fabricated PASS: the two project-switch and
snapshot-ordering fixes from the exact-range review
(`crates/medscale-desktop/src/main.rs`) were verified by tracing the exact
Slint property flow (`app.slint`'s `in`/`in-out` property declarations
against the Rust setter calls), not by a rendered or interactive session,
because this codebase has no headless `AppWindow` test harness either
(confirmed by grep: `AppWindow::new()` appears only in the real `main()`).
A screenshot alone would never substitute for the session-assertion tests
above in any case.

```text
RESULT = PASS (compiled + in-module session tests): exact-head CI run
35580861670 (head 9f84a6e) green 6/6; PR #129 merged as 89a88cf;
post-merge main run 35582548200 green 6/6.
RESIDUAL = no rendered PNG evidence exists for this spec (no local or CI
rendering path); no headless AppWindow test harness exists for the two
Desktop-only exact-range-review fixes. Tracked for whichever future
spec adds Desktop UI-logic/rendering test infrastructure.
```
