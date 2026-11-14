# Qualification — Spec 084 MedScale Hub Foundation

Exact-head CI for the final code head is recorded in
`EXACT_HEAD_QUALIFICATION.md`; the counts below are test inventories.

All data is synthetic. Transport is in-process (JSON round trip of
`AuthorityRequest`/`AuthorityResponse`) and the Spec 024 local socket; no
network code exists.

## Contracts (`crates/medscale-contracts/src/hub.rs`, 5 tests)

`vocabularies_round_trip_and_are_closed` (unknown intent kinds do not
parse), `hex_and_endpoint_checks_are_strict` (uppercase, non-hex, wrong
length, non-ASCII, path-like endpoints refused),
`signing_payloads_are_domain_separated_and_stable`,
`envelopes_entries_and_links_validate`,
`event_chains_detect_edits_gaps_and_reorders`.

## Keys (`crates/medscale-keys/src/device_key.rs`, 2 tests)

`keys_sign_and_verify` (wrong payload and wrong key refused),
`encodings_are_strict` (uppercase, whitespace, signs, short, non-ASCII hex
refused; weak identity key refused).

## Storage v13 (`crates/medscale-storage/tests/hub_084.rs`, 6 tests)

`migration_v12_to_v13_is_additive` (idempotent reopen),
`hub_rows_hold_their_invariants` (one Hub identity; single redemption;
device bound to its invitation's Project; single-use device-bound nonces;
unique `(device, seq)` claims matching the outcome and envelope digest;
outbox only at the next sequence; outcome recorded once; mirror only
continues its chain; pages verified from the cursor),
`edited_or_removed_event_rows_fail_closed_on_read`,
`backup_restore_roundtrips_hub_rows_without_secrets` (no device secret in
the snapshot; open invitation restored revoked),
`restore_rejects_hand_edited_084_snapshots` (14 cases: event outcome
edited, event removed, events reordered, device key swapped, device moved
to another Project, revoked device restored active, invitation names
another device, events replaced by a string, devices missing, second Hub
identity, link cursor ahead of its mirror, outbox entry removed, mirrored
event edited, claimed sequence duplicated),
`pre_084_v12_backup_restores_with_empty_hub_tables`.
The Spec 078-083 storage suites drop the v13 tables when they rewind.

## Core (`crates/medscale-core/tests/hub_084.rs`, 8 tests)

| Behavior | Test |
|---|---|
| No Hub before init; init once; tampered, reused, revoked and wrongly signed invitations refused; malformed hex refused before lookup; wrong protocol version, bad signature (nonce consumed), replayed and never-issued nonces refused; device session denied non-Hub capabilities; another tenant scope sees nothing | `invitations_and_handshakes_fail_closed` |
| Two clients queue offline; after sync both mirrors hold the same six verified events; the Hub applied each as the device's own participant; outboxes drained; an idle sync changes nothing | `two_clients_edit_offline_and_converge` |
| Stale task update is a conflict with the current revision; stale note edit is a conflict copy; messages append; the Hub task keeps the first update | `conflicts_follow_q07` |
| Identical replay returns the stored outcome and records nothing; forged and re-signed envelopes refused as bad signatures; gaps refused; other hub and other device refused unrecorded; a different body validly signed at a used sequence refused as reused | `replays_forgeries_and_sequence_breaks_are_refused` |
| Revocation ends the session, refuses the next handshake, cannot be repeated, propagates to the other client; nothing from the revoked device is applied | `revocation_blocks_the_device_and_propagates` |
| Another Project's room refused even with membership; a room without membership refused; missing objects refused; refusals still claim sequences; an operator session cannot pull; links are scope-bound | `projects_and_rooms_stay_isolated` |
| Hub and client reopen and continue; a restored client keeps link and mirror but not its key and fails closed until re-enrollment | `hub_and_client_state_survive_reopen_and_restore_needs_re_enrollment` |
| A Hub served over the local socket enrolls and syncs a client | `a_hub_served_over_local_ipc_syncs_a_client` |

## CLI (`crates/medscale-cli/src/hub.rs`, 1 test)

`hub_commands_run_through_core_over_local_ipc`: status before init, init,
invite, `serve` in a thread for three connections, join and sync over the
local socket, a raw local peer whose sessionless `HubStatus` is refused by
the endpoint's capability limit, a malformed intent refused, links, outbox,
mirror and status in human and JSON form, revocation, and a repeated
revocation refused.

## Not demonstrated (recorded, non-blocking)

- No network transport, TLS or multi-machine run (out of scope).
- No Desktop surface.
- A crash between applying a submission and recording its event is not
  simulated; retry after such a crash could apply the submission again.
- Performance with many devices or long event chains is unmeasured; status
  verifies every chain in full.
