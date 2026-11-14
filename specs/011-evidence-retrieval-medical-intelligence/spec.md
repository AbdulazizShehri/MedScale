# Feature Specification: Evidence / Retrieval / Medical Intelligence

**Branch**: `spec/011-evidence-retrieval-medical-intelligence`  
**Status**: QUALIFIED (004+008 CLOSED)  
**Input**: Deterministic lexical retrieval over synthetic corpora; evidence sets as EvaluationRecord (`evidence_only=true`). **RELEVANCE ≠ AUTHORITY**. No vector DB, REAL_PHI, or ClinicalAssertion from retrieval.

## User Stories
### US1 Lexical Retrieve (P1)
Query synthetic documents; ranked hits with scores; durable EvaluationRecord evidence set.

### US2 Relevance Never Authority (P1)
Retrieval cannot create ClinicalAssertion; Proposal forbidden as auto-authority; evidence_only enforced.

### US3 Conflict / Freshness Hooks (P1)
Evidence payload may note conflict/freshness markers as non-authoritative metadata.

## Anti-scope
Tantivy/vector servers, terminology table import, SDC legal conclusions, REAL_PHI, MESC.
