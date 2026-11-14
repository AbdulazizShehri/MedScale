# ADR-026-001 — Synthetic trust root + Linux ReadyBaseMeasured sandbox

## Status
Accepted

## Context
Q09 requires Pack signer/anti-rollback and a real OS confinement apply path before admitting a CPU engine. Full multi-OS PLATFORM_QUALIFIED remains an EXTERNAL_GATES item.

## Decision
1. Require ed25519 signatures over a canonical Pack signing payload, verified against a MedScale-owned synthetic trust root (`synthetic-pack-trust-v1`).
2. Enforce anti-rollback on PackStore admission by pack epoch then version.
3. Introduce `OsSandboxQualification::ReadyBaseMeasured` and implement Landlock apply only on Linux for that qualification; keep Windows/macOS NotPlatformQualified; leave EXTERNAL_GATES OPEN.

## Consequences
- Existing unsigned fixture Packs must be re-signed or replaced.
- Windows/macOS hosts compile stubs but cannot claim measured confinement.
- Doctor can report Linux measured READY_BASE without multi-OS or release claims.
