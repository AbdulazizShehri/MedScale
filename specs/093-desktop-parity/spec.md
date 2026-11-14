# Spec 093 — Research OS Desktop parity (read-only slice)

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promotion date:** 2026-09-28
**Canonical base:** `41d2a9aad17d220ffa7798b84794913dbd4cb099` (Spec 092 closure merge)
**Target branch:** `spec/093-desktop-parity`

## Authority

- `RESEARCH_OS_V2_SPEC_IMPLEMENTATION_CONTRACTS.md` fixes the implementation
  order for every unit, including **G. native Desktop vertical slice**.
- Specs 084-091 each closed with the recorded residual "no Desktop
  surface".
- The Spec 092 qualification matrix classifies *CLI / Desktop parity for
  Research OS planes* as `MISSING` (repository-owned).
- The founder's standing directive requires continuing repository-owned,
  authorized residuals after the numbered queue, and
  `IMPLEMENTATION_AUTHORITY.md` remains active. This spec admits no
  dependency.

## Scope

A first Desktop parity slice that makes every Research OS plane added by
Specs 084-091 visible in the native Desktop through Core:

- a **Research OS** route (sidebar item, header, honesty panel, refresh)
  listing, for the active project, Hub links, Compute jobs, R workspaces,
  extension installs, huddles, Research Pack artifacts, institutional
  adapters and federation peers, each with an explicit state;
- a plane Core cannot read is shown as `unavailable`, never as empty;
  an empty plane says so;
- a view-model module (`research_os_workspace.rs`) with no storage,
  worker, transport or key code, unit-tested through `CliSession`;
- one Core read added for this purpose in Spec 090 (`adapter_list`).

## Explicitly not in this slice

- Desktop *actions* for these planes (submit, stage, publish, install,
  consent, send, export...): they stay in the CLI. Parity is therefore
  `PARTIAL` (read), recorded as such.
- Rendered UI screenshots or assistive-technology qualification (external
  gate `FINAL_V0_UI_ACCESSIBILITY_QUALIFICATION`).

## Completion rule

`CLOSED_CANONICAL` only after merge on a green exact head and recorded
post-main verification.
