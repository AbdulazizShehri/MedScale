# Spec 026 evidence summary

**Spec:** 026-pack-signer-os-sandbox  
**Claim:** READY_BASE (Trusted V1 Q09)  
**Date:** 2026-09-10

## Delivered

1. **Pack admission:** synthetic trust root `synthetic-pack-trust-v1` (ed25519 via `medscale-keys`); digests/rights/`sbom_ref` still required; signature over canonical `MEDSCALE_PACK_SIGN_V1` payload.
2. **Anti-rollback:** `PackStore::admit` rejects lower `pack_epoch` or equal-epoch lower version.
3. **OS sandbox:** `OsSandboxQualification::ReadyBaseMeasured` + Linux Landlock apply; Windows/macOS remain `NotPlatformQualified`.
4. **Doctor:** `pack_signer` / `os_sandbox` honesty (`linux_measured=true`, `platform_qualified=false`, `release_ready=false`).

## Fixtures

- `evidence/026-pack-signer-os-sandbox/fixtures/pack-fixture-signed-v1/`
- Updated `evidence/008-local-ai-capability-fabric/fixtures/pack-fixture-ner-v0/` with signer fields

## Admissions

- `docs/engineering/admissions/026-ed25519-dalek.md`
- `docs/engineering/admissions/026-landlock.md` (Linux-only link)

## Gates kept FALSE / OPEN

- RELEASE_READY, PRIVATE_DATA_READY, MULTI_CLIENT_RELEASE_READY
- EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` = OPEN
- Full multi-OS `PlatformQualified`
- Real model Pack admission / gated model terms
