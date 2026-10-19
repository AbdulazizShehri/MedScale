# Spec 106: Local speech recognition for AudioFlow

**Status:** `PROMOTED` under the founder directive "MEDSCALE — FULL PRODUCT COMPLETION, DEEP REA, SOURCE REUSE & OPENMED SUPERIORITY" (AudioFlow ambient vertical, local ASR reuse).
**Depends on:**
- Spec 081 (AudioFlow foundation: immutable audio sources, VAD segmentation, route policy, transcript revisions, the `AsrEngine` seam).
- Spec 103 (signed Packs, digest verification).
- Spec 104 (candle admission).

**Tasks:** [tasks.md](tasks.md).

## Goal

Fill Spec 081's `LocalAsrPack` route with real, local speech recognition, so that an ambient or dictated recording becomes a reviewable transcript on the device. The route keeps its governance and adds no network path or new security exception.

## Reuse decision (COPY > ADAPT > INTEGRATE > BUILD)

- **INTEGRATE:** candle-transformers `=0.9.1` (already admitted, MIT OR Apache-2.0) provides the Whisper encoder, decoder and log-mel front end. No new crate.
- **BUILD:** the mel filter bank is computed in code with the Slaney scale and norm, instead of copying a binary asset. Qualification checks it against OpenAI's published `mel_filters.npz` (MIT, pinned commit `c5d4256`). The greedy decoding loop and the Pack and Core wiring are first-party.
- **Acquire on demand, never bundle:** Whisper weights, at pinned revisions only.

## Requirements

- **R106-01 Runtime.**
  - Runtime id `candle_whisper_asr_v1`, CPU, data-only `model.safetensors` with `config.json` and `tokenizer.json`.
  - `num_mel_bins` must be 80 or 128.
  - The control tokens `<|startoftranscript|>`, `<|transcribe|>`, `<|notimestamps|>` and `<|endoftext|>` must exist.
  - Multilingual checkpoints are prompted for English.
  - Decoding is greedy and deterministic, without timestamps, at most 224 tokens per 30-second window.
  - Timestamp tokens and configured `suppress_tokens` are masked.
  - The KV cache is reset per window.
  - Input is limited to 10 minutes of audio per call.
- **R106-02 Audio Packs.** `build_asr_pack` writes a signed Pack only for a repository and revision in `ASR_SOURCES`:
  - `openai/whisper-tiny.en@87c7102`
  - `openai/whisper-tiny@169d4a4`
  - `openai/whisper-base.en@911407f`

  The model cards declare Apache-2.0. Every file is verified against Hub metadata (LFS SHA-256 or git blob id). Weights stay under the unchanged 1 GiB artifact bound (151–290 MB).
- **R106-03 Core route.**
  - `PackAsrEngine` serves only `LocalAsrPack`, through a second engine slot; the fixture engine keeps `FixtureAsr`.
  - Its identity names the runtime and Pack and is never a fixture.
  - `CliSession::use_local_asr_pack` installs it only when the folder admits and the same Pack and digest are admitted in the vault.
  - Without an engine, the route is listed unavailable and denied (`RouteUnavailable`).
  - Transcription needs 16 kHz, 16-bit PCM.
  - Transcript limitations still include `TranscriptNotClinicalTruth` and `MedicalAccuracyUnmeasured`.
- **R106-04 Qualification.** `asr-qualification.yml` runs on `qual-asr/**` branches only:
  - It downloads the pinned checkpoints and generates synthetic English clinical sentences with espeak-ng (a runner-only test tool).
  - It builds and admits the Pack, transcribes, and reports per-case and corpus WER, with a gate of corpus WER < 0.5.
  - It checks the mel filters against OpenAI's reference (max absolute difference < 1e-5).

## Non-goals

- Trained speaker-embedding diarization. Spec 106 adds only unvalidated acoustic clustering (`DiarizationState::AcousticClustering`): anonymous "speaker N" groups, never identity.
- Timestamps per word, streaming, language detection, translation and cloud ASR.
- Note generation from transcripts (a later slice).
- Any claim of medical accuracy.
