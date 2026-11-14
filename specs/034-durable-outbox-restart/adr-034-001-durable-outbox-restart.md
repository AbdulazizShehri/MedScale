# ADR 034-001 — Outbox restart qualification via audit-class persistence

## Decision

Qualify durable outbox READY_BASE with SyntheticVault restart fixtures over existing Spec 016 audit rows. Do not add a dedicated outbox store.

## Consequences

Doctor `outbox_restart_qualified=true`. Live NPHIES remains gated. EncryptedVault authority sync remains out of scope.
