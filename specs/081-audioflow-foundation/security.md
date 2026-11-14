# Security and Privacy — Spec 081 AudioFlow Foundation

| # | Threat | Control | Proof |
|---|---|---|---|
| A1 | Hidden cloud transcription | no network code in any 081 module; `cloud_asr` is always denied with a receipt; engines are in-process trait objects | Core `routes_are_local_only_and_every_request_leaves_a_receipt`; scope record grep |
| A2 | Stealth or ambient capture | capture starts only on an explicit authenticated request; state always readable; no device backend exists | Core `capture_is_explicit_bounded_and_stops_into_exact_audio` |
| A3 | Silent fallback to another engine | route decided before any engine call; denied routes never reach an engine | Core `fixture_transcripts_are_labelled_deterministic_and_command_text_is_inert` (engine call counter) |
| A4 | Fixture output mistaken for recognition | engine identity `is_fixture`; receipt limitation required by validation | contracts `receipts_state_their_limits` |
| A5 | Source audio overwritten | insert-once source rows; digest checked on every read and restore | storage `sources_are_immutable_and_digest_checked` |
| A6 | Silent transcript rewrite | corrections are new revisions; parent unchanged; only the latest can be corrected | Core `corrections_create_new_revisions_and_never_touch_the_parent` |
| A7 | Speaker identity claims | speakers are `unknown`; other labels are refused | contracts; storage tamper case "speaker named" |
| A8 | Voice command execution | `COMMAND` text is stored only; no execution path exists | Core fixture test; scope record |
| A9 | Malformed or hostile audio files | strict RIFF parser with bounds on every chunk; size and duration caps | Core unit `wav_round_trips_and_malformed_files_are_refused`; Core `imports_are_validated_immutable_and_measured` |
| A10 | Resource exhaustion | 32 MiB / 30 min per source, 1 MiB per chunk, 4,000 segments | Core capture test (oversized chunk) |
| A11 | Partial audio after crash | interrupted captures keep no chunks and produce no source | Core `an_open_capture_from_an_ended_process_reopens_interrupted` |
| A12 | Tampered storage or backups | column/body checks, digest checks, consistency on restore | storage `restore_rejects_hand_edited_081_snapshots` (10 cases) |
| A13 | Cross-scope reads | every read scope-checked | Core import test (other scope refused) |

Real PHI remains unauthorized, and all test audio is synthetic tone and
silence. Encryption at rest applies only to encrypted vaults; synthetic
vaults hold synthetic data only. Medical ASR quality is `UNMEASURED`.
