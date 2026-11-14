# Contracts: Spec 001 Bootstrap

## CLI surface (bootstrap)

```text
medscale --version
  → prints workspace version string from medscale-contracts

medscale doctor
  → prints non-PHI bootstrap health: toolchain channel label, workspace name,
    and explicit notice that vault/network/packs are not yet implemented
```

## Crate API stubs

### medscale-contracts

- `MEDSCALE_VERSION: &str` — semver workspace version
- `WorkspaceIdentity` — struct with `version` field only

### medscale-core

- `CoreFacade::bootstrap_report() -> BootstrapReport` — confirms authority stub is local-only; no storage/keys/network handles

## Forbidden in 001

- Opening databases, reading PHI paths, network sockets, model loads, FHIR parsing
