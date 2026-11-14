# DESKTOP QUALIFICATION — Spec 074 (074-E, T074-09)

## Binding

```text
BASE_SHA=ee8daef3a2782bbdbcb3324766a5d6b95c09fa09
BRANCH=spec/074-project-artifact-graph-foundation
BINARY=target/debug/medscale-desktop.exe (local debug build, Rust 1.97.1)
HOST=Windows 11 x64 (founder workstation, live session)
CAPTURE=UIAutomation-bound window capture + CopyFromScreen (PrintWindow returns
  black for GPU-composited Slint frames; documented, not hidden)
VAULT=%LOCALAPPDATA%\MedScale\vaults\desktop-projects (synthetic, created on first run)
REAL_PHI_USED=false
```

## Surface

New module `crates/medscale-desktop/src/project_workspace.rs` (view-models,
typed status mapping, input validation, unit tests) + `Projects` nav/route in
`ui/app.slint` (structs, properties, list/detail/experiments/refs/relationship
inspector, create/archive/refresh actions) + session/action wiring in
`src/main.rs`. Desktop uses the operator `CliSession` plus a synthetic vault
at the Core-owned default root; every read/mutation goes through the same
`CliSession` helpers as the CLI (Core parity by construction).

Workflow: Projects -> list (empty/loading/denied/conflict/missing/stale/
corrupt/unavailable states via typed status line) -> create/open -> detail
(overview, experiments, artifact references with resolutions, typed
relationship inspector) -> create experiment -> archive. No decorative
network graph; the inspector is a typed edge list.

Keyboard/focus/a11y: existing `NavItem`/`QuickAction`/`ToolbarAction`/
`AdaptiveLineEdit`/`StatusPill` components reused unchanged (focus,
Ctrl/⌘+K palette, Esc, minimum 1100x720, light/dark `Theme` tokens);
every new group carries `accessible-role` + `accessible-label`.
No WCAG conformance claimed.

## Rendered evidence

```text
PROJECTS_LIGHT.png (SHA256 B95120A9...) — Projects route, light, empty state
PROJECTS_DARK.png (SHA256 B7E0F025...) — Projects route, dark, empty state
PROJECTS_POPULATED.png (SHA256 1639069A...) — light, "1 project · Core-backed"
logs/desktop-projects-uia.txt — UIAutomation tree dump of the populated run:
  "Desktop Parity Proof, active, revision 1", "proj-1 · rev 1",
  "experiments 0 · refs 0 · edges 0", "Open. Detail"
```

Parity proof: `proj-1` was created by the CLI
(`medscale project create --vault-id desktop-projects ...`) and rendered by
Desktop from the same vault file — one Core authority serves both surfaces.
(Populated rows render below the taskbar fold at 125% DPI in a headless
capture; the UIA dump binds their exact content. No finding hidden.)

## Gates

```text
cargo fmt --all -- --check => PASS
cargo clippy -p medscale-desktop --all-targets --locked -- -D warnings => PASS (slint compiles)
cargo test -p medscale-desktop --locked => PASS (27 + 3 incl. 4 project_workspace tests:
  typed status mapping, name validation, endpoint labels, resolution labels)
medscale-desktop --smoke => PASS
```

Limitation: pixel captures cover empty states + populated status; populated
row pixels are bound via the UIA tree dump instead (busy-session occlusion).
Interactive click-through (Open/Archive buttons) is wired to the same tested
`CliSession` paths as the CLI slice; no separate Desktop-only authority exists
(`surface_bypass_not_present` holds by dependency direction + shared helpers).
