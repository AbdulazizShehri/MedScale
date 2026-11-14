# Findings — Spec 097

## Initial bounded Patients implementation

Shared sidebar, navigation, icons, header, status, search, table, row, empty state,
section, button and runtime state are extracted. The first-party outline family
uses one 24-unit grid, 18px rendering and two-unit strokes. Tabs will be added
with the supported Patient Detail slice; no inactive tab control is invented.

The Patients view has four prospective supported columns, an unknown total,
disabled search, compact unavailable row and an inline explanation. The reusable
PatientRow is not populated: no roster query has run and Sarah's Slint display
fixture is not imported. No Core, Rust command, capability, CSP or dependency
change is included in this slice.

Local verification: six frontend tests pass; strict TypeScript and Vite build
pass. The first build caught the missing search/commands icon union, repaired
before the successful build. No skipped test or native success is inferred.

Browser inspection covered Patients in both themes at 1440×900 and 1100×720,
keyboard Enter opening/closing the availability explanation, Ctrl+K opening the
palette and Escape restoring focus to its trigger. At 1280×800 the table is 956px
wide with no page-wide horizontal overflow. Home retains one sidebar and a
compact workspace notice. These are development observations, not native final
evidence or assistive-technology qualification.

Native Windows frames and exact build receipt are now under
`evidence/097-clinical-surfaces/native/4641cbb`: both themes at the two configured
sizes plus 1272×900 inferred intermediate client width, with original capture
hashes. All 1,027 package inventory entries verified. Three native platform build,
Clippy/test/package jobs passed in 36830696375; dependency policy failed glib.
No macOS/Linux visual evidence follows from those builds. The source detector
ran once and reported only three warnings for pinned Inter; retain the approved
brand. The fresh finish reviewer returned fix at presentation-preparation scope:
the keyboard-focusable table region lacked an authored monochrome focus ring.
Added the existing 2px focus token with an inset outline to avoid scroll-region
clipping. Corrected source `7146551` subsequently passed all three native
build/test/package jobs in 36834231752. Its 1,027 Windows package entries were
verified before launch. Twelve original native frames, including three focus
proofs, are recorded under `evidence/097-clinical-surfaces/native/7146551`.
The same fresh reviewer resolved the scored focus fix with no introduced
regressions observed. Its ship disposition covers only this correction.
Canonical run 36834231751 had five successful jobs; its remaining Windows Rust
job was later cancelled by workflow concurrency after a documentation push.
No complete canonical CI success is claimed. Subsequent documentation heads
have their own pending CI and do not relabel the 7146551 executable evidence.

Open: full exact-head canonical CI; populated row; governed Core
adapter; Patient Detail/Evidence; full provider/dependency/privacy qualification.
Jev and provider-backed Alibaba OCR are BLOCKED_BY_ZERO_COST. The inherited glib
advisory policy failure is unresolved. No merge or CLOSED_CANONICAL claim.
