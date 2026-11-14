# Feature Specification: Rust Repository + Spec Kit Bootstrap

**Feature Branch**: `spec/001-rust-repository-speckit-bootstrap`

**Created**: 2026-08-25

**Status**: Ready for implementation

**Input**: Bootstrap MedScale into a reproducible Rust/Spec Kit engineering repository: Spec Kit governance, minimal dependency-directed crate skeleton, CI/format/lint/test/supply-chain evidence, contribution conventions, architecture-direction checks, and a complete Spec 002 planning package. No medical, model, network, or PHI functionality.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Spec Kit Governance Materialized (Priority: P1)

An engineer opening the repository finds Spec Kit installed conventions, a constitution that matches founder-frozen MedScale invariants, and a complete Spec 001 package that can drive implementation without reopening product architecture.

**Why this priority**: Without governance and Spec Kit structure, autonomous execution has no binding process authority.

**Independent Test**: Inspect `.specify/memory/constitution.md` and `specs/001-*/` for complete constitution + Spec Kit artifacts aligned with `AGENTS.md` / `CURSOR.md`.

**Acceptance Scenarios**:

1. **Given** a clean clone, **When** an engineer reads the Spec Kit constitution, **Then** it restates Rust-owned authority, local-first/privacy-first, source/authority class discipline, evidence-before-claims, fail-closed safety, and MESC/PHI/network constraints without weakening them.
2. **Given** Spec Kit bootstrap artifacts, **When** the Spec 001 package is reviewed, **Then** `spec.md`, clarification closeout, `research.md`, `plan.md`, contracts, `quickstart.md`, checklists, and `tasks.md` are present and internally consistent.

---

### User Story 2 - Reproducible Rust Workspace Skeleton (Priority: P1)

An engineer can build and test a minimal Rust workspace with pinned toolchain and lockfile, establishing dependency direction for contracts → core → CLI without medical behavior.

**Why this priority**: All later specs depend on a working Rust workspace and ownership boundaries.

**Independent Test**: `cargo build` / `cargo test` succeed on the declared development platform; CLI prints version; no medical/model/network product behavior exists.

**Acceptance Scenarios**:

1. **Given** pinned toolchain and `Cargo.lock`, **When** `cargo test --workspace` runs, **Then** all crates build and tests pass.
2. **Given** the workspace crate graph, **When** dependency direction is inspected, **Then** app/CLI depends inward on contracts/core and no speculative empty medical crates exist.

---

### User Story 3 - CI, Supply-Chain, and Evidence Gates (Priority: P1)

CI runs format, lint, test, and supply-chain policy checks and archives evidence for the exact commit under test.

**Why this priority**: Definition of Done forbids PASS claims without exact-head evidence.

**Independent Test**: CI workflow files exist; local equivalents of rustfmt/clippy/deny/vet produce archived evidence under `evidence/`.

**Acceptance Scenarios**:

1. **Given** a PR changing Rust code, **When** CI runs, **Then** format, clippy (deny-level policy), tests, and declared supply-chain checks execute.
2. **Given** a successful local qualification, **When** evidence is archived, **Then** it records commit, toolchain, lock identity, platform, commands, and results.

---

### User Story 4 - Contribution and Dependency Admission Conventions (Priority: P2)

Contributors follow documented commit/PR/evidence conventions and a dependency admission template before adding load-bearing dependencies.

**Why this priority**: Prevents uncontrolled dependency and review drift as the repo grows.

**Independent Test**: Docs and templates exist and are referenced from README/CONTRIBUTING.

**Acceptance Scenarios**:

1. **Given** a proposed new dependency, **When** the admission template is applied, **Then** owner-spec, pin/lock, license/security/transitives, and exit strategy fields are required.
2. **Given** contribution docs, **When** a PR is opened, **Then** reviewers have an explicit checklist covering constitution, evidence, and no-medical-scope for Spec 001.

---

