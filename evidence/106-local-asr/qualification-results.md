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
