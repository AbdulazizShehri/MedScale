# Spec 095 qualification evidence

**Status:** implementation and native visual matrix reviewed; final evidence head CI, normal merge and post-main qualification pending.

## Live base

- Repository main: `1e2b7d94e970256b38bda15fa91f62bc397e825a`.
- Main run `36480424698`: six required jobs passed.
- Specs 068–094 stay closed; draft PRs124/125 remain separate.
- New program: `docs/planning/PRODUCTIZATION_PROGRAM.md`.

## Requirement traceability

| Requirement | Implementation/evidence | Qualification status |
|---|---|---|
| R095-01/02 approved geometry, inversion, size/space | assets/brand SVGs, LOGO_SPEC, approved-identity provenance; brand_foundation_095 numeric/asset tests | proof sheet rendered at 1200×1120 and inspected; black/white mark inspected in eight actual native frames; small shell favicon exception documented |
| R095-03 free fonts/rights | Inter/JB Mono NL exact binary/license hashes, native typography admission, FONT_NOTICE; builder/verifier/ZIP refusal probes | official rights and exact bytes verified; CI Windows package passed and extracted NOTICE plus both font OFL texts are present |
| R095-04 tokens | theme.slint, components.slint, typography/brand/state docs, web reference | live-token text/focus/selection contrast guard passed; sampled light/dark/compact native pixels inspected |
| R095-05 eleven distinct states | StateBadge literal names/unique markers/accessible text; brand_foundation_095 tests | deterministic eleven-state mapping passed; partial/unknown/unmeasured rendered with distinct text/markers in native frames, other eight not visually sampled |
| R095-06 actual Home application | command-center.slint bound to existing patient and runtime VM, scrollable current route dock | route/Home truth guards passed; eight-frame Home/About review and two before/after findings in NATIVE_VISUAL_REVIEW.md |
| R095-07 controlled snapshots | visual_evidence.rs, capture-native-ui.py, quickstart | bounded argument/export tests implemented; eight actual Windows `Window::take_snapshot` PNGs with adjacent source/binary metadata |
| R095-08 qualification | deterministic current guards, this record, exact-range/final CI/rendered records | source candidate `7be197c` run `36637843428` passed six required jobs; final evidence commit/head, merge and post-main pending |

## Local checks performed

2026-09-29, Windows, Rust toolchain1.97.1:

- `rtk cargo fmt --all -- --check`: passed after forward formatting repair.
- `rtk git diff --check`: passed.
- `rtk proxy pwsh -NoProfile -File scripts/check-dependency-direction.ps1`: passed.
- Spec066 standard-library regression executable: 5 passed, including contrast measured from actual Theme values.
- Spec068 standard-library current brand/product binding executable: 5 passed.
- The two std-only tests were compiled with `rtk proxy rustc --edition2024 --test` and CARGO_MANIFEST_DIR pointing at medscale-core. This avoided a cold workstation workspace build; it is not a claim that cargo workspace tests ran locally.
- All16 SVGs parsed; exact inverse/runtime master consistency is additionally guarded by the new numeric tests. The proof sheet was genuinely rendered and visually inspected. Its geometry/Inter type/inverses/clear-space/small-scale samples had no clipping. This qualifies the proof layout only.

## Qualified native candidate and findings

- Branch source candidate `7be197cd2505aa1bb0537b7bfb0fd40defe27482`, tree `e72568c074c9aa0f1e4d065036e52bdedb79a5fd` passed all six jobs in run `36637843428`. The Windows package source is the PR merge ref `654fca938dd4fd9421531b227f108060ed9f7c00` with the same tree.
- Its verified portable ZIP SHA-256 is `aac7e1b2e854d9f2b219c4029b81a3422f6a267fbaa567584e9a571b01a96b35`; packaged desktop executable SHA-256 is `cd58993420bddd33c70cfee4819df2425ee84d55e3485a46364584b90471bcec`. The sidecar and manifest matched before capture.
- Actual native Windows 11 Slint frames cover Home light/dark at 1440×900 and 1100×720, standard/compact at minimum size, plus About light/dark at minimum size. Captures use scale 1, one fresh nonsynced synthetic vault per process. `NATIVE_VISUAL_REVIEW.md` records every inspected frame and precise limits.
- F095-01 Home badge/source collision and F095-02 About dark attribution hierarchy/contrast were found in a prior six-job-green native candidate and fixed forward. Before frames/metadata remain in `findings/`; current after frames/metadata remain in `captures/`.

## Scope and limits

No new Cargo dependency or lockfile mutation. No Rust Core/storage/network/contract authority source changed. Current presentation regression guards move forward under new founder authority; closed spec/evidence artifacts are not rewritten. Minimal font assets are explicitly admitted and their redistribution gap is repaired. Native capture is opt-in, isolated synthetic-only and refuses existing outputs.

The rendered source candidate predates the evidence-only commit that will add these screenshots and records; the final exact-head CI must qualify that later commit. No WCAG, release/privacy/multi-client readiness, clinical model qualification or real-PHI claim is made. Full launch UI freeze remains owned by100.
