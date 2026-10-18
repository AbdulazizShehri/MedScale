# Spec 106 tasks

| Task | Description | Status |
|---|---|---|
| T106-01 | `asr_runtime`: candle Whisper, Slaney mel filters, greedy decoding, bounds, tiny fixtures and unit tests | Done (branch `spec/106-local-asr`) |
| T106-02 | `build_asr_pack` for pinned `ASR_SOURCES` with per-file Hub verification | Done |
| T106-03 | Core `PackAsrEngine`, `LocalAsrPack` route slot, `use_local_asr_pack`, route status | Done; `local_asr_106` test |
| T106-04 | `asr-qualification.yml`: pinned checkpoints, espeak-ng synthetic speech, WER, mel reference check | Passed: run `39165339322` (evidence/106-local-asr) |
| T106-05 | Desktop Audio workspace: admit an Audio Pack and transcribe on `LocalAsrPack`, with correction and evidence | Next |
| T106-06 | Draft note from a reviewed transcript (proposal only) | Later slice |
