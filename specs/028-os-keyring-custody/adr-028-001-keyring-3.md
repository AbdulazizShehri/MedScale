# ADR-028-001: Admit keyring 3.6.3 for OsKeyStore

## Status

Accepted (Spec 028 READY_BASE)

## Context

Q03 PRIVATE_DATA_READY path requires OS key custody beyond MemoryMock. Workspace rust-version is 1.85; keyring 4.x requires 1.88.

## Decision

- Depend on crates.io `keyring` **3.6.3** with features `windows-native`, `apple-native`, `linux-native`.
- Implement `OsKeyStore: KeyStore` using binary secrets.
- Prefer OS at runtime when probe succeeds; Memory fallback otherwise.
- Do **not** set `PRIVATE_DATA_READY=true`.

## Consequences

- Windows CI can exercise real Credential Manager.
- Linux CI may fail soft without keyutils; FakeOsKeyStore covers API tests.
- Jumping to keyring 4.x requires an explicit rust-version ADR.
