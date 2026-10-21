# MedScale — comprehensive first-party rename directive (2026-10-21)

**Founder instruction (approved):** All MedScale-owned product surfaces and first-party implementation components must use MedScale naming. The founder states that he has full legal authorization to copy, adapt, use, and rename donor source code, including OpenMed-derived implementations and other permissioned sources. Do not demand disclosure of confidential grant documents as a prerequisite to the naming task.

**Release baseline:** `main` at `4676924` passed `ci`, `tauri-preview`, and `desktop-e2e`. Rename work is a separate scoped release delta, not an assertion that those pre-rename binaries contain the updates.

## Required inventory and migration

Audit the *whole tracked repository*, not only desktop JSX. The initial 2026-10-21 inventory found 429 tracked files with `OpenMed`, `MESC`, `EverHealth` or `MedOrigin` strings. This is a search inventory, **not** evidence that all 429 are defective; sort every occurrence into the categories below.

| Category | Required disposition | Verification |
|---|---|---|
| Product name, help, desktop/welcome/about, model catalog, E2E fixtures, icons, screenshots, installers, user docs and marketing | MedScale only; eliminate competitor branding | exact frontend/CLI/package scans, native screenshots |
| First-party namespaces, adapters, identifiers, internal files and Rust modules derived from permissioned implementations | Rename to meaningful MedScale-owned names; migrate callers, exports, tests and build scripts together | `cargo test --workspace --locked`, clippy and integration tests |
| Third-party model listings and model IDs | Use unambiguous MedScale catalog *display aliases*, but keep functional upstream IDs for real downloads until a genuinely available authorized MedScale mirror is created and validated | model lookup and download replay; no dead URLs or fake ownership claims |
| Wire/schema keys and previously persisted IDs | Renames require explicit backward-compatible readers, migrations and tests; never simply substitute the string and orphan user data | migrate-from-v0.1.0 fixture, restart, CLI compatibility |
| Immutable third-party code/data copies, exact benchmark inputs, scientific baselines, hashes and pin metadata | Preserve byte-integrity and original research truth until safely migrated to new pinned copies; any new MedScale labels must reflect real assets, not fabricated external identities | digest verification, benchmark reproducibility |
| Legal/attribution records and historical evidence | Keep original author attributions and legally required notices in compliance records, independently of the MedScale brand; do not rewrite historical results as if MedScale authored them | release NOTICE, LICENSE, SBOM, source archives |

## Execution sequence

1. Resolve the live GitHub frontier and re-audit every first-party product surface and build artifact.
2. Finish all MedScale-facing names and stable display aliases with no OpenMed/MESC brand visible in ordinary app screens.
3. Rename safe first-party internal namespaces/modules/filenames in reviewable batches. Preserve functional contracts and compatible model-download IDs. Record renamed-path-to-old-path mappings.
4. Where the desired name requires a new hosted repository or weights mirror, label that work `REQUIRES_REAL_MIRROR`, retain a working existing identifier, and do not pretend a renamed URL is valid.
5. Run per-batch tests and exact-head required CI, Windows installer, macOS installer, and real Windows desktop E2E. Check logo 12, UI labels, metadata, About and CLI.
6. Merge only after all required checks pass. Obtain new release artifacts and SHA-256 from the new commit; earlier installers are superseded for branding purposes.
7. Output a full name-inventory report with before/after paths, semantic exceptions, remaining blockers and final release gates. Do not claim complete rename without verified scans.

The rename authorization is granted. Third-party provenance and licensing are records of underlying facts rather than MedScale product branding. The aim is a genuinely independent MedScale product, not broken dependencies, misleading model ownership, rewritten research evidence, or a falsely claimed production/clinical release.
