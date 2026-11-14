# Spec 094 — Research OS Desktop row actions (reversible slice)

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promotion date:** 2026-09-28
**Canonical base:** `4dfc8ada9d88df4fc818cc49b25f3f6ec278b1b2` (Spec 093 closure merge)
**Target branch:** `spec/094-desktop-actions`

## Authority

- `RESEARCH_OS_V2_SPEC_IMPLEMENTATION_CONTRACTS.md` step **G. native
  Desktop vertical slice**: a vertical slice includes acting, not only
  reading.
- Spec 093 closed the Desktop *read* gap for Specs 084-091 and recorded
  "Desktop actions stay in the CLI" as a repository-owned residual (matrix
  row 28 `PARTIAL`).
- `IMPLEMENTATION_AUTHORITY.md` remains active; the founder's standing
  directive requires continuing authorized repository-owned residuals.
  This spec admits no dependency.

## Scope

Row actions on the Desktop **Research OS** route, each through Core with
the same capability and receipts as the CLI:

- Compute: **Cancel** a `queued` or `running` job;
- Extensions: **Enable** / **Disable** an installed extension;
- Institutional adapters: **Suspend** / **Resume**.

Rule for admission to Desktop in this slice: the action is reversible or
stops work, and moves no data across a boundary. A row offers at most one
action, derived from its plane and state; anything else is refused before
Core is asked. After an action the route refreshes and states the outcome
(done with the resulting state, or not done with Core's refusal).

## Explicitly not in this slice

- Terminal or data-moving actions: adapter revoke, send, reconcile and
  retry; federation trust, revoke, export and import; extension install,
  upgrade, grant and uninstall; R stage, launch and publish; huddle
  consent, attach, transcribe, propose and review; Research Pack acts.
  They stay in the CLI; Desktop parity remains `PARTIAL`.
- Rendered UI and assistive-technology qualification
  (`FINAL_V0_UI_ACCESSIBILITY_QUALIFICATION`).

## Completion rule

`CLOSED_CANONICAL` only after merge on a green exact head and recorded
post-main verification.
