# ADR-025-001 — Pack-like synthetic evidence corpus

## Status
Accepted

## Context
Spec 011 hardcodes three synthetic notes. Q10 requires a source-versioned local corpus with digests, rights, and lifecycle markers without introducing licensed corpora or clinical authority.

## Decision
Admit MedScale-owned synthetic corpora via Pack-like `corpus.manifest.json` (source identity, version, digests, `synthetic_owned` rights). Lexical retrieval filters retracted docs by default and attaches freshness/conflict markers as evidence metadata only.

## Consequences
- Retrieval depends on admitted corpus identity, not embedded literals alone.
- Version bumps change `corpus_id@version` and content digest independently of source-identity rules.
- Doctor can report READY_BASE without RELEASE_READY or clinical-quality claims.
