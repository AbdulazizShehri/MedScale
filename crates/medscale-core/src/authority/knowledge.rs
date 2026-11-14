//! Knowledge + Research Canvas Core authority paths (Spec 083).
//!
//! The Project index is a rebuildable projection over exact source
//! revisions that Core already governs: Spec 075 snapshots, Spec 080 web
//! evidence, Spec 081 transcript revisions and Spec 082 derived tables. Every
//! read goes through the same scope and Project checks as those specs, and
//! nothing here writes to a source. Freshness is computed against the live
//! Project on every request, so a superseded or archived source is reported,
//! never silently served. There is no retrieval cache: authorization and
//! freshness are evaluated per request (Q31). Only lexical retrieval exists
//! (Q28/Q29).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use medscale_clinical::evidence_coverage::{
    CoverageStatus, Requirement, build_coverage_matrix, normalize_claim_id,
    normalize_evidence_class,
};
use medscale_clinical::evidence_span_audit::{OverlapKind, audit_evidence_spans, span_from_ref};
use medscale_contracts::data_sources::{CellValue, SourceStatus};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::knowledge::{
    CANVAS_OPS_MAX, CHUNK_MAX_CHARS, CanvasNodeContent, CanvasOp, CanvasRelation, CanvasRevision,
    CanvasSummary, CanvasView, CoverageState, EvidenceOverlap, EvidenceOverlapKind,
    EvidenceSpanRef, Freshness, HITS_DEFAULT, HITS_MAX, INDEX_CHUNKS_MAX, IndexChunk,
    IndexManifest, IndexSourceRef, IndexStatus, KNOWLEDGE_SCHEMA_VERSION, KnowledgeSourceKind,
    LEXICAL_TOKENIZER, NodeResolution, NoteCoverage, QUERY_MAX_CHARS, QUERY_TERMS_MAX, ReceiptHit,
    RetrievalDenyReason, RetrievalHit, RetrievalOutcome, RetrievalPlan, RetrievalReceipt,
    RetrievalRequest, RetrievalResult, RetrievalStage, SpanLocator, chunks_digest,
};
use medscale_contracts::objects::{DigestSha256, ObjectHeader, OpaqueId};
use medscale_storage::MetaError;

use super::data_sources::DataSources;

fn meta_err(err: MetaError) -> AuthorityError {
    match err {
        MetaError::NotFound => AuthorityError::NotFound,
        MetaError::Conflict(message) => AuthorityError::Conflict { message },
        MetaError::UnsupportedSchema(message) => AuthorityError::UnsupportedSchema { message },
        MetaError::CorruptObjectBody(message) => AuthorityError::Corrupt { message },
        other => AuthorityError::Internal {
            message: other.to_string(),
        },
    }
}

fn invalid(message: impl Into<String>) -> AuthorityError {
    AuthorityError::InvalidArgument {
        message: message.into(),
    }
}

/// `lexical-v1` tokens: lowercase runs of alphanumeric characters.
#[must_use]
pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The coverage class of a source kind, in MedScale's own terms.
fn coverage_class(kind: KnowledgeSourceKind) -> &'static str {
    match kind {
        KnowledgeSourceKind::DataSnapshot => "data_snapshot",
        KnowledgeSourceKind::AnalyticsResult => "analytics_result",
        KnowledgeSourceKind::TranscriptRevision => "transcript",
        KnowledgeSourceKind::WebEvidence => "web_evidence",
    }
}

