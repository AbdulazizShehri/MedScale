//! Knowledge + Research Canvas contracts (Spec 083).
//!
//! A per-Project lexical index over exact, pinned source revisions (Spec 075
//! snapshots, Spec 080 web evidence, Spec 081 transcript revisions, Spec 082
//! analytics results). The index is a rebuildable projection, never a source
//! of truth: every hit resolves to an `EvidenceSpanRef` naming the exact
//! object, content digest and span it came from. Relevance is not
//! authority. Vector retrieval is not admitted (decision Q28/Q29), so every
//! plan is lexical only. A Research Canvas holds live references to evidence
//! spans, never silent copies of their text.

use serde::{Deserialize, Serialize};

use crate::objects::{DigestSha256, ObjectHeader, OpaqueId};

/// Durable schema version for every Knowledge object (Spec 083 v1).
pub const KNOWLEDGE_SCHEMA_VERSION: u32 = 1;

/// The only tokenizer/scorer this build has: lowercase alphanumeric tokens,
/// integer TF-IDF with length normalization (deterministic on every
/// platform; no floating point).
pub const LEXICAL_TOKENIZER: &str = "lexical-v1";

pub const QUERY_MAX_CHARS: usize = 512;
pub const QUERY_TERMS_MAX: usize = 32;
pub const HITS_MAX: u32 = 50;
pub const HITS_DEFAULT: u32 = 10;
/// Chunks are windows of at most this many characters of one span.
pub const CHUNK_MAX_CHARS: usize = 1_000;
/// Upper bound on chunks in one index version.
pub const INDEX_CHUNKS_MAX: usize = 100_000;
pub const TITLE_MAX_CHARS: usize = 200;
pub const NOTE_MAX_CHARS: usize = 4_000;
pub const NODE_KEY_MAX_CHARS: usize = 32;
pub const CANVAS_NODES_MAX: usize = 500;
pub const CANVAS_EDGES_MAX: usize = 2_000;
pub const CANVAS_OPS_MAX: usize = 64;
pub const COLUMN_NAME_MAX_CHARS: usize = 128;

macro_rules! closed_vocabulary {
    ($name:ident, $what:literal, { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            pub fn parse(value: &str) -> Result<Self, String> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    other => Err(format!(concat!("unknown ", $what, " {}"), other)),
                }
            }
        }
    };
}

/// Which kind of object an indexed span comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeSourceKind {
    /// A Spec 075 data snapshot (text cells).
    DataSnapshot,
    /// A Spec 081 transcript revision (segments with time spans).
    TranscriptRevision,
    /// A Spec 080 web evidence item (its stored excerpt).
    WebEvidence,
    /// A Spec 082 derived table (text cells).
    AnalyticsResult,
}

closed_vocabulary!(KnowledgeSourceKind, "knowledge source kind", {
    DataSnapshot => "data_snapshot",
    TranscriptRevision => "transcript_revision",
    WebEvidence => "web_evidence",
    AnalyticsResult => "analytics_result",
});

/// The exact revision of one indexed object.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexSourceRef {
    pub kind: KnowledgeSourceKind,
    /// The snapshot, transcript revision, evidence item or derived table.
    pub object_id: OpaqueId,
    /// Content identity of that exact revision (for a transcript revision,
    /// the digest of its canonical segment list).
    pub content_digest: DigestSha256,
    /// The object whose newer revisions supersede this one: the data
    /// source or the audio source. For objects that are never superseded it
    /// is their container (the browse session of web evidence) or the
    /// object itself (derived tables).
    pub lineage_id: OpaqueId,
}

/// Where inside the source the span sits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
pub enum SpanLocator {
    /// A table cell (0-based row, column name).
    Cell { row: u64, column: String },
    /// A transcript segment and its time span.
    Segment {
        seq: u32,
        start_ms: u64,
        end_ms: u64,
    },
    /// The stored excerpt of a web evidence item.
    Excerpt,
}

/// An exact, resolvable evidence span: object revision, location, and the
/// character range `[char_start, char_end)` of that location's text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSpanRef {
    pub source: IndexSourceRef,
    pub locator: SpanLocator,
    pub char_start: u32,
    pub char_end: u32,
}