### User Story 5 - Spec 002 Package Ready (Priority: P1)

Before Spec 001 closes, a complete Spec 002 planning package exists and passes consistency analysis so execution can continue immediately.

**Why this priority**: Autonomous progression requires the next unit to be READY at 001 closeout.

**Independent Test**: `specs/002-*` contains complete Spec Kit artifacts with no unresolved blocker contradictions relative to the roadmap.

**Acceptance Scenarios**:

1. **Given** Spec 001 implementation complete, **When** Spec 002 package is analyzed, **Then** no unresolved blocker remains that would prevent marking 002 READY after 001 merge.
2. **Given** Spec 002 scope, **When** anti-scope is checked, **Then** H0 medical ingest/presentation remain owned by 003/004, not 002.

### Edge Cases

- Spec Kit tool unavailable in CI: document pin and install from recorded upstream tag; do not require network at product runtime.
- Platform matrix incomplete on first PR: Windows is the current development host; declare Linux CI as required when runners are available; do not claim multi-OS PASS without evidence.
- Accidental medical/model dependency introduced: supply-chain and review gates MUST fail closed.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Repository MUST materialize Spec Kit structure and constitution from canonical MedScale invariants without reopening founder decisions.
- **FR-002**: Repository MUST provide a complete Spec 001 Spec Kit package and implementation handoff.
- **FR-003**: Repository MUST provide a minimal Rust workspace with pinned toolchain and reproducible lock strategy.
- **FR-004**: Workspace MUST include only crates needed for bootstrap dependency direction (contracts, core stub, CLI), not speculative empty medical crates.
- **FR-005**: Repository MUST establish rustfmt, clippy deny policy, and workspace tests.
- **FR-006**: Repository MUST establish supply-chain baseline: cargo-deny, advisory policy, cargo-vet policy scaffold, SBOM/evidence path conventions, and secret-scanning guidance as appropriate.
- **FR-007**: Repository MUST provide CI workflows for the declared development platforms of this phase.
- **FR-008**: Repository MUST provide contribution/PR/evidence conventions and dependency admission template.
- **FR-009**: Repository MUST provide architecture/dependency-direction checks sufficient to keep future app/UI/worker code from owning trusted contracts.
- **FR-010**: Spec 001 MUST produce a complete Spec 002 planning package before closeout.
- **FR-011**: Spec 001 MUST NOT introduce medical functionality, models, OCR/ASR, product runtime network, real PHI, or MESC mutation.

### Key Entities

- **WorkspaceIdentity**: toolchain channel/version, `Cargo.lock` digest, crate graph.
- **EvidenceRecord**: commit/tree, platform, commands, results, limitations.
- **DependencyAdmission**: owner spec, pin, license, security, exit strategy.
- **SpecPackage**: Spec Kit artifacts for a numbered unit.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A clean checkout builds and tests the workspace with zero failing tests on the recorded platform.
- **SC-002**: Format and clippy gates are deterministic and fail on introduced violations.
- **SC-003**: Supply-chain policy commands run and produce archived evidence for the exact head.
- **SC-004**: Constitution, `AGENTS.md`, Cursor rules, and Spec Kit artifacts agree on authority and sequencing.
- **SC-005**: Spec 002 package exists with no unresolved analyze-blocker before Spec 001 closeout.
- **SC-006**: No product medical/model/network behavior is present in the Spec 001 deliverable.

## Assumptions

- Spec Kit CLI pin is `v1.0.1` (commit recorded in research).
- Rust stable toolchain currently available on the development host is acceptable to pin via `rust-toolchain.toml`.
- Initial CI targets GitHub Actions; Windows is proven locally; Linux job is included when feasible on GitHub-hosted runners.
- cargo-auditable and fuzz scaffolding may be policy/docs-first in 001 if full integration would expand medical-adjacent scope; record exact admission status.
- Ordinary tooling choices follow `IMPLEMENTATION_DECISION_DEFAULTS.md`.
