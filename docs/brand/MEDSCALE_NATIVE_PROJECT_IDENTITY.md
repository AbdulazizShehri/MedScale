# MedScale — native product identity contract

MedScale has one canonical first-party identity: **MedScale**. Repository ownership is `AbdulazizShehri/MedScale`.

## Required user-visible behavior

1. Product title, splash, About, desktop window, install package and documentation introduction identify **MedScale** only.
2. First-party Rust modules, user-visible CLI descriptions, UI component names, test names and developer entry points must use names describing **MedScale features** rather than other projects.
3. Model selection and evaluation use MedScale-owned display names. Real model IDs, digests and evaluation identities remain verifiable in evidence storage, not mislabeled in user interfaces.
4. A generic `reference` comparison may be shown when substantively useful, but never promote the comparator to product branding, authorship or authority.
5. Core-owned state, input validation, patient-data restrictions, privacy, compatibility and required security checks are unchanged by visual renaming.
6. Any user-visible legacy name is a bug and should have a failing regression test before a fix is merged.

## Enforced checks

- `apps/desktop-tauri/src/identity.test.ts` checks the public README introduction, Tauri frontend, desktop application metadata and legacy Slint UI for superseded project labels and incorrect MedScale identity.
- `crates/medscale-desktop/src/product_intelligence.rs` tests native identity of actual model/evidence view-model output, not merely source text.
- `crates/medscale-contracts/tests/external_artifact_import_compatibility.rs` verifies that a canonical MedScale Rust API rename does not silently corrupt the legacy serialized record.
- Required repository CI applies to every PR before merging into protected `main`.

## Boundaries that must be resolved by engineering rather than cosmetic changes

A first-party, native-looking repository is **not a reason to invent authorship**. A third-party package remains attributed in its required license/notice, and any real package import, pinned model identifier, historical research measurement or release hash must be truthful. Retaining those records under `third_party/`, `evidence/` or historical specifications does not make their names first-party product features.

The existing model catalog still includes externally sourced records, and some tests consume a real pinned reference model. Replacing these with fully MedScale-owned implementation or distribution requires: (1) a verified local implementation/pack or permitted mirror, (2) accurate model/asset provenance, (3) unchanged or explicitly migrated integrity checks, and (4) independent tests. Until then, do not rewrite a model identity or signature and pretend it is MedScale-owned.

## Remaining identity closure work

- Audit every first-party package, executable, test path and generated metadata for dormant source-branded naming.
- Audit visible UI messages and captured CLI outputs in supported Windows, macOS and Linux test runs.
- Migrate compatibility-only public Rust symbols with versioned adapters when there are real consumers; do not silently change persisted doctor or release schemas.
- Decide whether the historical comparison workbench belongs in product UI or only in the internal engineering evidence archive.
- Keep contributor onboarding on a single MedScale README without advertising unrelated engineering histories as core product features.
- Before declaring native-identity completion, verify an automated user-visible zero-legacy-label scan and exact-head CI on protected `main`.

**Release remains blocked** until all independent product, privacy, clinical and distribution gates are qualified. A successful rename is neither clinical validation nor permission to process real patient data.
