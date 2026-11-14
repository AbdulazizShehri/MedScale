# Contracts — Spec 081 AudioFlow Foundation

`crates/medscale-contracts/src/audio.rs`, schema `AUDIO_SCHEMA_VERSION = 1`.

```text
AudioSource        { header, project_id, kind, label, format, duration_ms,
                     byte_length, content_digest, health, capture_session_id? }
AudioSession       { header, revision, project_id, label, backend, format,
                     state, started_by, captured_bytes, chunk_count, source_id? }
TranscriptRevision { header, project_id, source_id, source_digest,
                     revision_no, origin, mode, language?, diarization,
                     state, segments[] }
TranscriptSegment  { seq, start_ms, end_ms, speaker, status, text? }
TranscriptReceipt  { header, project_id, source_id, source_digest,
                     request_digest, route, decision, transcript_revision_id?,
                     engine?, vad, segment_count, limitations[] }
AudioEvidenceRef   { source_id, source_digest, start_ms, end_ms,
                     transcript_revision_id, segment_seq }
AudioRouteRequest  { project_id, source_id, route, mode, language?,
                     diarization_required }
```

Closed vocabularies:
- `AudioSourceKind`: imported_file, capture.
- `CaptureBackendKind`: native_device (unavailable), scripted.
- `CaptureState`: recording, paused, stopped, cancelled, interrupted.
- `CaptureHealth`: ok, silent, clipping, no_audio.
- `VoiceInputMode`: command, context, dictation.
- `AudioRoute`: segmentation_only, fixture_asr, local_asr_pack (unavailable),
  cloud_asr (always denied).
- Also closed: `AudioRouteDenyReason`, `SegmentStatus`, `DiarizationState`,
  `TranscriptState`, `AudioLimitation`.

`SpeakerLabel` is `unknown` or an anonymous index, never a name.

Invariants (enforced by `validate`):
- segments are numbered 1..n, ordered, non-overlapping and inside the source;
- a segment's status matches its text;
- only human corrections mark segments corrected;
- a fixture revision names the fixture engine;
- no revision comes from `cloud_asr` or `local_asr_pack`;
- a receipt names a revision exactly when allowed, and always states
  `transcript_not_clinical_truth`;
- a stopped session names its source.
