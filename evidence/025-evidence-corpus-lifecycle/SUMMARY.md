# Evidence SUMMARY — Spec 025 Evidence Corpus Lifecycle

**Status:** CLOSED_CANONICAL READY_BASE  
**Branch:** `spec/025-evidence-corpus-lifecycle`  
**Design:** Pack-like `corpus.manifest.json` admits MedScale-owned synthetic lexical corpus; lexical retrieval + filters; EvaluationRecords remain evidence-only.

## Delivered

- `EvidenceCorpusManifest` with source identity (`corpus_id@version`), version, digests, `synthetic_owned` rights
- Builtin admitted corpus `synthetic-lexical@1.0.0` (5 docs; retracted + conflict markers)
- Version-bump fixture `synthetic-lexical@1.1.0`
- Lexical retrieval replaces hardcoded three-note fixture; `include_retracted` filter
- Hit/evaluation freshness, retraction, conflict markers as evidence metadata
- Doctor `evidence_corpus`: READY_BASE, `clinical_quality_claimed=false`, `release_ready=false`
- Tests: load, search hits, retracted excluded/marked, version bump identity, relevance ≠ authority

## Honesty

- RELEASE_READY = FALSE
- PRIVATE_DATA_READY = FALSE
- MULTI_CLIENT_RELEASE_READY = FALSE
- Clinical evidence quality not claimed
- OpenMed superiority not claimed
- REAL_PHI unauthorized; synthetic-owned fixtures only
