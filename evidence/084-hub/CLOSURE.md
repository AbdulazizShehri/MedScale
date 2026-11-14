# CLOSURE — Spec 084 MedScale Hub Foundation

## Terminal truth

```text
SPEC_084_CLOSED_CANONICAL=true
MERGE_SHA=6caa698606a154d8d93b23529364a11c33cadd5c (PR #147)
FINAL_HEAD=1c9847896aaeabda4b7cd9a227dd1559ec0210d8
EXACT_HEAD_CI=36033616030 (6/6)
CODE_HEAD_CI=36023022359 (6/6 on 7858106; ubuntu 854 passed / 0 failed /
  1 ignored, windows 851 passed / 0 failed / 1 ignored)
POST_MERGE_MAIN_CI=36193194092 (6/6 on 6caa698)
BASE=426bb34 (PR #148 merge, Spec 083 closure; post-main run 36033406508 6/6)
REVIEW_POLICY=FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22 (no external reviewer;
  deterministic scope record in EXACT_RANGE_REVIEW.md; security challenge in
  SECURITY.md)
TRANSPORT=in-process and Spec 024 local IPC only; NETWORK_LISTENER=none
DEPENDENCIES_ADDED=none
STORAGE_SCHEMA=v13 (v12->v13 additive; 14 restore tamper cases)
CONTRACTS=PASS (5)  KEYS=PASS (2)  STORAGE_MIGRATION_RECOVERY=PASS (6)
CORE_AUTHORITY=PASS (8)  CLI=PASS (1, real local socket on Linux and Windows)
REAL_PHI_AUTHORIZED=false
RELEASE_READY=false
PRIVATE_DATA_READY=false
MULTI_CLIENT_RELEASE_READY=false
SPEC_085_IMPLEMENTATION_AUTHORIZED=false until its own promotion
```

## What this closure establishes

MedScale has a local-first Hub foundation owned by Core:
- A Hub vault issues single-use invitations; a client enrolls an ed25519
  device key bound to the invitation's Project. Handshakes use single-use,
  device-bound nonces.
- Clients sign envelopes; the Hub verifies each one and applies it only
  through the Spec 076 collaboration authority. Conflicts follow Q07.
- Each Project keeps a hash-chained event log with unique `(device, seq)`
  claims; replays return the stored outcome, forgeries and sequence breaks
  are refused, and refusals still claim their sequence.
- Revocation ends the session, refuses the next handshake and propagates
  to other clients; nothing from a revoked device is applied.
- Clients keep links, an outbox and a read mirror; a restored client keeps
  link and mirror but not its key, and fails closed until re-enrollment.
- The Hub's device-facing IPC endpoint answers only bootstrap and sync.

## Defects found and fixed during qualification

- Design challenge: a hand-edited backup could swap a device key or plant
  an invitation; the key is now bound into `device_enrolled` and open
  invitations restore as revoked.
- Security challenge (`4b1aef7`): a sessionless peer on the Hub's endpoint
  ran as the vault's lease holder and could read Hub state; serving is now
  capability-limited (`8852460`).
- CI found four compile/Clippy defects, fixed forward (see
  `EXACT_RANGE_REVIEW.md`).

## Honest residuals (non-blocking, recorded)

- No network transport, TLS or multi-machine run; the Hub trusts the
  local socket's OS access control and does not sign responses.
- Applying a submission and recording its event are two commits; a crash
  between them can apply that submission again on retry (not simulated).
- Device secrets are protected by the vault at rest, not an OS key store.
- The Hub operator can read all Hub state (no end-to-end encryption, Q06).
- Row integrity in unencrypted vaults is structural and hash-chained, not
  Hub-signed.
- Performance with many devices or long chains is unmeasured; status
  verifies every chain in full.
- No Desktop surface; no rendered Desktop evidence.
- No local compile or test run completed on this workstation; GitHub
  Actions is the compiler of record.
