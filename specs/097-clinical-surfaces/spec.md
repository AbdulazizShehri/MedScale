# Spec 097 — Clinical presentation surfaces

**Status:** `PREPARATORY_IMPLEMENTATION; CLOSURE_BLOCKED_BY_096`

**Authority:** [2026-10-01 founder amendment](../../docs/planning/TAURI_PATIENTS_PRIORITY_2026-10-01.md).
Base: verified Tauri head `c585f84ca433e9ee1e2f72777d66525c79e27267`.

## Requirements

- R097-01 Preserve the approved native shell; extend one sidebar and local
  first-party identity rather than replacing its composition.
- R097-02 Build a compact Patients Index with reusable patient rows. Render only
  governed records; distinguish unavailable/denied/unknown from an empty
  successful query. No general roster is assumed.
- R097-03 Centralize recurring icon, status, header, navigation, table, row,
  section, button, search and empty-state presentation with semantic tokens.
- R097-04 Build dark and light together. Native targets are 1440×900 and
  1100×720, with an intermediate resize, visible focus and intentional scrolling.
- R097-05 Patient Detail and Evidence follow the supported Index. No invented
  identity, condition, review state, timestamp, literature or authority claim.
- R097-06 Any native read uses named typed Rust commands, verified owned
  synthetic Host context and existing Core authorization. No generic dispatcher,
  file/shell/SQL/network bridge, automatic private-vault attachment or promotion.
- R097-07 Bind tests, native evidence and reviews to exact heads; preserve Jev,
  Alibaba OCR, advisory, predecessor and privacy gates. No release/PHI claim.

## Initial bounded slice

The existing Tauri status command remains unchanged. Patients gets a route
composition and reusable primitives, but no records while the owned Core adapter
and roster capability are absent. A successful shell diagnostic is not a
successful patient query. The patient total stays unknown, not zero.

Patient Detail data and Evidence migration remain pending actual domain wiring.
This slice does not manufacture sample rows to qualify the row design.
