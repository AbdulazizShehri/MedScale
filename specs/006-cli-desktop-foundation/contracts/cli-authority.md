# Contract: CLI Authority Path (Spec 006)

**Spec**: 006-cli-desktop-foundation  
**Status**: Message/API sketch for implement (extends Spec 002 authority facade + Spec 003/004/005 capabilities)  
**Scope**: Synthetic-only; CLI + Desktop scaffold clients; in-process Core Host; no direct DB; no Tauri; no REAL_PHI; no network

## Roles

| Role | Spec 006 binding |
|---|---|
| `CoreHost` | In-process owner of EncryptedVault connection + active keys for the CLI/Desktop session |
| `Client` | `medscale-cli` / `medscale-desktop` — envelope callers only |
| `TransientHostOwner` | CLI process may own the host for the duration of a command/session |

Workers never receive vault keys or ambient DB. CLI has **no** privileged capability set beyond Desktop/SDK.

## Hard rules

1. **No direct DB open** from `medscale-cli` or `medscale-desktop` (no rusqlite/SQLCipher/EncryptedVault open APIs in those crates).
2. **No key material** in CLI argv logs, doctor output, or help text.
3. **Same envelopes** as Spec 002: `AuthorityRequest` / `AuthorityResponse`.
4. Unknown capability → `AuthorityError::Unauthorized`.
5. Sync/remote path → `SyncRootRefused` / `PathOutsideClaim` (Spec 005).

## Capabilities exercised by CLI wedge

```text
Capability::CreateEncryptedVault
Capability::OpenEncryptedVault
Capability::CloseEncryptedVault          // if exposed
Capability::UnlockWithPassphrase
Capability::UnlockWithRecoveryCode       // optional CLI surface
Capability::IngestFhirSynthetic          // Spec 003 name as implemented
Capability::GetTimeline
Capability::GetBrief
Capability::GetCoverage
Capability::RebuildProjection            // as needed before get
Capability::ResolveDefaultVaultPath
// Doctor aggregation may call read-only status helpers — still via core, not raw DB
```

Desktop scaffold may call a subset (version/doctor smoke/facade ping) without implementing full wedge UX.

## CLI session sketch

```text
CliSession::start() -> InProcess CoreFacade + optional vault open
CliSession::dispatch(AuthorityRequest) -> AuthorityResponse
CliSession::end() -> close lease / zeroize unlock material
```

Passphrase input: env var or interactive prompt policy documented at implement; never echo; never log.

## Forbidden

- `medscale-storage` EncryptedVault types used as public CLI API
- Embedding SQLCipher PRAGMA keys in CLI config files
- Network client crates for product path
- Tauri command bridge that re-opens vault outside Core Host

## Error mapping (CLI edge)

| AuthorityError class | CLI exit |
|---|---|
| Unauthorized / WrongScope | 2 |
| SyncRootRefused / PathOutsideClaim | 3 |
| LeaseHeld / AlreadyHeld | 4 |
| MissingKeyMaterial / unlock fail | 5 |
| NotFound | 6 |
| Other typed failures | 1 |

Human messages must be non-secret. `anyhow` may wrap at the binary edge after typed mapping.

## Tests required

- Dependency/lint or architecture test: cli/desktop crates do not call EncryptedVault::open / rusqlite::Connection::open
- Capability parity: CLI-only superuser capability does not exist
- Negative: wrong passphrase → typed fail; no secret in stderr
