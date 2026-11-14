# Security Challenge — Spec 084 MedScale Hub Foundation

Deterministic challenge under `FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`.
No external or LLM reviewer ran. The threat table with controls and proofs
is `specs/084-hub/security.md` (H1-H12); this file records what the
challenge changed and what remains.

## Defects found in the design before first CI and fixed

| Finding | Fix |
|---|---|
| A hand-edited Hub backup could swap a device's public key, letting the holder of another key act as that device after restore | the public key is part of the chained `device_enrolled` event; restore checks every device against it (tamper case "device key swapped") |
| A hand-edited backup could plant an open invitation whose token the editor knows | invitations open at backup time restore revoked (round-trip test) |
| A backup could carry device secrets | secrets live in `hub_link_secrets`, which no backup writes; a restored client fails closed until re-enrollment |
| An envelope naming another device or hub could be written into this Project's chain | such envelopes are answered and never recorded; consistency requires every event to name an existing device of the same Project |
| A sessionless read over the Hub's IPC endpoint runs as the vault's lease holder, so a device-side peer could read Hub state (status, messages) as the operator | `serve_hub` answers only `HubBootstrap` and `HubSync`; everything else is refused before Core (`serve_connection_limited`; CLI test raw peer) |
| A malformed hex string could reach decoding | every key, signature, nonce and token is checked as exact-length lowercase hex before use; decoding refuses signs, whitespace and non-ASCII (PR #144 lesson) |

## Honest residuals

- Transport is local only; the Hub trusts the local socket's OS access
  control to reach the right Hub process (no Hub signature on responses).
- Device secrets are protected by the vault at rest, not an OS key store.
- Applying a submission and recording its event are two commits; a crash
  between them can apply that submission again on retry.
- The Hub operator can read all Hub state (no end-to-end encryption, Q06).
- Row-level integrity in unencrypted vaults is structural and hash-chained,
  not signed by the Hub.
