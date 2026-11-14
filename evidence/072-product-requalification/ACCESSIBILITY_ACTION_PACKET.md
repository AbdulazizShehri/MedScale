# Spec 072 Rebuilt-Product Accessibility External Action Packet

This packet is required before any WCAG or final assistive-technology product claim. Repository tests prove engineering invariants only and do not satisfy this external measurement gate.

## Bound candidate prerequisites

Record the exact release-candidate commit/tree, package digest, OS/build, display scaling, locale, keyboard layout, screen-reader name/version, and whether the package is signed or unsigned. Use synthetic/permitted data only.

## Final route inventory

Qualify every final native route: Home, Patients, Documents, Insights, Models, Evidence, Workflows, Tasks, Messages, Audit Trail, Exports, Integrations, Settings, and About, plus the command palette and patient/insight sub-tabs.

## Required observations

1. **Keyboard-only traversal** — record focus order, visible focus, activation, escape/recovery behavior, and traps for every route and actionable control.
2. **Screen readers** — on macOS VoiceOver, Windows NVDA, and Linux Orca (or explicitly document unavailable platform coverage), record names, roles, selected/current state, status announcements, reading order, and actionable-control announcements.
3. **Models** — verify keyboard/screen-reader access to the signed local Pack directory input, Admit local Pack, Refresh, inventory rows, qualification-reference rows, trust/runtime/digest/promotion text, empty/error/refusal states, and the distinction between session inventory and qualification reference.
4. **Evidence** — verify the pinned OpenMed baseline, 39/39 accounting summary, claim-state pills, MedScale/OpenMed evidence text, evidence references, and limitations are announced in a usable order. Confirm `UNMEASURED`, `STRUCTURAL ONLY`, and `ANTI-METRIC` states do not depend on color alone.
5. **Text/reflow** — inspect at 200% effective text/display scaling and the minimum supported native window; record clipping, overlap, inaccessible horizontal content, and loss of controls.
6. **Rendered contrast/focus** — verify final rendered foreground/background combinations, status text, focus indicators, error/refusal states, and disabled states. Repository token-floor tests remain engineering evidence only.
7. **Recovery states** — exercise loading, empty, error, conflict, unsupported, reconciliation-required, default-deny, invalid claim-ledger, Model Center inventory/admission refusal, and Evidence Center unmeasured states.
8. **Provenance/source access** — confirm keyboard and screen-reader access to source/evidence drill-down and that relevance, qualification reference, comparative evidence, and claim states are announced without clinical or comparative authority inflation.

## Required result artifact

Commit or attach a bound report containing commit/tree/package digest, environment matrix, per-check PASS/FAIL, discovered defects, remediation references, and an explicit conformance decision. Screenshots must contain synthetic/permitted data only. Until that artifact exists, `wcag_conformance_claimed=false` and accessibility `release_ready=false` remain mandatory.
