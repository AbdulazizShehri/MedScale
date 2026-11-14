# ADR-022-001: Release qualification prep without RELEASE_READY

## Status

Accepted (Spec 022)

## Context

Trusted V1 Q05 needs locked builds and evidence artifacts after immutable Action pins. Claiming `RELEASE_READY` would be false: OS matrix incomplete, branch protection external, license/signing/SBOM packaging incomplete.

## Decision

Ship Spec 022 as **READY_BASE prep only**: `--locked` CI/docs, evidence binding procedures, doctor `release_qualification` axis with `release_ready=false` and missing evidence classes, EXTERNAL_GATES row for owner-managed branch protection. Renumber deferred advanced roadmap work to **023+** so Spec 022 owns this prep unit.

## Consequences

- Operators see honest FALSE readiness.
- Autonomous Trusted V1 path may become external-gate-only after 022 closes (MESC, branch protection, license, etc.).
- No GitHub settings mutation from Cursor.
