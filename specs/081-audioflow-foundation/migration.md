# Migration and Recovery — Spec 081

Additive: storage schema v9 -> v10. `CURRENT_META_SCHEMA_VERSION` becomes 10.

```text
audio_sources              (insert-once; WAV content BLOB; digest re-checked on read)
audio_capture_sessions     (revisioned; CAS on every transition)
audio_capture_chunks       (only while a capture is open; contiguous seq)
audio_transcripts          (insert-once; unique source_id + revision_no)
audio_transcript_receipts  (insert-once; at most one per revision)
```

Atomic writes:
- Stop inserts the source, closes the session and deletes the chunks in one
  transaction.
- Cancel and interruption delete the chunks together with the state change.
- A transcription writes its receipt and revision in one transaction.

Cross-row invariants (`verify_audio_consistency`, run on restore):
- every row names a Project in its own realm and scope;
- a stopped session and its capture source name each other;
- chunks exist only for open sessions and sum to the session's bytes;
- revisions are contiguous per source and match the source digest and
  duration;
- corrections cite an earlier revision of the same source;
- each engine revision has exactly one matching receipt.

A v9 backup restores with empty audio tables. A crash mid-v10 migration
fails closed (`MigrationIncomplete(10)`). Tests of Specs 078-080 that rewind
a vault also drop the v10 tables.

Recovery: an open capture whose owning process ended is marked
`interrupted` on first access by a new process, and its chunks are deleted.
No partial source is ever produced.
