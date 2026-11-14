# Security — Spec 084 MedScale Hub Foundation

| # | Threat | Control | Proof |
|---|---|---|---|
| H1 | A second authority plane | the Hub writes collaboration rows only through the Spec 076 `Collab` authority, under the device's own Core session; clients never write Hub state | Core `two_clients_edit_offline_and_converge` (authors are the device participants) |
| H2 | Enrollment by someone without the invitation or the key | one-time token (digest stored); enrollment signature over hub id, token and public key; reused, revoked or unknown tokens are indistinguishable | Core `invitations_and_handshakes_fail_closed` |
| H3 | Handshake replay or forgery | single-use nonce bound to the device and consumed before the signature check; protocol version check; `verify_strict` | Core `invitations_and_handshakes_fail_closed` |
| H4 | Session abuse | a device session is granted only `HubSync`; an operator session is not a device | Core handshake and isolation tests |
| H5 | Envelope replay, forgery, reordering | per-device sequence claims (unique `(device, seq)`); identical resubmission returns the stored outcome; bad signature, reused sequence and gaps are refused and recorded | Core `replays_forgeries_and_sequence_breaks_are_refused`; storage `hub_rows_hold_their_invariants` |
| H6 | Last-writer-wins on canonical state | tasks use expected revisions (conflict returned); notes keep conflict copies; messages append (Q07) | Core `conflicts_follow_q07` |
| H7 | Revoked devices | revocation revokes the participant and every session of the device; handshakes are refused; a racing submission is recorded as `device_revoked` | Core `revocation_blocks_the_device_and_propagates` |
| H8 | Cross-Project or cross-scope writes | the intent's room must belong to the device's Project; membership is re-checked by 076; realm and scope bind every Hub and client read | Core `projects_and_rooms_stay_isolated`, handshake scope check |
| H9 | Tampered Hub history | per-Project hash chain verified on every read, pull, status and restore; clients verify continuity and refuse a Hub whose head moved backwards | storage `edited_or_removed_event_rows_fail_closed_on_read`, `restore_rejects_hand_edited_084_snapshots` |
| H10 | Key swap or planted invitation in a backup | the device key is bound into the chained enrollment event; invitations open at backup time restore revoked | storage restore tamper cases |
| H11 | Device secret disclosure | the secret stays in the client vault; it is never returned by Core, never sent, never in a backup | storage `backup_restore_roundtrips_hub_rows_without_secrets`; Core restore test |
| H12 | Malformed input | strict lowercase hex of exact lengths; closed vocabularies; bounded batches (100) and names | contract and key unit tests |
| H13 | Lease-holder reads over the Hub endpoint | the device-facing endpoint serves only `HubBootstrap` and `HubSync` | CLI `hub_commands_run_through_core_over_local_ipc` (raw peer refused) |

Transport is local only (in-process or the Spec 024 local socket). There
is no network listener, no TLS and no new dependency. Real PHI remains
unauthorized; all fixtures are synthetic.
