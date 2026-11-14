# Feature Specification: Perf Harness + Package/SBOM Checksum Evidence (Trusted V1 Q05 remnants)

**Feature Branch**: `spec/027-perf-sbom-release-evidence`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 022 CLOSED_CANONICAL READY_BASE (prep); Specs 016–026 CLOSED  
**Does not**: claim `RELEASE_READY`, budget attainment, signed packages, full release SBOM (native/model assets), `PRIVATE_DATA_READY`, or `MULTI_CLIENT_RELEASE_READY`.

## User Stories

### US1 — Deterministic performance harness (P1)
An operator or CI runs a lightweight timed harness over synthetic fixtures for a subset of Trusted V1 delivery-plan budgets (timeline projection, lexical search, FHIR ingest). The harness records p50/p95 methodology, hardware/toolchain notes, and explicitly states that targets are **not** claimed met. CI asserts only that the harness runs and reports numbers — never budget pass/fail.

### US2 — SBOM scaffold path (P1)
A documented script under `scripts/` generates a CycloneDX-like (or SPDX-like) JSON SBOM scaffold from `cargo metadata` / `Cargo.lock` for workspace crates, with committed example evidence and honest limitations vs a full release SBOM (native/model assets missing).

### US3 — Package checksum manifest (P1)
A script archives/lists workspace crate sources + `Cargo.lock` digests into an evidence checksum manifest (sha256). This is **not** a signed release package.

### US4 — Doctor honesty (P1)
`release_qualification` doctor axis reports `perf_harness_present`, `sbom_scaffold_present`, and `release_ready=false`, updating missing evidence classes for remaining release-bar gaps.

## Requirements

- **FR-001**: Deterministic `tests/perf_harness_027.rs` (or equivalent) with fixed warmup+N, JSON evidence under `evidence/027-perf-sbom-release-evidence/`; no CI fail on budgets.
- **FR-002**: Scripted SBOM scaffold generation + committed example or generate-in-evidence path; limitations documented.
- **FR-003**: Scripted package/source checksum manifest (sha256) under evidence.
- **FR-004**: Doctor fields `perf_harness_present`, `sbom_scaffold_present`; `release_ready=false`; missing classes remain non-empty for true release gaps.
- **FR-005**: Spec Kit package + evidence; BUILD_QUEUE / roadmap / START_HERE → Spec 027 CLOSED; deferred advanced work **028+**.

## Out of scope

Claiming delivery-plan budgets achieved; Criterion/dev-dep if deny-incompatible (prefer harness test); signed/notarized packages; full release SBOM with native/model assets; macOS/mobile release qualification; configuring GitHub branch protection; RELEASE_READY=true.

## Success Criteria

- Workspace fmt / clippy `-D warnings` / `cargo test --workspace --locked` PASS
- Perf harness reports numbers without asserting budget pass
- SBOM scaffold + checksum scripts produce evidence artifacts
- Doctor honesty: `release_ready=false`; harness/SBOM scaffold present flags true
- Queue/roadmap: Spec 027 CLOSED_CANONICAL READY_BASE; deferred **028+**
- No RELEASE_READY / PRIVATE_DATA_READY / MULTI_CLIENT_RELEASE_READY / budget attainment claim