impl EvidenceSpanRef {
    pub fn validate(&self) -> Result<(), String> {
        if self.char_start >= self.char_end {
            return Err("an evidence span must be non-empty".to_owned());
        }
        match &self.locator {
            SpanLocator::Cell { column, .. } => {
                if column.is_empty() || column.chars().count() > COLUMN_NAME_MAX_CHARS {
                    return Err("span column name out of bounds".to_owned());
                }
                if !matches!(
                    self.source.kind,
                    KnowledgeSourceKind::DataSnapshot | KnowledgeSourceKind::AnalyticsResult
                ) {
                    return Err("a cell span needs a table source".to_owned());
                }
            }
            SpanLocator::Segment {
                start_ms, end_ms, ..
            } => {
                if start_ms >= end_ms {
                    return Err("a segment span needs start < end".to_owned());
                }
                if self.source.kind != KnowledgeSourceKind::TranscriptRevision {
                    return Err("a segment span needs a transcript source".to_owned());
                }
            }
            SpanLocator::Excerpt => {
                if self.source.kind != KnowledgeSourceKind::WebEvidence {
                    return Err("an excerpt span needs a web evidence source".to_owned());
                }
            }
        }
        Ok(())
    }
}

/// Whether an indexed or referenced revision is still the live one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// The revision is readable, its digest matches, and it is the latest
    /// revision of its lineage.
    Current,
    /// Readable and unchanged, but a newer revision of its lineage exists.
    Superseded,
    /// No longer readable in this Project: archived, missing, moved out of
    /// scope, or its content no longer matches the pinned digest.
    Tombstoned,
}

closed_vocabulary!(Freshness, "freshness", {
    Current => "current",
    Superseded => "superseded",
    Tombstoned => "tombstoned",
});

/// One indexed window of text. Stored with its index version; the text is
/// a derived copy used only for matching and is re-resolved from the live
/// source before it is shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexChunk {
    /// 1-based, contiguous within the index version.
    pub seq: u32,
    pub span: EvidenceSpanRef,
    pub text: String,
}

impl IndexChunk {
    pub fn validate(&self) -> Result<(), String> {
        self.span.validate()?;
        let n = self.text.chars().count();
        if n == 0 || n > CHUNK_MAX_CHARS {
            return Err("chunk text out of bounds".to_owned());
        }
        if u32::try_from(n).ok() != self.span.char_end.checked_sub(self.span.char_start) {
            return Err("chunk text disagrees with its span".to_owned());
        }
        Ok(())
    }
}

/// Canonical digest of an index version's chunk list.
#[must_use]
pub fn chunks_digest(chunks: &[IndexChunk]) -> DigestSha256 {
    DigestSha256::of(&serde_json::to_vec(chunks).unwrap_or_default())
}

/// One built index version for a Project. Immutable once written; a rebuild
/// writes version n+1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexManifest {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    /// 1-based, contiguous per Project.
    pub version: u32,
    pub tokenizer: String,
    /// Every source revision indexed, sorted, without duplicates.
    pub sources: Vec<IndexSourceRef>,
    pub chunk_count: u32,
    /// `chunks_digest` of the stored chunks.
    pub chunks_digest: DigestSha256,
}

impl IndexManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.version == 0 {
            return Err("index versions start at 1".to_owned());
        }
        if self.tokenizer != LEXICAL_TOKENIZER {
            return Err("unknown tokenizer".to_owned());
        }
        if self.chunk_count as usize > INDEX_CHUNKS_MAX {
            return Err("index exceeds its chunk bound".to_owned());
        }
        if self.sources.windows(2).any(|w| w[0] >= w[1]) {
            return Err("index sources must be sorted and unique".to_owned());
        }
        Ok(())
    }
}

/// Freshness of an index version against the live Project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexStatus {
    pub manifest_id: OpaqueId,
    pub version: u32,
    pub chunk_count: u32,
    pub current_sources: u32,
    pub superseded_sources: u32,
    pub tombstoned_sources: u32,
    /// Live source revisions that this version does not include.
    pub unindexed_sources: u32,
}

impl IndexStatus {
    /// True only when every indexed source is current and nothing is
    /// missing from the index.
    #[must_use]
    pub const fn is_fresh(&self) -> bool {
        self.superseded_sources == 0 && self.tombstoned_sources == 0 && self.unindexed_sources == 0
    }
}

