//! Deterministic lexical retrieval (Specs 011 + 025) — relevance ≠ authority.

use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::evidence::{
    EvidenceCorpusDocument, EvidenceCorpusManifest, EvidenceDocStatus, LexicalRetrieveRequest,
    LexicalRetrieveResult, RetrievalHit,
};
use medscale_contracts::objects::{EvaluationRecord, ObjectHeader, OpaqueId};

use super::corpus::{default_synthetic_corpus, matches_request, scale_synthetic_corpus_10k};
use super::store::{InMemoryAuthorityStore, StoredObject};

fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

fn score(query: &str, doc: &str) -> f64 {
    let q = tokenize(query);
    if q.is_empty() {
        return 0.0;
    }
    let d = tokenize(doc);
    let mut hits = 0_u32;
    for t in &q {
        if d.iter().any(|x| x == t) {
            hits += 1;
        }
    }
    f64::from(hits) / q.len() as f64
}

fn select_corpus<'a>(
    request_corpus_id: &str,
    alternate: Option<&'a EvidenceCorpusManifest>,
) -> Result<&'a EvidenceCorpusManifest, AuthorityError> {
    let default = default_synthetic_corpus();
    if matches_request(default, request_corpus_id) {
        return Ok(default);
    }
    let scale = scale_synthetic_corpus_10k();
    if matches_request(scale, request_corpus_id) {
        return Ok(scale);
    }
    if let Some(alt) = alternate
        && matches_request(alt, request_corpus_id)
    {
        return Ok(alt);
    }
    Err(AuthorityError::InvalidArgument {
        message: format!("unknown corpus_id {request_corpus_id}"),
    })
}

/// Rank admitted corpus; persist evidence-only EvaluationRecord.
pub(crate) fn retrieve_lexical(
    store: &mut InMemoryAuthorityStore,
    realm_id: medscale_contracts::objects::RealmId,
    scope_id: medscale_contracts::objects::AuthorityScopeId,
    request: LexicalRetrieveRequest,
) -> Result<LexicalRetrieveResult, AuthorityError> {
    retrieve_lexical_with_corpus(store, realm_id, scope_id, request, None)
}

/// Rank with optional alternate admitted corpus (version-bump tests).
pub(crate) fn retrieve_lexical_with_corpus(
    store: &mut InMemoryAuthorityStore,
    realm_id: medscale_contracts::objects::RealmId,
    scope_id: medscale_contracts::objects::AuthorityScopeId,
    request: LexicalRetrieveRequest,
    alternate: Option<&EvidenceCorpusManifest>,
) -> Result<LexicalRetrieveResult, AuthorityError> {
    let corpus = select_corpus(&request.corpus_id, alternate)?;
    let max = request.max_hits.clamp(1, 32) as usize;

    let mut scored: Vec<(f64, &EvidenceCorpusDocument)> = corpus
        .documents
        .iter()
        .filter(|d| request.include_retracted || !matches!(d.status, EvidenceDocStatus::Retracted))
        .map(|d| (score(&request.query, &d.text), d))
        .filter(|(s, _)| *s > 0.0)
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(max);

    let hits: Vec<RetrievalHit> = scored
        .into_iter()
        .map(|(s, d)| RetrievalHit {
            doc_id: OpaqueId::new(d.doc_id.clone()),
            score: s,
            snippet: d.text.clone(),
            relevance_only: true,
            retracted: matches!(d.status, EvidenceDocStatus::Retracted),
            freshness_marker: d.freshness_marker.clone(),
            conflict_marker: d.conflict_marker.clone(),
            corpus_version: corpus.version.clone(),
        })
        .collect();

    let evaluation_id = store.alloc_id("eval");
    let target_refs: Vec<OpaqueId> = hits.iter().map(|h| h.doc_id.clone()).collect();
    let source_identity = corpus.source_identity();
    store.insert(StoredObject::Evaluation(EvaluationRecord {
        header: ObjectHeader {
            id: evaluation_id.clone(),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id,
            authority_scope_id: scope_id,
        },
        target_refs,
        evaluator: "medscale.retrieval.lexical.v1".to_owned(),
        result: serde_json::json!({
            "corpus_id": corpus.corpus_id,
            "corpus_version": corpus.version,
            "corpus_source_identity": source_identity,
            "corpus_content_digest": corpus.content_digest.to_hex(),
            "query": request.query,
            "hits": hits,
            "relevance_is_not_authority": true,
            "evidence_only": true,
            "freshness": "corpus_document_markers",
            "conflict": "corpus_document_markers",
            "clinical_quality_claimed": false
        }),
        evidence_only: true,
    }));

    Ok(LexicalRetrieveResult {
        corpus_id: corpus.corpus_id.clone(),
        corpus_version: corpus.version.clone(),
        corpus_source_identity: source_identity,
        query: request.query,
        hits,
        evaluation_id,
        evidence_only: true,
        relevance_is_not_authority: true,
    })
}
