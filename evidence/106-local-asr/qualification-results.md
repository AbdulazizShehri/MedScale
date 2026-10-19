# Spec 106 qualification results (synthetic speech)

**Run:** `asr-qualification` run `39165339322` on branch `qual-asr/106-03`, `ubuntu-latest`, 2 jobs, both passed. Earlier runs `39163305776` and `39164088980` failed and led to two fixes:
- a runaway-decoding guard;
- WER number normalization, plus the mel reference read without deflate.

**Not claimed:** this is synthetic espeak-ng speech, 8 short English clinical sentences. It is evaluation evidence for a working recognizer. It is not a medical-accuracy claim, and it is not representative of real clinical audio.

## Mel filter bank vs OpenAI reference

MedScale computes the filter bank in code. OpenAI's `whisper/assets/mel_filters.npz` (MIT, commit `c5d4256`) is the reference.

| Filters | Max absolute difference |
|---|---|
| 80 mels | 1.86e-9 |
| 128 mels | 3.73e-9 |

## Transcription

| Checkpoint (pinned revision) | Signed Audio Pack | Prepare | Corpus WER (8 sentences) |
|---|---|---|---|
| `openai/whisper-tiny.en` (`87c7102`) | `hf-whisper-tiny.en-asr` | 776 ms | 0.313 |
| `openai/whisper-base.en` (`911407f`) | `hf-whisper-base.en-asr` | 1,444 ms | 0.104 |

**Observed errors:**
- Both checkpoints miss "metformin" ("Met for M in", "meant for info"), a medical-vocabulary weakness.
- Spoken numbers come out in mixed forms ("100 for the over 90").
- whisper-tiny.en drops most of "No known drug allergies. Follow up in 2 weeks." ("No no"). This is the case that looped before the runaway guard.

These are the reasons transcripts stay drafts for a person to correct.

## Diarization (acoustic clustering, unvalidated)

**Run:** `asr-qualification` run `39202024592` (branch `qual-asr/106-07`).

The dialogue uses six turns of synthetic espeak-ng speech, alternating two voices (`en-us+m3` and `en-us+f4`), with the same script also read in one voice.

| Dialogue | Speakers expected | Speakers found | Time-weighted accuracy |
|---|---|---|---|
| Two voices (male / female) | 2 | 2 | 1.0 |
| One voice | 1 | 1 | 1.0 |

### Method

- Each VAD segment gets a cepstral embedding: c1..c12 of the mean log-mel spectrum of its voiced frames, plus log pitch.
- Embeddings are standardized across the recording and grouped by average-linkage clustering.
- The cut is at the largest jump in merge distance.
- Groups less than about 10% apart in pitch are merged back.

### Development history

- Run `39198024187` found 4 groups for the two-voice dialogue, using pitch and zero-crossing rate only. That led to the embedding method above.
- Local checks with Windows SAPI synthetic voices gave:
  - David + Zira: 2 speakers, 1.0
  - Mark + Zira: 2 speakers, 1.0
  - David alone: 1 speaker

### Known limitation

**Two male voices** (David + Mark) were not separated reliably: accuracy 0.58. This is unsupervised acoustic clustering, not a trained speaker model. Labels are anonymous groups and never identity. Every clustered transcript carries `DiarizationIsUnvalidatedClustering`.
