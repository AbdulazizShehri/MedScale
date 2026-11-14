# Spec 054 — Native / Full Release SBOM Qualification (READY_BASE)

## What this is
Deterministic CycloneDX 1.5 release SBOM (`SBOM.cdx.json`, 212 components)
generated from the actual source state via `scripts/generate-release-sbom.ps1`,
plus a Rust generator/verifier (`medscale-core::release_sbom`) with positive
and negative tests (`crates/medscale-core/tests/release_sbom_054.rs`).

## Binding (at generation time)
- source_sha = 7763fc8ad2787ccd04cf8b7ac7ca2f7c299524a1 (parent main at generation)
- tree_sha = 1481f17641dbf6915e389b44ca3abcba0fab502a
- cargo_lock_sha256 = 7d6f1597db5419c07aee3a3f91cc7e28ee30a57e011bb98d34fee6fddf2d3775
- doc sha256 recomputed by the verifier on every check; tamper/stale/missing
  inputs fail closed (7 negative tests).

## Honesty limits (READY_BASE, not release)
- `SBOM_DOCUMENT_FORMAT = CycloneDX-1.5`; `PUBLIC_PROJECT_LICENSE = UNDECIDED
  (external legal decision)`. Workspace crates record
  `UNLICENSED-workspace-no-public-license`; unknown third-party licenses record
  `NOASSERTION` — never fabricated.
- `REPRODUCIBLE_BUILD = UNPROVEN`; `release_ready = false`.
- The committed snapshot binds the state at generation time; the release flow
  regenerates the document at tag time (a commit always moves HEAD by design).
  The verifier rejects stale digests, so staleness is detectable, not silent.
- Remaining in `missing_evidence_classes`: `release_sbom_signing_provenance`
  (signing identity external), reproducible package contents, real installer
  proof, SPDX choice, branch protection, budgets, WCAG/final UI.

## Regeneration log
- 2026-10-07: regenerated with `scripts/generate-release-sbom.ps1` (unchanged
  generator) after the `medscale-desktop-vm` workspace crate was added on the
  Tauri desktop branch (PR #177). The previous snapshot (212 components)
  predated that crate and the dependency additions on this stack, so
  `committed_document_is_self_consistent` failed with
  `workspace package missing: medscale-desktop-vm`. The document now lists
  799 components and binds:
  - source_sha = 7686435beb42712c3dced097b0914c8725a1b8bc (parent at generation)
  - tree_sha = 46318dd3379904bd855cffe04b8c616941425104
  - cargo_lock_sha256 = c12df3b452d6fd1dcb63078340c06a891487c425a46fe44aa3c0847cad432402
  Honesty limits above are unchanged: READY_BASE only, `release_ready = false`.
