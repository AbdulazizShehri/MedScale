# Contracts — Spec 083 Knowledge + Research Canvas

`crates/medscale-contracts/src/knowledge.rs`, schema
`KNOWLEDGE_SCHEMA_VERSION = 1`, tokenizer `lexical-v1`.

```text
IndexSourceRef   { kind, object_id, content_digest, lineage_id }
                 kind: data_snapshot | transcript_revision | web_evidence
                       | analytics_result
SpanLocator      cell{row, column} | segment{seq, start_ms, end_ms} | excerpt
EvidenceSpanRef  { source, locator, char_start, char_end }
IndexChunk       { seq, span, text }            (text <= 1000 chars)
IndexManifest    { header, project_id, version, tokenizer, sources[],
                   chunk_count, chunks_digest }
IndexStatus      { manifest_id, version, chunk_count, current_sources,
                   superseded_sources, tombstoned_sources, unindexed_sources }
Freshness        current | superseded | tombstoned
RetrievalRequest { project_id, query, max_hits?, include_stale }
RetrievalPlan    { stages[lexical], terms[], max_hits, include_stale }
RetrievalReceipt { header, project_id, query, query_digest, plan?,
                   manifest_id?, manifest_chunks_digest?, index_status?,
                   outcome, deny_reason?, hits[ReceiptHit] }
ReceiptHit       { rank, chunk_seq, score, span, freshness }   (no text)
RetrievalResult  { receipt, hits[{hit, text?}] }
CanvasRevision   { header, project_id, revision, title, nodes[], edges[] }
CanvasNode       { key, content: note{text} | evidence{span} }
CanvasEdge       { from, to, relation: supports|contradicts|relates|questions }
CanvasOp         add_note | add_evidence | link | unlink | remove_node | retitle
CanvasView       { canvas, resolutions[], contradictions[],
                   unsupported_notes[], superseded_evidence[],
                   missing_evidence[] }
```

Lineage: a snapshot's data source, a transcript revision's audio source, a
web evidence item's browse session, and a derived table itself.

Invariants (validated in contracts and re-checked on every storage read):
- manifest sources are sorted and unique; chunks are numbered 1..n, each
  from an indexed source, with text length equal to its span; the
  chunk-list digest is recorded;
- a receipt is `denied` exactly when it has a deny reason; receipts that ran
  name their plan and index; the outcome is `results` exactly when a current
  span matched; stale hits appear only with `include_stale`; ranks are 1..n;
- canvas node keys are unique (`[a-z0-9_-]`, at most 32 characters); edges
  join existing, distinct nodes and are unique; notes are 1-4000 characters.

Lexical scoring (`lexical-v1`), integer only: for each distinct query term
in a chunk, `min(tf, 3) * 1000 * (n + 1) / (df + 1)`, summed, times
`64 / (64 + chunk tokens)`. Ties break by chunk sequence.

Envelope capabilities: `KnowledgeIndex`, `KnowledgeSearch`,
`KnowledgeRead` (read-only) and `KnowledgeCanvas`.
