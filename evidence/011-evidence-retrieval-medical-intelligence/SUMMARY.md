# Spec 011 evidence summary

## Delivered
- Deterministic lexical retrieval over `synthetic-lexical-v0`
- Hits persisted as EvaluationRecord with `evidence_only=true`
- `relevance_is_not_authority` on every result

## Limitations
- No Tantivy/vector DB
- No terminology grounding runtime
- No REAL_PHI; synthetic corpus only
- No ClinicalAssertion from retrieval
