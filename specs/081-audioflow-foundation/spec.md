# Spec 081 — AudioFlow Foundation

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promoted:** 2026-09-23
**Base SHA:** `c682178ed4b8a4bbee67fe64d89cc94450d60b13`
**Target branch:** `spec/081-audioflow-foundation`
**Dependency:** 077 + 079 (both closed)
**Promotion authority:** `docs/planning/SPEC_081_PROMOTION.md`

## 1. Problem

Research and clinical work produces audio (dictation, interviews, huddles).
Without a governed local path, audio and anything derived from it would live
outside MedScale authority. There would be no immutable source, no link from
text back to the exact audio span, no record of which engine produced which
text, and no guarantee that audio never silently leaves the device.

## 2. Goal

One Core-owned, local-only AudioFlow foundation:
- audio sources are immutable evidence;
- capture starts only on an explicit request and is always visible;
- transcripts are derived revisions with receipts, and corrections add
  revisions;
- every segment resolves to its exact source audio;
- routes that cannot run (remote speech, unadmitted engines, diarization)
  are refused with reasons, never faked.

## 3. Scenarios

1. A researcher imports a synthetic WAV. A source with digest, format,
   duration and signal health is stored.
2. A malformed, compressed, truncated or oversized file is refused, and
   nothing is stored.
3. A capture is started explicitly, paused, resumed and stopped; the stored
   source equals exactly the captured frames. A cancelled capture leaves
   nothing.
4. A process ends during a capture. The next process reports that capture
   as `interrupted`, with no partial audio.
5. Asking for `cloud_asr`, `local_asr_pack` or diarization returns a
   receipt with the reason; no engine runs.
6. `segmentation_only` yields deterministic timestamped segments. With the
   fixture engine installed, `fixture_asr` yields labelled fixture text
   whose receipt says it is not speech recognition.
7. A correction creates revision n+1 linked to its parent; the parent is
   unchanged.
8. `COMMAND`-mode text is stored and nothing executes it.

## 4. Requirements

- FR-01 WAV import with format, size and duration checks; digest-bound
  immutable source.
- FR-02 Capture lifecycle (start, append, pause, resume, stop, cancel) with
  CAS revisions; native backend unavailable with a reason.
- FR-03 Interruption recovery for captures orphaned by an ended process.
- FR-04 Deterministic VAD segmentation and signal health with frozen,
  receipted parameters.
- FR-05 Route decisions persisted as receipts; local-only routes.
- FR-06 Transcript revisions (contiguous per source) and human corrections.
- FR-07 Audio evidence references from segment to source digest and time.
- FR-08 Storage v10, backup/restore and consistency checks.
- FR-09 CLI and Desktop over Core.

## 5. Success criteria

The frozen acceptance requirements in `SPEC_081_PROMOTION.md`.
