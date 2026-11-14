# Security and Privacy Qualification — Spec 081

Threats A1-A13 from `specs/081-audioflow-foundation/security.md`. Every
proving test passed in CI run `35889704517` on code head `502b49a` (see
`QUALIFICATION.md`).

| # | Threat | Result |
|---|---|---|
| A1 | Hidden cloud transcription | pass: `cloud_asr` always denied with a receipt; the scope record finds no network, socket or process API in any 081 module, and no change to `medscale-network` |
| A2 | Stealth or ambient capture | pass: capture starts only on an explicit request; no device backend exists; state readable at all times |
| A3 | Silent engine fallback | pass: the engine call counter stays at 0 for denied routes |
| A4 | Fixture output mistaken for recognition | pass: `is_fixture` identity plus a validated receipt limitation; CLI prints "fixture; not speech recognition" |
| A5 | Source audio overwritten | pass: insert-once rows; tampered bytes refused on read and restore |
| A6 | Silent transcript rewrite | pass: corrections are new revisions; the parent is byte-identical |
| A7 | Speaker identity claims | pass: speakers `unknown`; a restored anonymous label is refused |
| A8 | Voice command execution | pass: no code path outside tests references `VoiceInputMode::Command` (scope record) |
| A9 | Malformed audio files | pass |
| A10 | Resource exhaustion | pass: chunk, size, duration and segment bounds |
| A11 | Partial audio after crash | pass |
| A12 | Tampered storage or backups | pass (10 tamper cases) |
| A13 | Cross-scope reads | pass |

Honest limits:
- There is no native capture backend and no real ASR or diarization engine,
  so live device capture and speech recognition are not demonstrated.
- Medical ASR quality (medications, doses, numbers, units, negation,
  allergies, speaker attribution, Arabic/English code-switching) is
  `UNMEASURED`.
- The VAD is an energy threshold, validated on synthetic tones only;
  performance on real speech and noise is unmeasured.
- Encryption at rest applies only to encrypted vaults.
- Log-capture leakage is proven structurally (no print calls in 081 modules
  outside tests), not by an automated capture test.
- An `interrupted` transition is written on first read by a new process, so
  a read request can perform that one recovery write. It is audited.
