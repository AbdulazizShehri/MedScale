# Final Accessibility External Action Packet

This packet is required before any WCAG or final assistive-technology product claim. Repository tests do not satisfy it.

## Preconditions
- Use the exact signed/unsigned release candidate commit and package digest being qualified.
- Record OS version, display scaling, locale, keyboard layout, screen reader name/version, and package digest.
- Use synthetic/permitted data only.

## Required observations
1. Keyboard-only: traverse every top-level route, patient tabs, assistant input/actions, workflow review controls, command palette, Documents, Audit, Exports, Settings, and Integrations. Record focus order, visible focus, activation, escape/recovery behavior, and any traps.
2. Screen reader: on macOS VoiceOver, Windows NVDA, and Linux Orca (or explicitly document an unavailable platform), record names, roles, selected/current state, status announcements, table/list reading order, and actionable-control announcements.
3. Text/reflow: inspect at 200% effective text/display scaling and the minimum supported native window. Record clipping, overlap, inaccessible horizontal content, and loss of controls.
4. Contrast: verify final rendered foreground/background combinations, including focus indicators and status text. The repository 4.5:1 token regression is engineering evidence only.
5. State recovery: exercise loading, empty, error, conflict, unsupported, reconciliation-required, and default-deny states. Confirm meaning does not depend on color alone.
6. Provenance/source access: confirm keyboard and screen-reader access to source/evidence drill-down and that relevance/coverage states are announced without clinical-authority inflation.

## Required result artifact
Commit or attach a bound report containing commit/tree/package digest, environment matrix, per-check PASS/FAIL, screenshots only if they contain synthetic data, discovered defects, remediation references, and an explicit conformance decision. Until that artifact exists, `wcag_conformance_claimed=false` and accessibility `release_ready=false` remain mandatory.