/// How each kind of source covers each note, from the clinical
/// evidence-coverage matrix. A `supports` edge from current evidence makes
/// its cell present and a `contradicts` edge from current evidence makes it
/// conflicting; evidence that was superseded leaves the cell unreviewed and
/// evidence that is no longer readable leaves it missing. Cells that disagree
/// merge to conflicting. Notes without such edges have no cells (they are in
/// `unsupported_notes`).
fn evidence_coverage(
    canvas: &CanvasRevision,
    resolutions: &[NodeResolution],
) -> (Vec<NoteCoverage>, String) {
    let notes: BTreeSet<&str> = canvas
        .nodes
        .iter()
        .filter(|n| matches!(n.content, CanvasNodeContent::Note { .. }))
        .map(|n| n.key.as_str())
        .collect();
    let mut requirements = Vec::new();
    let mut claims = HashMap::new();
    let mut classes = HashMap::new();
    for edge in &canvas.edges {
        let supports = match edge.relation {
            CanvasRelation::Supports => true,
            CanvasRelation::Contradicts => false,
            _ => continue,
        };
        if !notes.contains(edge.to.as_str()) {
            continue;
        }
        let Some(CanvasNodeContent::Evidence { span }) = canvas
            .nodes
            .iter()
            .find(|n| n.key == edge.from)
            .map(|n| &n.content)
        else {
            continue;
        };
        let Some(freshness) = resolutions
            .iter()
            .find(|r| r.key == edge.from)
            .map(|r| r.freshness)
        else {
            continue;
        };
        let source = format!("sha256:{}", span.source.content_digest.to_hex());
        let (status, fingerprints) = match (freshness, supports) {
            (Freshness::Current, true) => (CoverageStatus::Present, vec![source]),
            (Freshness::Current, false) => (CoverageStatus::Conflicting, vec![source]),
            (Freshness::Superseded, _) => (CoverageStatus::Unreviewed, vec![source]),
            (Freshness::Tombstoned, _) => (CoverageStatus::Missing, Vec::new()),
        };
        // Node keys may start with `-` or `_`, which the matrix's identifier
        // rule refuses, so claims are named `note:<key>`.
        let claim = format!("note:{}", edge.to);
        let class = coverage_class(span.source.kind);
        if let (Ok(c), Ok(k)) = (normalize_claim_id(&claim), normalize_evidence_class(class)) {
            claims.insert(c, edge.to.clone());
            classes.insert(k, class);
        }
        requirements.push(Requirement {
            claim_id: claim,
            evidence_class: class.to_owned(),
            status,
            source_fingerprints: fingerprints,
        });
    }
    let Ok(matrix) = build_coverage_matrix(requirements) else {
        return (Vec::new(), String::new());
    };
    let mut cells: Vec<NoteCoverage> = matrix
        .records
        .iter()
        .filter_map(|r| {
            Some(NoteCoverage {
                note: claims.get(&r.claim_id)?.clone(),
                evidence_class: (*classes.get(&r.evidence_class)?).to_owned(),
                state: match r.status {
                    CoverageStatus::Present => CoverageState::Present,
                    CoverageStatus::Missing => CoverageState::Missing,
                    CoverageStatus::Conflicting => CoverageState::Conflicting,
                    CoverageStatus::Unreviewed => CoverageState::Unreviewed,
                },
                source_count: u32::try_from(r.source_fingerprints.len()).unwrap_or(u32::MAX),
            })
        })
        .collect();
    cells.sort();
    (cells, matrix.matrix_hash())
}

/// Evidence nodes of one Canvas whose spans overlap within one source
/// location, found by the clinical evidence-span audit. Pairs in different
/// locations are left out: their offsets count different text.
fn evidence_overlaps(canvas: &CanvasRevision) -> Vec<EvidenceOverlap> {
    let mut keys = HashMap::new();
    let mut spans = Vec::new();
    for node in &canvas.nodes {
        if let CanvasNodeContent::Evidence { span } = &node.content
            && let Ok(audited) = span_from_ref(&node.key, span)
        {
            keys.insert(audited.evidence_id().to_owned(), node.key.clone());
            spans.push(audited);
        }
    }
    // A Canvas holds at most CANVAS_NODES_MAX nodes, below the audit's bound.
    let Ok(audit) = audit_evidence_spans(spans) else {
        return Vec::new();
    };
    let mut overlaps: Vec<EvidenceOverlap> = audit
        .overlaps
        .iter()
        .filter_map(|o| {
            let kind = match o.kind {
                OverlapKind::Exact => EvidenceOverlapKind::Exact,
                OverlapKind::Nested => EvidenceOverlapKind::Nested,
                OverlapKind::Partial => EvidenceOverlapKind::Partial,
                OverlapKind::CrossSource => return None,
            };
            let a = keys.get(o.first.evidence_id())?.clone();
            let b = keys.get(o.second.evidence_id())?.clone();
            let (first, second) = if a <= b { (a, b) } else { (b, a) };
            Some(EvidenceOverlap {
                first,
                second,
                kind,
            })
        })
        .collect();
    overlaps.sort();
    overlaps
}

/// Splits text into windows of at most `CHUNK_MAX_CHARS` characters,
/// returning `(char_start, char_end, text)`; blank windows are skipped.
#[must_use]
pub fn windows(text: &str) -> Vec<(u32, u32, String)> {
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(CHUNK_MAX_CHARS)
        .enumerate()
        .filter(|(_, w)| w.iter().any(|c| !c.is_whitespace()))
        .map(|(i, w)| {
            let start = (i * CHUNK_MAX_CHARS) as u32;
            (start, start + w.len() as u32, w.iter().collect())
        })
        .collect()
}

