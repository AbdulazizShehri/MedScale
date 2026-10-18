# Spec 096 — Tauri desktop foundation and navigation

**Status:** `PREPARATORY_IMPLEMENTATION; CLOSURE_BLOCKED_BY_095`

**Authority:** 2026-09-30 founder decision and [migration record](../../docs/planning/TAURI_DESKTOP_MIGRATION_DECISION_2026-09-30.md)
**Start point:** local `e040e8d`, including three unpublished 095 documentation commits over remote `a1bf5dc`; rebind before merge qualification.

## Outcome

A separate synthetic-only Tauri 2 application hosts MedScale's React, strict TypeScript, Vite and Tailwind presentation foundation. It has one branded navigation system, responsive light/dark themes, a route-safe command palette, explicit unavailable states, minimal capabilities and a typed read-only IPC seam to the existing Rust authority path. Slint continues to work.

## Requirements

- **R096-01:** A separate package builds without changing or deleting the Slint binary. Frontend dependencies are locked, assets local, and shipped code has no CDN, telemetry or runtime network requirement.
- **R096-02:** The shell uses the approved paired-M, Inter, JetBrains Mono NL, first-party geometric icons and semantic light/dark tokens. All 25 current routes are reachable with visible focus at 1100×720.
- **R096-03:** A closed typed registry drives navigation and bounded palette matching. Unknown route/command IDs refuse without Core activity. Keyboard search, selection, Escape and focus return are tested. This palette navigates only.
- **R096-04:** Read-only named IPC commands expose actual Core-derived DTOs after bounded validation and existing Core Host authorization/session checks. No generic SQL, shell, file, network or Core dispatcher exists. If safe host reuse is not ready, return an explicit unavailable state; do not fabricate clinical rows.
- **R096-05:** Capabilities default deny. Document every permission, CSP/navigation/external-link policy and WebView cache/crash/storage limit. No broad filesystem, shell, process, HTTP or clipboard grant. Prototype remains synthetic-only pending privacy proof.
- **R096-06:** Native Tauri light/dark Home/shell frames at 1440×900 and 1100×720 are captured and inspected, bound to head/tree/build/package/platform/PNG digests. Browser/source views alone are insufficient.
- **R096-07:** Typecheck/build, route/IPC/security tests, dependency and license review, exact-range scope, six exact-head CI jobs and mandatory Jev/Alibaba OCR results or explicit zero-cost blockers are recorded before closure.

## Boundaries and dependency

No real PHI, model execution, live network, fake sign-in, MESC, Core rewrite, direct WebView vault access, fabricated records, release claim or Slint removal. Spec 097 owns Home/Patients/Evidence depth; 098 remaining route bodies; 099 Welcome/Access, accessibility and broad native/privacy matrix; 100 final audit. Spec 095 remains historical Slint brand evidence, draft and unmerged while required reviews are blocked. Reversible 096 preparation is authorized; canonical merge/closure waits for 095 and all R096 evidence. `RELEASE_READY=false`, `PRIVATE_DATA_READY=false`, `MULTI_CLIENT_RELEASE_READY=false`.