/// Retrieval stages. Only lexical matching exists in this build; vector
/// retrieval is not admitted (Q28/Q29) and is never silently substituted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalStage {
    Lexical,
}

closed_vocabulary!(RetrievalStage, "retrieval stage", {
    Lexical => "lexical",
});

/// A retrieval request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrievalRequest {
    pub project_id: OpaqueId,
    pub query: String,
    pub max_hits: Option<u32>,
    /// Also list superseded and tombstoned matches (never with text).
    pub include_stale: bool,
}

/// The plan Core ran for a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrievalPlan {
    pub stages: Vec<RetrievalStage>,
    /// Distinct lowercase query terms, in query order.
    pub terms: Vec<String>,
    pub max_hits: u32,
    pub include_stale: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalOutcome {
    /// At least one current span matched.
    Results,
    /// No current span matched: a first-class answer, never an empty
    /// success.
    InsufficientEvidence,
    Denied,
}

closed_vocabulary!(RetrievalOutcome, "retrieval outcome", {
    Results => "results",
    InsufficientEvidence => "insufficient_evidence",
    Denied => "denied",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalDenyReason {
    /// The query has no searchable term.
    EmptyQuery,
    QueryTooLong,
    TooManyTerms,
    BadHitLimit,
    /// The Project has no index yet.
    NoIndex,
}

closed_vocabulary!(RetrievalDenyReason, "retrieval deny reason", {
    EmptyQuery => "empty_query",
    QueryTooLong => "query_too_long",
    TooManyTerms => "too_many_terms",
    BadHitLimit => "bad_hit_limit",
    NoIndex => "no_index",
});

/// One ranked match as recorded on the receipt (no text).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptHit {
    /// 1-based rank.
    pub rank: u32,
    pub chunk_seq: u32,
    pub score: u64,
    pub span: EvidenceSpanRef,
    pub freshness: Freshness,
}

/// Every retrieval request that reaches a Project leaves one receipt.
/// Immutable once written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrievalReceipt {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub query: String,
    pub query_digest: DigestSha256,
    pub plan: Option<RetrievalPlan>,
    pub manifest_id: Option<OpaqueId>,
    pub manifest_chunks_digest: Option<DigestSha256>,
    pub index_status: Option<IndexStatus>,
    pub outcome: RetrievalOutcome,
    pub deny_reason: Option<RetrievalDenyReason>,
    pub hits: Vec<ReceiptHit>,
}

impl RetrievalReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if self.query.chars().count() > QUERY_MAX_CHARS {
            return Err("receipt query exceeds bound".to_owned());
        }
        if self.query_digest != DigestSha256::of(self.query.as_bytes()) {
            return Err("receipt query digest mismatch".to_owned());
        }
        if (self.outcome == RetrievalOutcome::Denied) != self.deny_reason.is_some() {
            return Err("exactly denied receipts carry a deny reason".to_owned());
        }
        let ran = self.outcome != RetrievalOutcome::Denied;
        if ran
            != (self.plan.is_some()
                && self.manifest_id.is_some()
                && self.manifest_chunks_digest.is_some()
                && self.index_status.is_some())
        {
            return Err("exactly receipts that ran name their plan and index".to_owned());
        }
        if !ran && !self.hits.is_empty() {
            return Err("a denied receipt has no hits".to_owned());
        }
        let current = self.hits.iter().any(|h| h.freshness == Freshness::Current);
        if ran && (self.outcome == RetrievalOutcome::Results) != current {
            return Err("results exactly when a current span matched".to_owned());
        }
        if let Some(plan) = &self.plan {
            if self.hits.len() > plan.max_hits as usize {
                return Err("more hits than the plan allows".to_owned());
            }
            if !plan.include_stale && self.hits.iter().any(|h| h.freshness != Freshness::Current) {
                return Err("stale hits listed without include_stale".to_owned());
            }
        }
        for (i, hit) in self.hits.iter().enumerate() {
            if hit.rank as usize != i + 1 {
                return Err("hit ranks must be 1..n".to_owned());
            }
            hit.span.validate()?;
        }
        Ok(())
    }
}

/// A shown match: the receipt hit plus the text of its span read from the
/// live source now, present only while the pinned revision is readable
/// (current or superseded), never for a tombstoned span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrievalHit {
    pub hit: ReceiptHit,
    pub text: Option<String>,
}

