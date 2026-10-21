# MedScale

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/medscale-horizontal-white.svg">
  <img src="assets/brand/medscale-horizontal-black.svg" alt="MedScale" width="280">
</picture>

MedScale is a local-first clinical intelligence platform. A Rust Core owns every source, assertion, provenance record and authority decision. The desktop and command-line surfaces only present and request through it.

- **Local by default.** Records, vaults and models stay on the device. Network egress is denied unless an explicit, governed action allows it.
- **Evidence you can inspect.** Every statement links back to its source, its coverage and its review state. Unknown, conflicting and unsupported data are shown as such, never filled in.
- **Encrypted workspaces.** Vaults are SQLCipher-encrypted at rest and sealed on close, with recovery codes.
- **Local models, honestly labelled.** Models run offline through signed MedScale Packs and propose; people decide. Catalog listings are kept separate from downloaded, admitted, tested and qualified models.
- **One Core, several surfaces.** The CLI and the Tauri desktop ([`apps/desktop-tauri`](apps/desktop-tauri/README.md)) share the same Core semantics. The desktop is built for Windows and macOS; a Linux desktop waits on an upstream Tauri move off GTK 3 / `glib` 0.18 (see [external gates](docs/planning/EXTERNAL_GATES.md), `TAURI_GLIB_ADVISORY_F096_T01`). The CLI and Core stay supported on Linux. The Slint desktop (`crates/medscale-desktop`) is kept only as a legacy reference implementation.

> **Status: engineering preview, synthetic data only.** MedScale is not release-ready, not clinically validated and not a medical device. Real patient data is not authorized. See [external gates](docs/planning/EXTERNAL_GATES.md).

Design: [brand identity system](docs/brand/BRAND_IDENTITY_SYSTEM.md) (founder-approved Logo 12, [logo specification](docs/brand/LOGO_SPEC.md)) and [`DESIGN.md`](DESIGN.md). Desktop: [`apps/desktop-tauri`](apps/desktop-tauri/README.md). Product program: [productization](docs/planning/PRODUCTIZATION_PROGRAM.md).

## Working on MedScale (engineering and agents)

### Cursor: start here

Open the repository in Cursor and tell it to read [`CURSOR.md`](CURSOR.md) and continue MedScale from live repository truth. The repository contains standing founder authorization and an autonomous build queue; Cursor should not need routine founder decisions.

Read order:

1. [`CURSOR.md`](CURSOR.md)
2. [`AGENTS.md`](AGENTS.md)
3. [`docs/planning/START_HERE.md`](docs/planning/START_HERE.md)
4. [`docs/planning/BUILD_QUEUE.md`](docs/planning/BUILD_QUEUE.md)
5. [`docs/planning/MASTER_BUILD_PLAN_V2.md`](docs/planning/MASTER_BUILD_PLAN_V2.md)

### Active state

```text
PLAN = CANONICAL_V2
SPEC_000 = CLOSED_CANONICAL
CURRENT_EXECUTION_QUEUE = docs/planning/BUILD_QUEUE.md
PRODUCT_MATURITY = SYNTHETIC_FOUNDATION_NOT_RELEASE_READY
CURSOR_AUTONOMOUS_IMPLEMENTATION = AUTHORIZED_WITHIN_PLAN
REAL_PHI = NOT_AUTHORIZED
MESC_MUTATION = NOT_AUTHORIZED
PRODUCT_RUNTIME_EGRESS = DEFAULT_DENY
```

See the [whole-product review](docs/planning/WHOLE_PRODUCT_REVIEW_2026-09-09.md) and
[prioritized delivery plan](docs/planning/TRUSTED_V1_DELIVERY_PLAN.md) for current gaps and
follow-on work. Synthetic-only OpenMed comparisons are recorded in the [evaluation evidence](evidence/103-local-model-catalog/eval/) and the [v0.1.0 release-candidate notes](docs/product/RELEASE_CANDIDATE_2026-10-20.md). Those results do not establish clinical superiority.

### Engineering bootstrap (historical foundation)

- Spec Kit constitution: [`.specify/memory/constitution.md`](.specify/memory/constitution.md)
- Closed Spec 001 package: [`specs/001-rust-repository-speckit-bootstrap/`](specs/001-rust-repository-speckit-bootstrap/)
- Contributing: [`docs/engineering/CONTRIBUTING.md`](docs/engineering/CONTRIBUTING.md)
- Dependency direction: [`docs/engineering/DEPENDENCY_DIRECTION.md`](docs/engineering/DEPENDENCY_DIRECTION.md)

```powershell
cargo test --workspace --locked
cargo run -p medscale-cli --locked -- --version
cargo run -p medscale-cli --locked -- doctor
```

### Execution support

- [Cursor execution playbook](docs/planning/CURSOR_EXECUTION_PLAYBOOK.md)
- [Implementation authority](docs/planning/IMPLEMENTATION_AUTHORITY.md)
- [Decision defaults](docs/planning/IMPLEMENTATION_DECISION_DEFAULTS.md)
- [Definition of done](docs/planning/DEFINITION_OF_DONE.md)
- [External gates](docs/planning/EXTERNAL_GATES.md)
- [Source acquisition and copy plan](docs/planning/SOURCE_ACQUISITION_AND_COPY_PLAN.md)
- [v0 UI integration contract (historical; superseded by Specs 095–101)](docs/planning/V0_UI_INTEGRATION_CONTRACT.md)

### Canonical architecture and planning

- [Master Build Plan V2](docs/planning/MASTER_BUILD_PLAN_V2.md)
- [Spec Kit Master Roadmap V2](docs/planning/SPECKIT_MASTER_ROADMAP_V2.md)
- [Canonical Source Register V2](docs/planning/CANONICAL_SOURCE_REGISTER_V2.md)
- [OSS / Code Absorption Matrix V2](docs/planning/OSS_CODE_ABSORPTION_MATRIX_V2.md)
- [External baseline evidence matrix](docs/planning/OPENMED_PARITY_SURPASS_MATRIX_V2.md)
- [GLM 5.3 Reconciliation V2](docs/planning/GLM53_RECONCILIATION_V2.md)


## Engineering evidence and source records

MedScale is an independently branded product. Synthetic-only runtime comparisons are documented in the [evaluation evidence](evidence/103-local-model-catalog/eval/) and [release candidate](docs/product/RELEASE_CANDIDATE_2026-10-20.md), with their original comparator names and measured limitations retained in those technical records. Exact third-party model identities and required legal notices remain traceable under [`third_party/provenance`](third_party/provenance), the [identity and provenance audit](docs/brand/PRE_RELEASE_IDENTITY_AND_PROVENANCE_AUDIT.md), and the generated `NOTICE.md` and SBOM. Historical inter-project boundaries are recorded separately in the engineering specs.

## License

MedScale first-party source is licensed under the [Apache License 2.0](LICENSE). Crates remain `publish = false`; the source license decision does not imply release readiness or authorize restricted data, credentials, terminology, model assets, or partner integrations.
