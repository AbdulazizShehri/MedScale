# Migration — Spec 084 MedScale Hub Foundation

Storage schema v12 -> v13, additive, inside `begin/finish_migration(13)`:

| Table | Purpose |
|---|---|
| `hub_identity` | this vault's Hub role (at most one row) |
| `hub_invitations` | one-time invitations (token digest only) |
| `hub_devices` | enrolled devices (public key only) |
| `hub_nonces` | single-use handshake nonces |
| `hub_events` | per-Project hash-chained event log; `(device_id, seq)` unique for claimed submissions |
| `hub_links` | client links to a Hub |
| `hub_link_secrets` | client device secrets (never exported) |
| `hub_outbox` | client signed submissions and outcomes |
| `hub_mirror` | client copies of Hub events |

No existing table changes. Reopening is idempotent.

Backup: every family except `hub_link_secrets` and `hub_nonces` is
written. Restore of a v13 snapshot requires all seven Hub families as
arrays, replays events in chain order, restores open invitations as
revoked, and re-verifies every cross-row invariant; a v12 snapshot restores
with empty Hub tables. Rollback: there is no newer-schema guard in the
storage layer, so an older build opening a v13 vault leaves the Hub tables
untouched and unused; the supported downgrade is restoring a backup taken
before the upgrade.