/// The answer to a retrieval request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrievalResult {
    pub receipt: RetrievalReceipt,
    pub hits: Vec<RetrievalHit>,
}

/// How two Canvas nodes relate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanvasRelation {
    Supports,
    Contradicts,
    Relates,
    Questions,
}

closed_vocabulary!(CanvasRelation, "canvas relation", {
    Supports => "supports",
    Contradicts => "contradicts",
    Relates => "relates",
    Questions => "questions",
});

/// What a Canvas node holds. Evidence nodes hold a live reference only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
pub enum CanvasNodeContent {
    /// A researcher's note: a claim, question or idea (not evidence).
    Note { text: String },
    /// A live reference to an exact evidence span.
    Evidence { span: EvidenceSpanRef },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvasNode {
    /// Short key, unique within the Canvas (`[a-z0-9_-]`).
    pub key: String,
    pub content: CanvasNodeContent,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvasEdge {
    pub from: String,
    pub to: String,
    pub relation: CanvasRelation,
}

/// One revision of a Research Canvas. Revisions are immutable and
/// contiguous; an edit writes revision n+1 from revision n.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvasRevision {
    /// `header.id` is the Canvas id, shared by every revision.
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub revision: u32,
    pub title: String,
    pub nodes: Vec<CanvasNode>,
    pub edges: Vec<CanvasEdge>,
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.chars().count() <= NODE_KEY_MAX_CHARS
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

impl CanvasRevision {
    pub fn validate(&self) -> Result<(), String> {
        if self.revision == 0 {
            return Err("canvas revisions start at 1".to_owned());
        }
        if self.title.trim().is_empty()
            || self.title.chars().count() > TITLE_MAX_CHARS
            || self.title.chars().any(char::is_control)
        {
            return Err("canvas title must be 1-200 printable characters".to_owned());
        }
        if self.nodes.len() > CANVAS_NODES_MAX || self.edges.len() > CANVAS_EDGES_MAX {
            return Err("canvas exceeds its node or edge bound".to_owned());
        }
        let mut keys = std::collections::HashSet::new();
        for node in &self.nodes {
            if !valid_key(&node.key) {
                return Err(format!("bad node key {:?}", node.key));
            }
            if !keys.insert(node.key.as_str()) {
                return Err(format!("duplicate node key {:?}", node.key));
            }
            match &node.content {
                CanvasNodeContent::Note { text } => {
                    if text.trim().is_empty() || text.chars().count() > NOTE_MAX_CHARS {
                        return Err("note text must be 1-4000 characters".to_owned());
                    }
                }
                CanvasNodeContent::Evidence { span } => span.validate()?,
            }
        }
        let mut edges = std::collections::HashSet::new();
        for edge in &self.edges {
            if !keys.contains(edge.from.as_str()) || !keys.contains(edge.to.as_str()) {
                return Err("an edge names a missing node".to_owned());
            }
            if edge.from == edge.to {
                return Err("an edge cannot join a node to itself".to_owned());
            }
            if !edges.insert(edge) {
                return Err("duplicate edge".to_owned());
            }
        }
        Ok(())
    }
}

/// One Canvas edit. A batch of ops applies to one expected revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "op", deny_unknown_fields)]
pub enum CanvasOp {
    AddNote {
        key: String,
        text: String,
    },
    AddEvidence {
        key: String,
        span: EvidenceSpanRef,
    },
    Link {
        from: String,
        to: String,
        relation: CanvasRelation,
    },
    Unlink {
        from: String,
        to: String,
        relation: CanvasRelation,
    },
    /// Removes the node and every edge touching it.
    RemoveNode {
        key: String,
    },
    Retitle {
        title: String,
    },
}

impl CanvasOp {
    /// Applies the op to a draft revision. The caller validates the result.
    pub fn apply(&self, canvas: &mut CanvasRevision) -> Result<(), String> {
        match self {
            Self::AddNote { key, text } => canvas.nodes.push(CanvasNode {
                key: key.clone(),
                content: CanvasNodeContent::Note { text: text.clone() },
            }),
            Self::AddEvidence { key, span } => canvas.nodes.push(CanvasNode {
                key: key.clone(),
                content: CanvasNodeContent::Evidence { span: span.clone() },
            }),
            Self::Link { from, to, relation } => canvas.edges.push(CanvasEdge {
                from: from.clone(),
                to: to.clone(),
                relation: *relation,
            }),
            Self::Unlink { from, to, relation } => {
                let before = canvas.edges.len();
                canvas
                    .edges
                    .retain(|e| !(e.from == *from && e.to == *to && e.relation == *relation));
                if canvas.edges.len() == before {
                    return Err("no such edge".to_owned());
                }
            }
            Self::RemoveNode { key } => {
                let before = canvas.nodes.len();
                canvas.nodes.retain(|n| n.key != *key);
                if canvas.nodes.len() == before {
                    return Err(format!("no node {key:?}"));
                }
                canvas.edges.retain(|e| e.from != *key && e.to != *key);
            }
            Self::Retitle { title } => canvas.title.clone_from(title),
        }
        Ok(())
    }
}

/// Live resolution of one evidence node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeResolution {
    pub key: String,
    pub freshness: Freshness,
    /// The span's text read from the live source now; present only while
    /// the pinned revision is readable (current or superseded).
    pub text: Option<String>,
}

/// How two evidence nodes cite the same source location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOverlapKind {
    /// The same character range.
    Exact,
    /// One range inside the other.
    Nested,
    /// The ranges share some characters.
    Partial,
}

