# MedScale engineering contribution guide

MedScale is an Apache-2.0, maintainer-led community project. Review the root [CONTRIBUTING.md](../../CONTRIBUTING.md), [SECURITY.md](../../SECURITY.md), [DESIGN.md](../../DESIGN.md) and the [public qualification status](../qualification/) before changing Core code.

## Safe engineering workflow

- Work in a focused feature branch and open a Pull Request against protected `main`.
- Describe intended behavior and tests. Only synthetic examples/fixtures are authorized; do not publish real health data or secrets.
- Keep Rust Core as the single source of authority; no unauthorized product network calls or client-side claim elevation.
- Keep third-party licenses, actual source identities, dependencies and [provenance records](../../third_party/provenance/) accurate.
- Review the [dependency admission requirements](DEPENDENCY_ADMISSION_TEMPLATE.md) before adding packages or model resources.
- Required workflow status and evidence govern merges; do not bypass checks.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 scripts/check_public_repository_hygiene.py
```

The Tauri desktop has its own Cargo manifest; follow [desktop build instructions](../../apps/desktop-tauri/README.md) for frontend and native commands.

## Public repository boundary

Do not commit internal execution plans, founder scheduling, editor-specific agent playbooks, or strategy/roadmap working files. Keep public code, issue discussions, qualification evidence, user documentation and the minimum technical specs required for verification. The public repository policy check rejects internal plan paths.

Successful tests, unsigned preview installers or merged pull requests do not authorize clinical deployment or a product release.
