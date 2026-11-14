# Desktop Qualification — Spec 078 (T078-07)

## What was added

- Route **Model Fleet** in the native Slint shell (`crates/medscale-desktop/
  ui/app.slint`): a nav item after MedAgent, a header subtitle, and one panel
  built only from existing components (`HonestyPanel`, `AdaptiveLineEdit`,
  `ToolbarAction`, `QuickAction`, `StatusPill`) and `Theme` tokens. So it
  inherits the existing focus order, keyboard activation, and light/dark
  parity, and adds no new colors or widgets.
- Panel content: authority-boundary statement; create-fleet form (task
  prompt), lane-id and Pack-directory inputs; lane list; fleet-run list with
  Open / Dispatch (pending) / Cancel (pending or running); fleet detail with
  per-lane run state and Execute (running lanes); Compare (completed or
  partially failed); the latest report's participating/excluded lanes and
  its factual observations.
- Every interactive or list element carries `accessible-role` +
  `accessible-label` (or `accessible-name` for inputs), following the MedAgent
  and Collaboration panels.
- `crates/medscale-desktop/src/model_fleet_workspace.rs`: view-model
  functions over `CliSession` only; no storage or model-runtime access.
  `main.rs` maps them to Slint rows and routes the `fleet-*` actions. The
  Pack directory goes through the existing `validated_model_pack_path`
  (absolute path required) before any Core call.
- `MEDSCALE_EVIDENCE_ROUTE=Model Fleet` is accepted by the evidence route
  override, so the route can be captured once a rendering environment
  exists.

Agent-lane creation stays CLI-only in this slice, as Spec 077 left
identity/context creation CLI-only.

## Proof

`model_fleet_workspace::tests::model_fleet_workspace_flows_through_real_core_session`
drives every view-model function against a real `CliSession` with real
local ONNX execution: lane list, create, refused compare on a pending fleet,
dispatch, per-lane execute, detail with live lane states, compare,
latest-report readback, conflict status on a terminal cancel, and cancel of
a pending fleet. `status_message_is_explicit_for_every_error_class` covers
the status mapping.

## Rendered evidence: honest residual

No rendered Desktop evidence exists for this route, the same residual Specs
075/076/077 recorded. CI has no rendering step and no headless `AppWindow`
harness. This workstation cannot build the Desktop binary: there is no MSVC
toolchain (`EXTERNAL_GATES.md`), and the WSL distro used for local Linux
builds stopped starting during this session. `app.slint` compiles in CI
through `build.rs` as part of the Desktop crate build and Clippy, which
proves the markup is valid Slint, not that it renders as intended.
Keyboard traversal, screen-reader output, and contrast in both themes are
unmeasured for this route and remain under the existing
`FINAL_V0_UI_ACCESSIBILITY_QUALIFICATION` external gate.