/// Two evidence nodes whose spans overlap in one source location (same
/// revision and the same cell, segment or excerpt). `first` sorts before
/// `second`. A review signal only: neither node is preferred or removed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceOverlap {
    pub kind: EvidenceOverlapKind,
    pub first: String,
    pub second: String,
}

/// A Canvas revision with every evidence node resolved live, plus the
/// contradiction and missing-evidence inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvasView {
    pub canvas: CanvasRevision,
    pub resolutions: Vec<NodeResolution>,
    /// `contradicts` edges, as stated by the researcher.
    pub contradictions: Vec<CanvasEdge>,
    /// Notes with no `supports` edge from a current evidence node.
    pub unsupported_notes: Vec<String>,
    /// Evidence nodes whose revision has been superseded.
    pub superseded_evidence: Vec<String>,
    /// Evidence nodes whose source is no longer readable.
    pub missing_evidence: Vec<String>,
    /// Evidence nodes that cite the same or overlapping text of one source
    /// location, so one piece of evidence may be counted twice. Sorted by
    /// kind, then keys.
    pub overlapping_evidence: Vec<EvidenceOverlap>,
}

/// Summary row for listings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvasSummary {
    pub canvas_id: OpaqueId,
    pub title: String,
    pub revision: u32,
    pub node_count: u32,
    pub edge_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{AuthorityScopeId, RealmId};

    fn h(id: &str) -> ObjectHeader {
        ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: KNOWLEDGE_SCHEMA_VERSION,
            realm_id: RealmId::new("realm"),
            authority_scope_id: AuthorityScopeId::new("scope"),
        }
    }

    fn source(kind: KnowledgeSourceKind, id: &str) -> IndexSourceRef {
        IndexSourceRef {
            kind,
            object_id: OpaqueId::new(id),
            content_digest: DigestSha256::of(id.as_bytes()),
            lineage_id: OpaqueId::new(id),
        }
    }

    fn cell_span() -> EvidenceSpanRef {
        EvidenceSpanRef {
            source: source(KnowledgeSourceKind::DataSnapshot, "snap-1"),
            locator: SpanLocator::Cell {
                row: 0,
                column: "note".to_owned(),
            },
            char_start: 0,
            char_end: 5,
        }
    }

    fn canvas() -> CanvasRevision {
        CanvasRevision {
            header: h("canvas-1"),
            project_id: OpaqueId::new("p"),
            revision: 1,
            title: "Statins".to_owned(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    fn hit(rank: u32, freshness: Freshness) -> ReceiptHit {
        ReceiptHit {
            rank,
            chunk_seq: rank,
            score: 10,
            span: cell_span(),
            freshness,
        }
    }

    fn plan(include_stale: bool) -> RetrievalPlan {
        RetrievalPlan {
            stages: vec![RetrievalStage::Lexical],
            terms: vec!["ldl".to_owned()],
            max_hits: 5,
            include_stale,
        }
    }

    fn receipt(outcome: RetrievalOutcome, hits: Vec<ReceiptHit>) -> RetrievalReceipt {
        let ran = outcome != RetrievalOutcome::Denied;
        RetrievalReceipt {
            header: h("r"),
            project_id: OpaqueId::new("p"),
            query: "ldl".to_owned(),
            query_digest: DigestSha256::of(b"ldl"),
            plan: ran.then(|| plan(true)),
            manifest_id: ran.then(|| OpaqueId::new("m")),
            manifest_chunks_digest: ran.then(|| DigestSha256::of(b"c")),
            index_status: ran.then(|| IndexStatus {
                manifest_id: OpaqueId::new("m"),
                version: 1,
                chunk_count: 1,
                current_sources: 1,
                superseded_sources: 0,
                tombstoned_sources: 0,
                unindexed_sources: 0,
            }),
            outcome,
            deny_reason: (!ran).then_some(RetrievalDenyReason::NoIndex),
            hits,
        }
    }

    #[test]
    fn vocabularies_round_trip_and_are_closed() {
        for k in KnowledgeSourceKind::ALL {
            assert_eq!(KnowledgeSourceKind::parse(k.as_str()), Ok(*k));
        }
        for f in Freshness::ALL {
            assert_eq!(Freshness::parse(f.as_str()), Ok(*f));
        }
        for r in CanvasRelation::ALL {
            assert_eq!(CanvasRelation::parse(r.as_str()), Ok(*r));
        }
        for o in RetrievalOutcome::ALL {
            assert_eq!(RetrievalOutcome::parse(o.as_str()), Ok(*o));
        }
        for d in RetrievalDenyReason::ALL {
            assert_eq!(RetrievalDenyReason::parse(d.as_str()), Ok(*d));
        }
        assert_eq!(RetrievalStage::ALL, &[RetrievalStage::Lexical]);
        assert!(RetrievalStage::parse("vector").is_err());
        assert!(Freshness::parse("fresh").is_err());
    }

    #[test]
    fn spans_match_their_source_kind() {
        assert!(cell_span().validate().is_ok());
        let mut empty = cell_span();
        empty.char_end = 0;
        assert!(empty.validate().is_err());
        let mut wrong = cell_span();
        wrong.source.kind = KnowledgeSourceKind::TranscriptRevision;
        assert!(wrong.validate().is_err());
        let seg = EvidenceSpanRef {
            source: source(KnowledgeSourceKind::TranscriptRevision, "rev-1"),
            locator: SpanLocator::Segment {
                seq: 1,
                start_ms: 500,
                end_ms: 500,
            },
            char_start: 0,
            char_end: 3,
        };
        assert!(seg.validate().is_err(), "zero-length time span");
        let excerpt = EvidenceSpanRef {
            source: source(KnowledgeSourceKind::AnalyticsResult, "res-1"),
            locator: SpanLocator::Excerpt,
            char_start: 0,
            char_end: 3,
        };
        assert!(excerpt.validate().is_err());
    }

    #[test]
    fn chunks_and_manifests_hold_their_invariants() {
        let chunk = IndexChunk {
            seq: 1,
            span: cell_span(),
            text: "hello".to_owned(),
        };
        assert!(chunk.validate().is_ok());
        let mut short = chunk.clone();
        short.text = "hell".to_owned();
        assert!(short.validate().is_err(), "text length must equal span");
        let digest = chunks_digest(std::slice::from_ref(&chunk));
        assert_eq!(digest, chunks_digest(&[chunk]));
        let mut manifest = IndexManifest {
            header: h("m"),
            project_id: OpaqueId::new("p"),
            version: 1,
            tokenizer: LEXICAL_TOKENIZER.to_owned(),
            sources: vec![
                source(KnowledgeSourceKind::DataSnapshot, "a"),
                source(KnowledgeSourceKind::DataSnapshot, "b"),
            ],
            chunk_count: 1,
            chunks_digest: digest,
        };
        assert!(manifest.validate().is_ok());
        manifest.sources.reverse();
        assert!(manifest.validate().is_err(), "unsorted sources");
        manifest.sources.reverse();
        manifest.tokenizer = "vector".to_owned();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn receipts_state_insufficient_evidence_honestly() {
        assert!(
            receipt(RetrievalOutcome::Results, vec![hit(1, Freshness::Current)])
                .validate()
                .is_ok()
        );
        assert!(
            receipt(RetrievalOutcome::InsufficientEvidence, Vec::new())
                .validate()
                .is_ok()
        );
        // Only stale matches: insufficient, never results.
        assert!(
            receipt(
                RetrievalOutcome::InsufficientEvidence,
                vec![hit(1, Freshness::Superseded)]
            )
            .validate()
            .is_ok()
        );
        assert!(
            receipt(
                RetrievalOutcome::Results,
                vec![hit(1, Freshness::Tombstoned)]
            )
            .validate()
            .is_err()
        );
        assert!(
            receipt(
                RetrievalOutcome::InsufficientEvidence,
                vec![hit(1, Freshness::Current)]
            )
            .validate()
            .is_err()
        );
        assert!(
            receipt(RetrievalOutcome::Denied, Vec::new())
                .validate()
                .is_ok()
        );
        assert!(
            receipt(RetrievalOutcome::Denied, vec![hit(1, Freshness::Current)])
                .validate()
                .is_err()
        );
        let mut r = receipt(RetrievalOutcome::Results, vec![hit(2, Freshness::Current)]);
        assert!(r.validate().is_err(), "ranks start at 1");
        r.hits[0].rank = 1;
        r.query = "other".to_owned();
        assert!(r.validate().is_err(), "digest mismatch");
        let mut hidden = receipt(
            RetrievalOutcome::Results,
            vec![hit(1, Freshness::Current), hit(2, Freshness::Superseded)],
        );
        hidden.plan = Some(plan(false));
        assert!(hidden.validate().is_err(), "stale hits need include_stale");
    }

    #[test]
    fn canvas_revisions_and_ops_validate() {
        let mut c = canvas();
        for op in [
            CanvasOp::AddNote {
                key: "claim".to_owned(),
                text: "LDL fell".to_owned(),
            },
            CanvasOp::AddEvidence {
                key: "e1".to_owned(),
                span: cell_span(),
            },
            CanvasOp::Link {
                from: "e1".to_owned(),
                to: "claim".to_owned(),
                relation: CanvasRelation::Supports,
            },
        ] {
            op.apply(&mut c).unwrap();
        }
        assert!(c.validate().is_ok());
        let mut dup = c.clone();
        CanvasOp::AddNote {
            key: "claim".to_owned(),
            text: "again".to_owned(),
        }
        .apply(&mut dup)
        .unwrap();
        assert!(dup.validate().is_err(), "duplicate key");
        let mut dangling = c.clone();
        CanvasOp::Link {
            from: "e1".to_owned(),
            to: "nope".to_owned(),
            relation: CanvasRelation::Relates,
        }
        .apply(&mut dangling)
        .unwrap();
        assert!(dangling.validate().is_err());
        let mut self_loop = c.clone();
        CanvasOp::Link {
            from: "e1".to_owned(),
            to: "e1".to_owned(),
            relation: CanvasRelation::Relates,
        }
        .apply(&mut self_loop)
        .unwrap();
        assert!(self_loop.validate().is_err());
        let mut removed = c.clone();
        CanvasOp::RemoveNode {
            key: "e1".to_owned(),
        }
        .apply(&mut removed)
        .unwrap();
        assert!(removed.edges.is_empty(), "edges leave with their node");
        assert!(removed.validate().is_ok());
        assert!(
            CanvasOp::RemoveNode {
                key: "e1".to_owned()
            }
            .apply(&mut removed)
            .is_err()
        );
        let mut bad_key = canvas();
        CanvasOp::AddNote {
            key: "Bad Key".to_owned(),
            text: "x".to_owned(),
        }
        .apply(&mut bad_key)
        .unwrap();
        assert!(bad_key.validate().is_err());
        let mut blank = canvas();
        blank.title = " ".to_owned();
        assert!(blank.validate().is_err());
    }

    #[test]
    fn index_status_is_fresh_only_when_nothing_is_stale_or_missing() {
        let mut s = IndexStatus {
            manifest_id: OpaqueId::new("m"),
            version: 1,
            chunk_count: 3,
            current_sources: 2,
            superseded_sources: 0,
            tombstoned_sources: 0,
            unindexed_sources: 0,
        };
        assert!(s.is_fresh());
        s.unindexed_sources = 1;
        assert!(!s.is_fresh());
        s.unindexed_sources = 0;
        s.tombstoned_sources = 1;
        assert!(!s.is_fresh());
    }
}