/// `lexical-v1` score: for each distinct query term present in the chunk,
/// `min(tf, 3) * 1000 * (n + 1) / (df + 1)`, summed, then scaled by
/// `64 / (64 + chunk tokens)`. Integer arithmetic only.
#[must_use]
pub fn score(terms: &[String], tokens: &[String], n: u64, df: &HashMap<&str, u64>) -> u64 {
    let mut total: u64 = 0;
    for term in terms {
        let tf = tokens.iter().filter(|t| *t == term).count() as u64;
        if tf == 0 {
            continue;
        }
        let d = df.get(term.as_str()).copied().unwrap_or(0);
        total += tf.min(3) * 1000 * (n + 1) / (d + 1);
    }
    total * 64 / (64 + tokens.len() as u64)
}

/// Digest pinning one transcript revision's content.
fn transcript_digest(segments: &[medscale_contracts::audio::TranscriptSegment]) -> DigestSha256 {
    DigestSha256::of(&serde_json::to_vec(segments).unwrap_or_default())
}

/// Numeric suffix of a `prefix-N` id (ids come from durable sequences, so
/// a larger suffix is a later object).
fn id_seq(id: &OpaqueId) -> u64 {
    id.as_str()
        .rsplit('-')
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

fn cell_text(cell: &CellValue) -> Option<&str> {
    match cell {
        CellValue::Text(t) => Some(t.as_str()),
        _ => None,
    }
}

/// Every span of one source revision, in natural order.
type Spans = Vec<(SpanLocator, String)>;

impl DataSources<'_> {
    fn knowledge_header(&self, id: OpaqueId) -> ObjectHeader {
        ObjectHeader {
            id,
            schema_version: KNOWLEDGE_SCHEMA_VERSION,
            realm_id: self.realm.clone(),
            authority_scope_id: self.scope.clone(),
        }
    }

    fn in_scope(&self, header: &ObjectHeader) -> bool {
        header.realm_id == self.realm && header.authority_scope_id == self.scope
    }

    fn knowledge_in_scope(&self, header: &ObjectHeader) -> Result<(), AuthorityError> {
        if self.in_scope(header) {
            Ok(())
        } else {
            Err(AuthorityError::WrongScope)
        }
    }

    /// The latest snapshot id of an active data source of this Project.
    fn latest_snapshot(&self, source_id: &OpaqueId) -> Result<Option<OpaqueId>, AuthorityError> {
        let mut after: Option<String> = None;
        let mut best: Option<OpaqueId> = None;
        loop {
            let (page, next) = self
                .meta()
                .list_snapshots(source_id, 100, after.as_deref())
                .map_err(meta_err)?;
            for s in page {
                if best
                    .as_ref()
                    .is_none_or(|b| id_seq(&s.snapshot_id) > id_seq(b))
                {
                    best = Some(s.snapshot_id);
                }
            }
            match next {
                Some(cursor) => after = Some(cursor),
                None => return Ok(best),
            }
        }
    }

    /// The live, current revision of every indexable source of a Project.
    fn current_sources(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<IndexSourceRef>, AuthorityError> {
        let meta = self.meta();
        let mut out = Vec::new();
        let mut after: Option<String> = None;
        loop {
            let (page, next) = meta
                .list_data_sources(
                    &self.scope,
                    project_id,
                    Some(SourceStatus::Active),
                    100,
                    after.as_deref(),
                )
                .map_err(meta_err)?;
            for source in page {
                if let Some(snap_id) = self.latest_snapshot(&source.source_id)? {
                    let record = self.scoped_snapshot(&snap_id)?;
                    out.push(IndexSourceRef {
                        kind: KnowledgeSourceKind::DataSnapshot,
                        object_id: snap_id,
                        content_digest: record.snapshot.content_digest.clone(),
                        lineage_id: source.source_id.clone(),
                    });
                }
            }
            match next {
                Some(cursor) => after = Some(cursor),
                None => break,
            }
        }
        for audio in meta.list_audio_sources(project_id).map_err(meta_err)? {
            if !self.in_scope(&audio.header) {
                continue;
            }
            let revisions = meta
                .list_transcript_revisions(&audio.header.id)
                .map_err(meta_err)?;
            if let Some(latest) = revisions.iter().max_by_key(|r| r.revision_no) {
                out.push(IndexSourceRef {
                    kind: KnowledgeSourceKind::TranscriptRevision,
                    object_id: latest.header.id.clone(),
                    content_digest: transcript_digest(&latest.segments),
                    lineage_id: audio.header.id.clone(),
                });
            }
        }
        for session in meta.list_browse_sessions(project_id).map_err(meta_err)? {
            if !self.in_scope(&session.header) {
                continue;
            }
            for (item, _) in meta
                .list_browse_evidence(&session.header.id)
                .map_err(meta_err)?
            {
                out.push(IndexSourceRef {
                    kind: KnowledgeSourceKind::WebEvidence,
                    object_id: item.header.id.clone(),
                    content_digest: item.content_digest.clone(),
                    lineage_id: session.header.id.clone(),
                });
            }
        }
        for receipt in meta.list_query_receipts(project_id).map_err(meta_err)? {
            if let Some(result_id) = &receipt.result_id {
                let (table, _) = meta.get_derived_table(result_id).map_err(meta_err)?;
                if self.in_scope(&table.header) {
                    out.push(IndexSourceRef {
                        kind: KnowledgeSourceKind::AnalyticsResult,
                        object_id: result_id.clone(),
                        content_digest: table.content_digest.clone(),
                        lineage_id: result_id.clone(),
                    });
                }
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// Reads one pinned revision if it is still readable in this Project
    /// with the pinned digest: `(freshness, spans)`. `Tombstoned` carries no
    /// spans. Any lookup failure is a tombstone, never an error, so a
    /// broken source cannot hide the rest of the Project.
    fn read_source(
        &self,
        project_id: &OpaqueId,
        source: &IndexSourceRef,
        with_spans: bool,
    ) -> (Freshness, Spans) {
        self.try_read_source(project_id, source, with_spans)
            .unwrap_or((Freshness::Tombstoned, Vec::new()))
    }

    fn try_read_source(
        &self,
        project_id: &OpaqueId,
        source: &IndexSourceRef,
        with_spans: bool,
    ) -> Option<(Freshness, Spans)> {
        let meta = self.meta();
        let mut spans = Vec::new();
        let freshness = match source.kind {
            KnowledgeSourceKind::DataSnapshot => {
                let record = self.scoped_snapshot(&source.object_id).ok()?;
                let data_source = meta.get_data_source(&source.lineage_id).ok()?;
                if &record.project_id != project_id
                    || record.snapshot.source_id != source.lineage_id
                    || record.snapshot.content_digest != source.content_digest
                    || data_source.status != SourceStatus::Active
                    || !self.in_scope(&data_source.header)
                {
                    return None;
                }
                if with_spans {
                    let doc = self.load_table(&record).ok()?;
                    for (row, cells) in doc.rows.iter().enumerate() {
                        for (field, cell) in doc.fields.iter().zip(cells) {
                            if let Some(text) = cell_text(cell) {
                                spans.push((
                                    SpanLocator::Cell {
                                        row: row as u64,
                                        column: field.name.clone(),
                                    },
                                    text.to_owned(),
                                ));
                            }
                        }
                    }
                }
                let latest = self.latest_snapshot(&source.lineage_id).ok()??;
                if latest == source.object_id {
                    Freshness::Current
                } else {
                    Freshness::Superseded
                }
            }
            KnowledgeSourceKind::TranscriptRevision => {
                let revision = meta.get_transcript_revision(&source.object_id).ok()?;
                if !self.in_scope(&revision.header)
                    || &revision.project_id != project_id
                    || revision.source_id != source.lineage_id
                    || transcript_digest(&revision.segments) != source.content_digest
                {
                    return None;
                }
                if with_spans {
                    for segment in &revision.segments {
                        if let Some(text) = &segment.text {
                            spans.push((
                                SpanLocator::Segment {
                                    seq: segment.seq,
                                    start_ms: segment.start_ms,
                                    end_ms: segment.end_ms,
                                },
                                text.clone(),
                            ));
                        }
                    }
                }
                let newest = meta
                    .list_transcript_revisions(&source.lineage_id)
                    .ok()?
                    .into_iter()
                    .map(|r| r.revision_no)
                    .max()?;
                if newest == revision.revision_no {
                    Freshness::Current
                } else {
                    Freshness::Superseded
                }
            }
            KnowledgeSourceKind::WebEvidence => {
                let session = meta.get_browse_session(&source.lineage_id).ok()?;
                if !self.in_scope(&session.header) || &session.project_id != project_id {
                    return None;
                }
                let (item, _) = meta
                    .list_browse_evidence(&session.header.id)
                    .ok()?
                    .into_iter()
                    .find(|(item, _)| item.header.id == source.object_id)?;
                if item.content_digest != source.content_digest {
                    return None;
                }
                if with_spans {
                    spans.push((SpanLocator::Excerpt, item.excerpt));
                }
                Freshness::Current
            }
            KnowledgeSourceKind::AnalyticsResult => {
                let (table, doc) = meta.get_derived_table(&source.object_id).ok()?;
                if !self.in_scope(&table.header)
                    || &table.project_id != project_id
                    || table.content_digest != source.content_digest
                {
                    return None;
                }
                if with_spans {
                    for (row, cells) in doc.rows.iter().enumerate() {
                        for (column, cell) in doc.columns.iter().zip(cells) {
                            if let Some(text) = cell_text(cell) {
                                spans.push((
                                    SpanLocator::Cell {
                                        row: row as u64,
                                        column: column.name.clone(),
                                    },
                                    text.to_owned(),
                                ));
                            }
                        }
                    }
                }
                Freshness::Current
            }
        };
        Some((freshness, spans))
    }

    /// Freshness of every source an index version names, plus the live
    /// sources it does not include.
    fn index_status(
        &self,
        manifest: &IndexManifest,
    ) -> Result<(IndexStatus, BTreeMap<IndexSourceRef, Freshness>), AuthorityError> {
        let mut freshness = BTreeMap::new();
        let (mut current, mut superseded, mut tombstoned) = (0_u32, 0_u32, 0_u32);
        for source in &manifest.sources {
            let (f, _) = self.read_source(&manifest.project_id, source, false);
            match f {
                Freshness::Current => current += 1,
                Freshness::Superseded => superseded += 1,
                Freshness::Tombstoned => tombstoned += 1,
            }
            freshness.insert(source.clone(), f);
        }
        let indexed: BTreeSet<&IndexSourceRef> = manifest.sources.iter().collect();
        let unindexed = self
            .current_sources(&manifest.project_id)?
            .iter()
            .filter(|s| !indexed.contains(s))
            .count() as u32;
        Ok((
            IndexStatus {
                manifest_id: manifest.header.id.clone(),
                version: manifest.version,
                chunk_count: manifest.chunk_count,
                current_sources: current,
                superseded_sources: superseded,
                tombstoned_sources: tombstoned,
                unindexed_sources: unindexed,
            },
            freshness,
        ))
    }

    /// Builds the next index version from the Project's current sources.
    /// Deterministic: the same sources always give the same chunk digest.
    pub fn knowledge_index_build(
        &mut self,
        project_id: &OpaqueId,
    ) -> Result<(IndexManifest, IndexStatus), AuthorityError> {
        self.require_project(project_id)?;
        let sources = self.current_sources(project_id)?;
        let mut chunks: Vec<IndexChunk> = Vec::new();
        for source in &sources {
            let (freshness, spans) = self.read_source(project_id, source, true);
            if freshness != Freshness::Current {
                return Err(AuthorityError::Conflict {
                    message: "a source changed while the index was being built".to_owned(),
                });
            }
            for (locator, text) in spans {
                for (start, end, window) in windows(&text) {
                    if chunks.len() >= INDEX_CHUNKS_MAX {
                        return Err(invalid(format!(
                            "the Project has more than {INDEX_CHUNKS_MAX} index chunks"
                        )));
                    }
                    chunks.push(IndexChunk {
                        seq: chunks.len() as u32 + 1,
                        span: EvidenceSpanRef {
                            source: source.clone(),
                            locator: locator.clone(),
                            char_start: start,
                            char_end: end,
                        },
                        text: window,
                    });
                }
            }
        }
        let previous = self
            .meta()
            .latest_index_manifest(project_id)
            .map_err(meta_err)?;
        let manifest = IndexManifest {
            header: self.knowledge_header(
                self.meta()
                    .alloc_knowledge_id("knowledge-index")
                    .map_err(meta_err)?,
            ),
            project_id: project_id.clone(),
            version: previous.map_or(1, |m| m.version + 1),
            tokenizer: LEXICAL_TOKENIZER.to_owned(),
            sources,
            chunk_count: chunks.len() as u32,
            chunks_digest: chunks_digest(&chunks),
        };
        self.meta()
            .commit_index_version(&manifest, &chunks)
            .map_err(meta_err)?;
        self.audit("knowledge.index.build", vec![manifest.header.id.clone()])?;
        let (status, _) = self.index_status(&manifest)?;
        Ok((manifest, status))
    }

    /// Freshness of the Project's latest index version, if it has one.
    pub fn knowledge_index_status(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Option<IndexStatus>, AuthorityError> {
        self.require_project(project_id)?;
        match self
            .meta()
            .latest_index_manifest(project_id)
            .map_err(meta_err)?
        {
            Some(manifest) => Ok(Some(self.index_status(&manifest)?.0)),
            None => Ok(None),
        }
    }

    /// Reads one span's text from its pinned revision if still readable.
    /// `cache` holds each source's spans for the length of one request.
    fn span_text(
        &self,
        project_id: &OpaqueId,
        span: &EvidenceSpanRef,
        cache: &mut HashMap<IndexSourceRef, (Freshness, Spans)>,
    ) -> (Freshness, Option<String>) {
        let (freshness, spans) = cache
            .entry(span.source.clone())
            .or_insert_with(|| self.read_source(project_id, &span.source, true));
        if *freshness == Freshness::Tombstoned {
            return (Freshness::Tombstoned, None);
        }
        let text = spans
            .iter()
            .find(|(locator, _)| *locator == span.locator)
            .and_then(|(_, text)| {
                let chars: Vec<char> = text.chars().collect();
                chars
                    .get(span.char_start as usize..span.char_end as usize)
                    .map(|w| w.iter().collect::<String>())
            });
        match text {
            Some(text) => (*freshness, Some(text)),
            // The digest matched, so a span that does not resolve was never
            // part of this revision.
            None => (Freshness::Tombstoned, None),
        }
    }

    fn plan(request: &RetrievalRequest) -> Result<RetrievalPlan, RetrievalDenyReason> {
        if request.query.chars().count() > QUERY_MAX_CHARS {
            return Err(RetrievalDenyReason::QueryTooLong);
        }
        let mut terms: Vec<String> = Vec::new();
        for t in tokenize(&request.query) {
            if !terms.contains(&t) {
                terms.push(t);
            }
        }
        if terms.is_empty() {
            return Err(RetrievalDenyReason::EmptyQuery);
        }
        if terms.len() > QUERY_TERMS_MAX {
            return Err(RetrievalDenyReason::TooManyTerms);
        }
        let max_hits = request.max_hits.unwrap_or(HITS_DEFAULT);
        if max_hits == 0 || max_hits > HITS_MAX {
            return Err(RetrievalDenyReason::BadHitLimit);
        }
        Ok(RetrievalPlan {
            stages: vec![RetrievalStage::Lexical],
            terms,
            max_hits,
            include_stale: request.include_stale,
        })
    }

    /// Runs one lexical retrieval over the Project's latest index version
    /// and records its receipt. Stale spans are never shown as current.
    pub fn knowledge_search(
        &mut self,
        request: RetrievalRequest,
    ) -> Result<RetrievalResult, AuthorityError> {
        self.require_project(&request.project_id)?;
        let project_id = request.project_id.clone();
        let query: String = request.query.chars().take(QUERY_MAX_CHARS).collect();
        let receipt_id = self
            .meta()
            .alloc_knowledge_id("knowledge-receipt")
            .map_err(meta_err)?;
        let mut receipt = RetrievalReceipt {
            header: self.knowledge_header(receipt_id.clone()),
            project_id: project_id.clone(),
            query_digest: DigestSha256::of(query.as_bytes()),
            query,
            plan: None,
            manifest_id: None,
            manifest_chunks_digest: None,
            index_status: None,
            outcome: RetrievalOutcome::Denied,
            deny_reason: None,
            hits: Vec::new(),
        };
        let manifest = self
            .meta()
            .latest_index_manifest(&project_id)
            .map_err(meta_err)?;
        let mut shown = Vec::new();
        match (Self::plan(&request), manifest) {
            (Err(reason), _) => receipt.deny_reason = Some(reason),
            (Ok(_), None) => receipt.deny_reason = Some(RetrievalDenyReason::NoIndex),
            (Ok(plan), Some(manifest)) => {
                let chunks = self.meta().get_index_chunks(&manifest).map_err(meta_err)?;
                let (status, freshness) = self.index_status(&manifest)?;
                let tokens: Vec<Vec<String>> = chunks.iter().map(|c| tokenize(&c.text)).collect();
                let mut df: HashMap<&str, u64> = HashMap::new();
                for term in &plan.terms {
                    let d = tokens.iter().filter(|t| t.contains(term)).count() as u64;
                    df.insert(term.as_str(), d);
                }
                let n = chunks.len() as u64;
                let mut scored: Vec<(u64, &IndexChunk, Freshness)> = chunks
                    .iter()
                    .zip(&tokens)
                    .filter_map(|(chunk, toks)| {
                        let s = score(&plan.terms, toks, n, &df);
                        let f = freshness
                            .get(&chunk.span.source)
                            .copied()
                            .unwrap_or(Freshness::Tombstoned);
                        (s > 0 && (plan.include_stale || f == Freshness::Current))
                            .then_some((s, chunk, f))
                    })
                    .collect();
                scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.seq.cmp(&b.1.seq)));
                scored.truncate(plan.max_hits as usize);
                let mut cache = HashMap::new();
                for (i, (s, chunk, f)) in scored.into_iter().enumerate() {
                    let hit = ReceiptHit {
                        rank: i as u32 + 1,
                        chunk_seq: chunk.seq,
                        score: s,
                        span: chunk.span.clone(),
                        freshness: f,
                    };
                    let text = if f == Freshness::Tombstoned {
                        None
                    } else {
                        self.span_text(&project_id, &chunk.span, &mut cache).1
                    };
                    receipt.hits.push(hit.clone());
                    shown.push(RetrievalHit { hit, text });
                }
                receipt.outcome = if receipt
                    .hits
                    .iter()
                    .any(|h| h.freshness == Freshness::Current)
                {
                    RetrievalOutcome::Results
                } else {
                    RetrievalOutcome::InsufficientEvidence
                };
                receipt.plan = Some(plan);
                receipt.manifest_id = Some(manifest.header.id.clone());
                receipt.manifest_chunks_digest = Some(manifest.chunks_digest.clone());
                receipt.index_status = Some(status);
            }
        }
        receipt.validate().map_err(|e| AuthorityError::Internal {
            message: format!("retrieval receipt invariant: {e}"),
        })?;
        self.meta()
            .insert_retrieval_receipt(&receipt)
            .map_err(meta_err)?;
        self.audit("knowledge.search", vec![receipt_id])?;
        Ok(RetrievalResult {
            receipt,
            hits: shown,
        })
    }

    pub fn knowledge_receipt(&self, id: &OpaqueId) -> Result<RetrievalReceipt, AuthorityError> {
        let r = self.meta().get_retrieval_receipt(id).map_err(meta_err)?;
        self.knowledge_in_scope(&r.header)?;
        Ok(r)
    }

    pub fn knowledge_receipts(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<RetrievalReceipt>, AuthorityError> {
        self.require_project(project_id)?;
        self.meta()
            .list_retrieval_receipts(project_id)
            .map_err(meta_err)
    }

    /// Creates an empty Canvas (revision 1).
    pub fn canvas_create(
        &mut self,
        project_id: OpaqueId,
        title: String,
    ) -> Result<CanvasRevision, AuthorityError> {
        self.require_project(&project_id)?;
        let mut canvas = CanvasRevision {
            header: self.knowledge_header(OpaqueId::new("pending")),
            project_id,
            revision: 1,
            title,
            nodes: Vec::new(),
            edges: Vec::new(),
        };
        canvas.validate().map_err(invalid)?;
        canvas.header =
            self.knowledge_header(self.meta().alloc_knowledge_id("canvas").map_err(meta_err)?);
        self.meta()
            .insert_canvas_revision(&canvas)
            .map_err(meta_err)?;
        self.audit("knowledge.canvas.create", vec![canvas.header.id.clone()])?;
        Ok(canvas)
    }

    fn scoped_canvas(
        &self,
        id: &OpaqueId,
        revision: Option<u32>,
    ) -> Result<CanvasRevision, AuthorityError> {
        let canvas = self.meta().get_canvas(id, revision).map_err(meta_err)?;
        self.knowledge_in_scope(&canvas.header)?;
        Ok(canvas)
    }

    /// Applies a batch of edits to the latest revision, which must be
    /// `expected_revision`, writing revision n+1. New evidence must resolve
    /// to a current span of this Project.
    pub fn canvas_edit(
        &mut self,
        canvas_id: &OpaqueId,
        expected_revision: u32,
        ops: Vec<CanvasOp>,
    ) -> Result<CanvasRevision, AuthorityError> {
        if ops.is_empty() || ops.len() > CANVAS_OPS_MAX {
            return Err(invalid(format!("send 1-{CANVAS_OPS_MAX} canvas edits")));
        }
        let latest = self.scoped_canvas(canvas_id, None)?;
        self.require_project(&latest.project_id)?;
        if latest.revision != expected_revision {
            return Err(AuthorityError::Conflict {
                message: format!(
                    "canvas is at revision {}, not {expected_revision}",
                    latest.revision
                ),
            });
        }
        let mut next = latest.clone();
        next.revision += 1;
        let mut cache = HashMap::new();
        for op in &ops {
            if let CanvasOp::AddEvidence { span, .. } = op {
                span.validate().map_err(invalid)?;
                let (freshness, text) = self.span_text(&latest.project_id, span, &mut cache);
                if freshness != Freshness::Current || text.is_none() {
                    return Err(invalid(
                        "evidence must be a current span of this Project".to_owned(),
                    ));
                }
            }
            op.apply(&mut next).map_err(invalid)?;
        }
        next.validate().map_err(invalid)?;
        self.meta()
            .insert_canvas_revision(&next)
            .map_err(meta_err)?;
        self.audit("knowledge.canvas.edit", vec![next.header.id.clone()])?;
        Ok(next)
    }

    /// One Canvas revision (latest by default) with every evidence node
    /// resolved live and the contradiction/missing-evidence inspection.
    pub fn canvas_view(
        &self,
        canvas_id: &OpaqueId,
        revision: Option<u32>,
    ) -> Result<CanvasView, AuthorityError> {
        let canvas = self.scoped_canvas(canvas_id, revision)?;
        self.require_project(&canvas.project_id)?;
        let mut cache = HashMap::new();
        let mut resolutions = Vec::new();
        let mut current_evidence = BTreeSet::new();
        let mut superseded_evidence = Vec::new();
        let mut missing_evidence = Vec::new();
        for node in &canvas.nodes {
            if let CanvasNodeContent::Evidence { span } = &node.content {
                let (freshness, text) = self.span_text(&canvas.project_id, span, &mut cache);
                match freshness {
                    Freshness::Current => {
                        current_evidence.insert(node.key.clone());
                    }
                    Freshness::Superseded => superseded_evidence.push(node.key.clone()),
                    Freshness::Tombstoned => missing_evidence.push(node.key.clone()),
                }
                resolutions.push(NodeResolution {
                    key: node.key.clone(),
                    freshness,
                    text,
                });
            }
        }
        let unsupported_notes = canvas
            .nodes
            .iter()
            .filter(|n| matches!(n.content, CanvasNodeContent::Note { .. }))
            .filter(|n| {
                !canvas.edges.iter().any(|e| {
                    e.to == n.key
                        && e.relation == CanvasRelation::Supports
                        && current_evidence.contains(&e.from)
                })
            })
            .map(|n| n.key.clone())
            .collect();
        let contradictions = canvas
            .edges
            .iter()
            .filter(|e| e.relation == CanvasRelation::Contradicts)
            .cloned()
            .collect();
        let overlapping_evidence = evidence_overlaps(&canvas);
        let (evidence_coverage, coverage_hash) = evidence_coverage(&canvas, &resolutions);
        Ok(CanvasView {
            canvas,
            resolutions,
            contradictions,
            unsupported_notes,
            superseded_evidence,
            missing_evidence,
            overlapping_evidence,
            evidence_coverage,
            coverage_hash,
        })
    }

    pub fn canvas_list(&self, project_id: &OpaqueId) -> Result<Vec<CanvasSummary>, AuthorityError> {
        self.require_project(project_id)?;
        Ok(self
            .meta()
            .list_canvases(project_id)
            .map_err(meta_err)?
            .into_iter()
            .filter(|c| self.in_scope(&c.header))
            .map(|c| CanvasSummary {
                canvas_id: c.header.id,
                title: c.title,
                revision: c.revision,
                node_count: c.nodes.len() as u32,
                edge_count: c.edges.len() as u32,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_and_windows_are_deterministic() {
        assert_eq!(
            tokenize("LDL-C fell; ldl rose!"),
            vec!["ldl", "c", "fell", "ldl", "rose"]
        );
        assert!(tokenize(" ,; ").is_empty());
        assert!(windows("   ").is_empty());
        let long = "a".repeat(CHUNK_MAX_CHARS * 2 + 5);
        let w = windows(&long);
        assert_eq!(w.len(), 3);
        assert_eq!((w[0].0, w[0].1), (0, CHUNK_MAX_CHARS as u32));
        assert_eq!(
            (w[2].0, w[2].1),
            (2 * CHUNK_MAX_CHARS as u32, 2 * CHUNK_MAX_CHARS as u32 + 5)
        );
        // Multi-byte text is windowed by characters, not bytes.
        let arabic = "\u{627}\u{644}\u{62f}\u{645}".repeat(300);
        let w = windows(&arabic);
        assert_eq!(w.len(), 2);
        assert_eq!(w[0].2.chars().count(), CHUNK_MAX_CHARS);
    }

    /// Hand-computed: n = 4 chunks, "ldl" in 2 of them (df 2), "statin" in
    /// 1 (df 1).
    #[test]
    fn scores_match_hand_computed_values() {
        let terms = vec!["ldl".to_owned(), "statin".to_owned()];
        let mut df = HashMap::new();
        df.insert("ldl", 2_u64);
        df.insert("statin", 1_u64);
        // tf(ldl) = 1, 4 tokens: 1 * 1000 * 5 / 3 = 1666; 1666 * 64 / 68 = 1568.
        let one = tokenize("ldl was very high");
        assert_eq!(score(&terms, &one, 4, &df), 1568);
        // tf(ldl) = 1, tf(statin) = 1, 3 tokens:
        // 1666 + 1000 * 5 / 2 = 1666 + 2500 = 4166; * 64 / 67 = 3979.
        let both = tokenize("statin lowered ldl");
        assert_eq!(score(&terms, &both, 4, &df), 3979);
        // tf capped at 3: tf(ldl) = 5 counts as 3; 5 tokens:
        // 3 * 1000 * 5 / 3 = 5000; 5000 * 64 / 69 = 4637.
        let many = tokenize("ldl ldl ldl ldl ldl");
        assert_eq!(score(&terms, &many, 4, &df), 4637);
        assert_eq!(score(&terms, &tokenize("nothing here"), 4, &df), 0);
    }

    #[test]
    fn id_sequence_orders_numerically() {
        assert!(id_seq(&OpaqueId::new("snap-10")) > id_seq(&OpaqueId::new("snap-9")));
        assert_eq!(id_seq(&OpaqueId::new("odd")), 0);
    }
}
