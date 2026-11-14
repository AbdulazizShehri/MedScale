# Qualification — Spec 081 AudioFlow Foundation

Code head `502b49a`, CI run `35889704517` (pull_request). Every test named
below passed in the `rust (ubuntu-latest)` job: 119 `test result` lines,
770 passed, 0 failed, 0 `FAILED`/`panicked` lines. The macOS job also
passed. The Windows result and the full job table are in
`EXACT_HEAD_QUALIFICATION.md`. All audio is synthetic tone and silence
generated in code; storage tests use byte patterns.

## Contracts (`crates/medscale-contracts/src/audio.rs`, 6 tests)

`vocabularies_round_trip_and_are_closed` (closed routes, native backend
unavailable), `capture_transitions_are_explicit` (terminal states are
final), `formats_and_sessions_validate` (16-bit PCM, 8-48 kHz, mono/stereo,
whole frames, stopped sessions name a source, native backend refused),
`transcript_revisions_hold_their_invariants` (ordered non-overlapping
segments inside the source, state matches text, no revision from
`cloud_asr`, fixture revisions name the fixture, corrections follow an
earlier revision, evidence references), `receipts_state_their_limits`,
`speaker_labels_serialize_without_identity`.

## Storage v10 (`crates/medscale-storage/tests/audio_081.rs`, 9 tests)

`migration_v9_to_v10_is_additive` (and idempotent reopen),
`crash_mid_v10_migration_fails_closed_and_backup_recovers`,
`sources_are_immutable_and_digest_checked`,
`capture_chunks_are_atomic_and_leave_with_the_open_state`,
`transcripts_form_a_contiguous_lineage_over_immutable_sources`,
`consistency_check_detects_invariant_breaks`,
`backup_restore_roundtrips_every_081_row_exactly` (including an open
capture's chunks), `restore_rejects_hand_edited_081_snapshots` (10 tamper
cases: source bytes, missing project, other scope, duplicate transcript,
revision gap, missing engine receipt, missing correction parent, dropped
chunk, stopped capture without source, named speaker),
`pre_081_v9_backup_restores_with_empty_audio_tables`. The Spec 078, 079 and
080 storage suites pass with `CURRENT_META_SCHEMA_VERSION = 10`.

## Core (`crates/medscale-core/tests/audio_081.rs`, 7 tests; unit tests 3)

| Behavior | Test |
|---|---|
| Valid import measured (duration, digest, health ok/silent); empty, non-RIFF, compressed, truncated and data-less files refused; other scope cannot read | `imports_are_validated_immutable_and_measured` |
| Native backend unavailable with reason; partial frame, oversized chunk, append while paused, stale revision and stop-by-transition refused; stop stores exactly the appended frames (digest equality); empty capture cannot stop; cancel leaves no source | `capture_is_explicit_bounded_and_stops_into_exact_audio` |
| Recording and paused captures from an ended process reopen `interrupted`, with no partial source; appends refused | `an_open_capture_from_an_ended_process_reopens_interrupted` |
| `cloud_asr`, `local_asr_pack`, uninstalled `fixture_asr` and diarization denied with reasons and receipts; `segmentation_only` yields deterministic segments, speakers unknown, limitations stated; only `segmentation_only` available without an engine | `routes_are_local_only_and_every_request_leaves_a_receipt` |
| Denied route never reaches the engine (call counter 0); fixture output labelled, deterministic across runs; `COMMAND` text stored only; every segment resolves to source digest and span | `fixture_transcripts_are_labelled_deterministic_and_command_text_is_inert` |
| Invalid corrections refused; correction is revision n+1 citing its parent; the parent is byte-identical; only the latest revision can be corrected | `corrections_create_new_revisions_and_never_touch_the_parent` |
| Sources, captures and revisions identical after reopen | `audio_state_survives_reopen` |
| Unit: WAV round trip and malformed files; VAD determinism, hangover, minimum speech, silence, clipping, stereo; route availability and fixture labelling | `authority::audio::tests::*` |

## CLI (`crates/medscale-cli/src/audio.rs`)

`audio_commands_run_through_core_across_fresh_sessions`:
- malformed import refused;
- imports and scripted captures in human and JSON form;
- native capture refused;
- `--leave-open` capture reported `interrupted` by the next process;
- every route requested, with receipt reasons verified in order;
- read commands;
- correction, a stale correction refused, and evidence;
- the parent revision unchanged.

## Desktop (`crates/medscale-desktop/src/audio_workspace.rs`)

`audio_view_models_flow_through_a_real_core_session`:
- empty overview with route availability and the native-capture reason;
- import and an open capture shown as `recording`;
- segmentation through Core (2 segments, untranscribed);
- a missing source refused.

The Slint route compiled on all three CI platforms. The Spec 066 hardening
test caught warning-coloured text in the first draft of the route; it now
uses `ink-subtle`. There is no rendered screenshot, the same residual as
Specs 075-080.

## Local supporting runs (not authoritative)

`cargo fmt --all` and `git diff --check` pass locally. No local compile or
test run completed: the Windows toolchain cannot link, two WSL builds were
stopped by the host for memory pressure, and the C: drive then reported
0 bytes free. CI is the only compiler of record for this spec.
