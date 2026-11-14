//! Project + Artifact Graph contracts (Spec 074).
//!
//! Organizational authority layer for grouping existing MedScale work into
//! Projects and Experiments. A Project REFERENCES canonical objects owned by
//! their existing systems; it never duplicates their authoritative payload.
//!
//! Reused primitives (authoritative, defined elsewhere):
//! `OpaqueId`, `ObjectHeader`, `DigestSha256`, `RealmId`, `AuthorityScopeId`,
//! existing audit/evidence/provenance contracts.
//!
//! Transport error mapping (semantic -> `crate::envelopes::AuthorityError`):
//!
//! ```text
//! Invalid          -> InvalidArgument { message }
//! NotFound         -> NotFound
//! Denied           -> Unauthorized | SessionDenied | WrongScope (most precise applies)
//! Conflict         -> Conflict { message } (stale expected_revision, duplicate)
//! StaleReference   -> StaleReference { message }
//! Corrupt          -> Corrupt { message }
//! UnsupportedSchema-> UnsupportedSchema { message }
//! Unavailable      -> Unavailable { message }
//! Internal         -> Internal { message }
//! ```
//!
//! `Denied` keeps the existing variants on purpose: no second auth taxonomy.
//! The six struct variants above extend `AuthorityError` additively in 074-A.
//!
//! Durable timestamps: none. 074 follows the existing audit convention where
//! ordering and history travel through the audit trail, not through per-object
//! clocks. `MedicalTime` keeps its clinical semantics and is not reused here.

use serde::{Deserialize, Serialize};

use crate::objects::{AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId};

/// Durable schema version for Project Graph rows.
pub const PROJECT_GRAPH_SCHEMA_VERSION: u32 = 1;

/// Initial revision assigned at create.
pub const PROJECT_REVISION_INITIAL: ProjectRevision = 1;

/// Maximum name length in Unicode scalar values.
pub const PROJECT_NAME_MAX_CHARS: usize = 128;

/// Maximum description length in Unicode scalar values.
pub const PROJECT_DESCRIPTION_MAX_CHARS: usize = 2_048;

/// Default page size for bounded graph queries.
pub const GRAPH_NEIGHBOR_LIMIT_DEFAULT: u32 = 25;

/// Maximum page size for bounded graph queries.
pub const GRAPH_NEIGHBOR_LIMIT_MAX: u32 = 100;

/// Maximum opaque cursor length in bytes.
pub const GRAPH_CURSOR_MAX_BYTES: usize = 256;

/// Maximum `OtherExplicit` kind identifier length in bytes.
pub const ARTIFACT_KIND_OTHER_MAX_BYTES: usize = 128;

/// Maximum opaque owner version-token length in Unicode scalar values.
pub const ARTIFACT_BINDING_REVISION_MAX_CHARS: usize = 128;

/// Maximum refs returned in one `ProjectContext`.
pub const PROJECT_CONTEXT_REFS_MAX: usize = 100;

/// Maximum graph edges returned in one `ProjectContext`.
pub const PROJECT_CONTEXT_GRAPH_MAX: usize = 100;

/// Monotonic optimistic-concurrency token.
///
/// Concurrency state only: not content identity and not a digest.
/// Create starts at [`PROJECT_REVISION_INITIAL`]; every successful mutation
/// increments by exactly one; mutations supply `expected_revision` and a
/// mismatch fails with `Conflict` without writing.
pub type ProjectRevision = u64;

/// Returns the revision a create must assign.
#[must_use]
pub const fn initial_revision() -> ProjectRevision {
    PROJECT_REVISION_INITIAL
}

/// Returns the next revision when `expected` matches `current`, else an error.
///
/// Pure contract helper shared by storage and Core so the increment rule has a
/// single definition: `current + 1` on match, `Conflict` without write on
/// mismatch.
pub fn check_revision(
    current: ProjectRevision,
    expected: ProjectRevision,
) -> Result<ProjectRevision, String> {
    if current == expected {
        Ok(current.saturating_add(1))
    } else {
        Err(format!(
            "stale revision: expected {expected}, current is {current}"
        ))
    }
}

/// Lifecycle of a Project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Active,
    Archived,
}

impl ProjectStatus {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }
}

/// Lifecycle of an Experiment. No move between Projects in 074.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentStatus {
    Draft,
    Active,
    Completed,
    Archived,
}

impl ExperimentStatus {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }
    /// Returns true for the transitions admitted in 074.
    ///
    /// Draft -> Active -> Completed -> Archived, plus early archive from any
    /// non-archived state. Archive is terminal: no experiment restore in 074.
    #[must_use]
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Active)
                | (Self::Draft, Self::Archived)
                | (Self::Active, Self::Completed)
                | (Self::Active, Self::Archived)
                | (Self::Completed, Self::Archived)
        )
    }
}

/// Organizational unit grouping existing work. References, never copies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
}

impl Project {
    /// Creates a revision-1 active Project after validating metadata.
    pub fn new(
        header: ObjectHeader,
        name: String,
        description: Option<String>,
    ) -> Result<Self, String> {
        validate_metadata_fields(&name, description.as_deref())?;
        Ok(Self {
            header,
            revision: initial_revision(),
            name,
            description,
            status: ProjectStatus::Active,
        })
    }

    /// Returns the post-mutation revision or a `Conflict` message.
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }

    /// Returns true when the header scope matches the admitted actor scope.
    #[must_use]
    pub fn scope_matches(&self, realm: &RealmId, scope: &AuthorityScopeId) -> bool {
        self.header.realm_id == *realm && self.header.authority_scope_id == *scope
    }
}

/// Experiment scoped to exactly one Project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Experiment {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub revision: ProjectRevision,
    pub name: String,
    pub description: Option<String>,
    pub status: ExperimentStatus,
}

impl Experiment {
    /// Creates a revision-1 draft Experiment after validating metadata.
    pub fn new(
        header: ObjectHeader,
        project_id: OpaqueId,
        name: String,
        description: Option<String>,
    ) -> Result<Self, String> {
        validate_metadata_fields(&name, description.as_deref())?;
        Ok(Self {
            header,
            project_id,
            revision: initial_revision(),
            name,
            description,
            status: ExperimentStatus::Draft,
        })
    }

    /// Returns the post-mutation revision or a `Conflict` message.
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }

    /// Returns true when the header scope matches the admitted actor scope.
    #[must_use]
    pub fn scope_matches(&self, realm: &RealmId, scope: &AuthorityScopeId) -> bool {
        self.header.realm_id == *realm && self.header.authority_scope_id == *scope
    }
}

/// Owning domain of a referenced canonical object.
///
/// Maps to existing durable families; never flattens them. Audit records,
/// projections, and merge decisions stay in their owning trails and are not
/// attachable: reference the underlying assertion or source instead.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    SourceRecord,
    DerivedSourceArtifact,
    Proposal,
    ClinicalAssertion,
    EvaluationRecord,
    IdentityAssertion,
    AmendmentRecord,
    EvidenceDocument,
    PackManifest,
    /// Versioned explicit identifier for a canonical family this vocabulary
    /// predates. Syntactically validated; Core reports `UnsupportedKind` when
    /// it cannot map the identifier to a resolvable object.
    OtherExplicit(String),
}

impl ArtifactKind {
    /// Constructs a bounded `OtherExplicit` identifier or rejects it.
    ///
    /// Admitted charset: lowercase ASCII alphanumeric plus `.`, `_`, `-`.
    /// Rejects empty values, overlong values, and identifiers colliding with
    /// the known snake_case vocabulary so display strings can never shadow
    /// authority.
    pub fn other_explicit(id: &str) -> Result<Self, String> {
        if id.is_empty() || id.len() > ARTIFACT_KIND_OTHER_MAX_BYTES {
            return Err(format!(
                "other kind identifier must be 1..={} bytes",
                ARTIFACT_KIND_OTHER_MAX_BYTES
            ));
        }
        let syntax_ok = id.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'_' || b == b'-'
        });
        if !syntax_ok {
            return Err("other kind identifier uses forbidden characters".to_owned());
        }
        const KNOWN: [&str; 9] = [
            "source_record",
            "derived_source_artifact",
            "proposal",
            "clinical_assertion",
            "evaluation_record",
            "identity_assertion",
            "amendment_record",
            "evidence_document",
            "pack_manifest",
        ];
        if KNOWN.contains(&id) {
            return Err("other kind identifier must not shadow a known kind".to_owned());
        }
        Ok(Self::OtherExplicit(id.to_owned()))
    }

    /// Returns the canonical snake_case name of a known kind.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::SourceRecord => "source_record",
            Self::DerivedSourceArtifact => "derived_source_artifact",
            Self::Proposal => "proposal",
            Self::ClinicalAssertion => "clinical_assertion",
            Self::EvaluationRecord => "evaluation_record",
            Self::IdentityAssertion => "identity_assertion",
            Self::AmendmentRecord => "amendment_record",
            Self::EvidenceDocument => "evidence_document",
            Self::PackManifest => "pack_manifest",
            Self::OtherExplicit(_) => "other_explicit",
        }
    }
}

/// Strongest version binding the owning contract actually supports.
///
/// Identity is mandatory. The binding never claims immutability the owner does
/// not provide, and a later change to the underlying object never silently
/// rewrites a pinned binding: resolution reports it explicitly.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactVersionBinding {
    /// Allowed only when the owner genuinely lacks a stronger stable version.
    IdentityOnly,
    Digest(DigestSha256),
    /// Opaque owner-native version token (never invented by the Project layer).
    Revision(String),
    DigestAndRevision {
        digest: DigestSha256,
        revision: String,
    },
}

impl ArtifactVersionBinding {
    /// Validates binding bounds without contacting storage.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::IdentityOnly | Self::Digest(_) => Ok(()),
            Self::Revision(rev) => validate_binding_revision(rev),
            Self::DigestAndRevision { revision, .. } => validate_binding_revision(revision),
        }
    }
}

/// Identity plus kind plus version binding of one canonical object.
///
/// Durable attachment authority; holds no title/body/patient/model payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactDescriptor {
    pub object_id: OpaqueId,
    pub kind: ArtifactKind,
    pub binding: ArtifactVersionBinding,
}

impl ArtifactDescriptor {
    /// Validates descriptor shape (binding rules; resolution happens in Core).
    pub fn validate(&self) -> Result<(), String> {
        if self.object_id.as_str().trim().is_empty() {
            return Err("artifact object_id must be non-empty".to_owned());
        }
        if let ArtifactKind::OtherExplicit(id) = &self.kind {
            ArtifactKind::other_explicit(id)?;
        }
        self.binding.validate()
    }
}

/// Attachment of one artifact to a Project (optionally one Experiment).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectArtifactRef {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub experiment_id: Option<OpaqueId>,
    pub artifact: ArtifactDescriptor,
    pub revision: ProjectRevision,
    pub status: RefStatus,
}

/// Lifecycle of an attachment: explicit tombstone, never cascade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefStatus {
    Active,
    Detached,
}

impl ProjectArtifactRef {
    /// Creates a revision-1 active attachment after shape validation.
    pub fn new(
        header: ObjectHeader,
        project_id: OpaqueId,
        experiment_id: Option<OpaqueId>,
        artifact: ArtifactDescriptor,
    ) -> Result<Self, String> {
        artifact.validate()?;
        Ok(Self {
            header,
            project_id,
            experiment_id,
            artifact,
            revision: initial_revision(),
            status: RefStatus::Active,
        })
    }

    /// Returns the post-mutation revision or a `Conflict` message.
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }

    /// Deterministic duplicate identity: same project, experiment slot,
    /// target identity, kind, and binding. Core treats a repeat as idempotent
    /// or an explicit conflict; never a silent duplicate.
    #[must_use]
    pub fn same_attachment_as(&self, other: &Self) -> bool {
        self.project_id == other.project_id
            && self.experiment_id == other.experiment_id
            && self.artifact == other.artifact
    }
}

/// Bounded organizational/workflow predicate vocabulary.
///
/// `Contains` is organizational containment, not storage ownership.
/// `ExperimentInput`/`ExperimentOutput` record a workflow role only: no
/// causality, provenance, diagnosis, or truth claim. Unknown predicates fail
/// closed; clinical-truth predicates are never admitted here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectGraphPredicate {
    Contains,
    References,
    AssociatedWith,
    ExperimentInput,
    ExperimentOutput,
}

impl ProjectGraphPredicate {
    /// Parses the exact snake_case vocabulary; unknown input fails closed.
    #[must_use]
    pub const fn parse(name: &str) -> Option<Self> {
        match name.as_bytes() {
            b"contains" => Some(Self::Contains),
            b"references" => Some(Self::References),
            b"associated_with" => Some(Self::AssociatedWith),
            b"experiment_input" => Some(Self::ExperimentInput),
            b"experiment_output" => Some(Self::ExperimentOutput),
            _ => None,
        }
    }

    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::References => "references",
            Self::AssociatedWith => "associated_with",
            Self::ExperimentInput => "experiment_input",
            Self::ExperimentOutput => "experiment_output",
        }
    }
}

/// One typed endpoint of a graph edge: artifact or in-project experiment.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphEndpoint {
    Artifact(ArtifactDescriptor),
    Experiment(OpaqueId),
}

impl GraphEndpoint {
    /// Validates endpoint shape (descriptor rules; admission happens in Core).
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Artifact(descriptor) => descriptor.validate(),
            Self::Experiment(id) => {
                if id.as_str().trim().is_empty() {
                    return Err("experiment endpoint id must be non-empty".to_owned());
                }
                Ok(())
            }
        }
    }
}

/// Explicit typed organizational relationship inside one Project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectGraphEdge {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub subject: GraphEndpoint,
    pub predicate: ProjectGraphPredicate,
    pub object: GraphEndpoint,
    pub revision: ProjectRevision,
    pub status: EdgeStatus,
}

/// Lifecycle of an edge: explicit removal, never endpoint deletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeStatus {
    Active,
    Removed,
}

impl ProjectGraphEdge {
    /// Creates a revision-1 active edge after shape validation.
    ///
    /// Self-edges are denied in 074: no predicate admits them.
    pub fn new(
        header: ObjectHeader,
        project_id: OpaqueId,
        subject: GraphEndpoint,
        predicate: ProjectGraphPredicate,
        object: GraphEndpoint,
    ) -> Result<Self, String> {
        subject.validate()?;
        object.validate()?;
        if subject == object {
            return Err("graph self-edges are not admitted".to_owned());
        }
        Ok(Self {
            header,
            project_id,
            subject,
            predicate,
            object,
            revision: initial_revision(),
            status: EdgeStatus::Active,
        })
    }

    /// Returns the post-mutation revision or a `Conflict` message.
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }

    /// Deterministic duplicate identity. Core treats a repeat as idempotent
    /// or an explicit conflict; never a silent duplicate.
    #[must_use]
    pub fn same_edge_as(&self, other: &Self) -> bool {
        self.project_id == other.project_id
            && self.subject == other.subject
            && self.predicate == other.predicate
            && self.object == other.object
    }
}

/// Typed resolution of one attachment against its canonical target.
///
/// `Denied` carries no target metadata. No fuzzy rebind: a changed target is
/// `Stale`, a gone target is `Missing`, never a silent replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceResolution {
    Current,
    Stale,
    Missing,
    UnsupportedKind,
    Denied,
    Corrupt,
}

/// One attachment plus its typed resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedArtifactRef {
    pub reference: ProjectArtifactRef,
    pub resolution: ReferenceResolution,
}

/// Non-sensitive Project metadata plus post-authorization counts.
///
/// Counts are computed after authorization filtering and must never prove the
/// existence of inaccessible protected artifacts where policy treats
/// existence itself as sensitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectSummary {
    pub project_id: OpaqueId,
    pub name: String,
    pub status: ProjectStatus,
    pub revision: ProjectRevision,
    pub experiment_count: u64,
    pub active_ref_count: u64,
    pub active_edge_count: u64,
}

/// Non-sensitive Experiment metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentSummary {
    pub experiment_id: OpaqueId,
    pub project_id: OpaqueId,
    pub name: String,
    pub status: ExperimentStatus,
    pub revision: ProjectRevision,
}

/// Traversal direction for a bounded neighbor query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphDirection {
    Outgoing,
    Incoming,
    Both,
}

/// Bounded neighbor query. Single hop, paginated, deterministic edge-id order.
/// No recursive traversal in 074.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphNeighborQuery {
    pub project_id: OpaqueId,
    pub start: GraphEndpoint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicates: Option<Vec<ProjectGraphPredicate>>,
    #[serde(default)]
    pub direction: Option<GraphDirection>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub cursor: Option<String>,
}

impl GraphNeighborQuery {
    /// Effective page size: default 25, clamped to 1..=100.
    #[must_use]
    pub fn effective_limit(&self) -> u32 {
        match self.limit {
            None | Some(0) => GRAPH_NEIGHBOR_LIMIT_DEFAULT,
            Some(n) => n.min(GRAPH_NEIGHBOR_LIMIT_MAX),
        }
    }

    /// Validates query bounds without contacting storage.
    ///
    /// `limit` of `None` or `0` selects the default page size; values above
    /// the maximum are rejected (callers wanting saturation use
    /// `effective_limit` only after successful validation of a bounded value).
    pub fn validate(&self) -> Result<(), String> {
        self.start.validate()?;
        if self
            .limit
            .is_some_and(|limit| limit > GRAPH_NEIGHBOR_LIMIT_MAX)
        {
            return Err(format!(
                "neighbor limit must be at most {}",
                GRAPH_NEIGHBOR_LIMIT_MAX
            ));
        }
        if self
            .cursor
            .as_ref()
            .is_some_and(|cursor| cursor.len() > GRAPH_CURSOR_MAX_BYTES)
        {
            return Err(format!(
                "neighbor cursor must be at most {} bytes",
                GRAPH_CURSOR_MAX_BYTES
            ));
        }
        Ok(())
    }
}

/// One deterministic page of neighbor edges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphNeighborPage {
    pub edges: Vec<ProjectGraphEdge>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Bounded inspectable manifest for one Project (optionally one Experiment).
///
/// Authorization-filtered only: every returned ref passed scope checks, no raw
/// artifact payload is included, and this is never an ambient vault handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectContext {
    pub project: ProjectSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experiment: Option<ExperimentSummary>,
    pub authorized_artifact_refs: Vec<ResolvedArtifactRef>,
    pub graph_slice: Vec<ProjectGraphEdge>,
    pub refs_total: u64,
    pub graph_total: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refs_next_cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph_next_cursor: Option<String>,
}

impl ProjectContext {
    /// Validates context page bounds without contacting storage.
    pub fn validate(&self) -> Result<(), String> {
        if self.authorized_artifact_refs.len() > PROJECT_CONTEXT_REFS_MAX {
            return Err(format!(
                "context refs exceed maximum of {}",
                PROJECT_CONTEXT_REFS_MAX
            ));
        }
        if self.graph_slice.len() > PROJECT_CONTEXT_GRAPH_MAX {
            return Err(format!(
                "context graph slice exceeds maximum of {}",
                PROJECT_CONTEXT_GRAPH_MAX
            ));
        }
        Ok(())
    }
}

/// Validates Project/Experiment metadata bounds without contacting storage.
///
/// Shared by constructors, Core update paths, and backup-restore replay so the
/// same bounds hold on every write path.
pub fn validate_metadata_fields(name: &str, description: Option<&str>) -> Result<(), String> {
    validate_name(name)?;
    validate_description(description)
}

fn validate_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("name must be non-empty".to_owned());
    }
    if name.chars().count() > PROJECT_NAME_MAX_CHARS {
        return Err(format!(
            "name must be at most {} characters",
            PROJECT_NAME_MAX_CHARS
        ));
    }
    Ok(())
}

fn validate_description(description: Option<&str>) -> Result<(), String> {
    if description.is_some_and(|text| text.chars().count() > PROJECT_DESCRIPTION_MAX_CHARS) {
        return Err(format!(
            "description must be at most {} characters",
            PROJECT_DESCRIPTION_MAX_CHARS
        ));
    }
    Ok(())
}

fn validate_binding_revision(revision: &str) -> Result<(), String> {
    if revision.trim().is_empty() {
        return Err("binding revision must be non-empty".to_owned());
    }
    if revision.chars().count() > ARTIFACT_BINDING_REVISION_MAX_CHARS {
        return Err(format!(
            "binding revision must be at most {} characters",
            ARTIFACT_BINDING_REVISION_MAX_CHARS
        ));
    }
    if revision.chars().any(char::is_control) {
        return Err("binding revision must not contain control characters".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(id: &str) -> ObjectHeader {
        ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: PROJECT_GRAPH_SCHEMA_VERSION,
            realm_id: RealmId::new("realm-1"),
            authority_scope_id: AuthorityScopeId::new("scope-1"),
        }
    }

    fn descriptor() -> ArtifactDescriptor {
        ArtifactDescriptor {
            object_id: OpaqueId::new("src-1"),
            kind: ArtifactKind::SourceRecord,
            binding: ArtifactVersionBinding::IdentityOnly,
        }
    }

    #[test]
    fn revision_starts_at_one_and_increments_by_one() {
        assert_eq!(initial_revision(), 1);
        let project = Project::new(header("proj-1"), "Study".to_owned(), None).unwrap();
        assert_eq!(project.revision, 1);
        assert_eq!(project.check_mutation(1).unwrap(), 2);
    }

    #[test]
    fn stale_revision_conflicts_without_write() {
        let project = Project::new(header("proj-1"), "Study".to_owned(), None).unwrap();
        let err = project.check_mutation(7).unwrap_err();
        assert!(err.contains("stale revision"));
        assert_eq!(project.revision, 1);
    }

    #[test]
    fn project_name_bounds_are_enforced() {
        assert!(Project::new(header("p"), "  ".to_owned(), None).is_err());
        assert!(Project::new(header("p"), String::new(), None).is_err());
        let long = "n".repeat(PROJECT_NAME_MAX_CHARS + 1);
        assert!(Project::new(header("p"), long, None).is_err());
        let max = "n".repeat(PROJECT_NAME_MAX_CHARS);
        assert!(Project::new(header("p"), max, None).is_ok());
    }

    #[test]
    fn project_description_bounds_are_enforced() {
        let long = "d".repeat(PROJECT_DESCRIPTION_MAX_CHARS + 1);
        assert!(Project::new(header("p"), "ok".to_owned(), Some(long)).is_err());
    }

    #[test]
    fn experiment_lifecycle_admits_only_074_transitions() {
        use ExperimentStatus::{Active, Archived, Completed, Draft};
        assert!(Draft.can_transition_to(Active));
        assert!(Draft.can_transition_to(Archived));
        assert!(Active.can_transition_to(Completed));
        assert!(Completed.can_transition_to(Archived));
        assert!(!Draft.can_transition_to(Completed));
        assert!(!Completed.can_transition_to(Active));
        assert!(!Archived.can_transition_to(Active));
        assert!(!Active.can_transition_to(Draft));
    }

    #[test]
    fn predicate_vocabulary_is_closed() {
        assert_eq!(
            ProjectGraphPredicate::parse("contains"),
            Some(ProjectGraphPredicate::Contains)
        );
        assert_eq!(
            ProjectGraphPredicate::parse("associated_with"),
            Some(ProjectGraphPredicate::AssociatedWith)
        );
        assert_eq!(ProjectGraphPredicate::parse("proves"), None);
        assert_eq!(ProjectGraphPredicate::parse("diagnoses"), None);
        assert_eq!(ProjectGraphPredicate::parse("derived_from"), None);
        assert_eq!(ProjectGraphPredicate::parse(""), None);
        assert_eq!(ProjectGraphPredicate::parse("Contains"), None);
    }

    #[test]
    fn other_explicit_kind_cannot_shadow_known_kinds() {
        assert!(ArtifactKind::other_explicit("custom.v1").is_ok());
        assert!(ArtifactKind::other_explicit("").is_err());
        assert!(ArtifactKind::other_explicit("source_record").is_err());
        assert!(ArtifactKind::other_explicit("has space").is_err());
        assert!(ArtifactKind::other_explicit("UPPER").is_err());
    }

    #[test]
    fn binding_revision_bounds_are_enforced() {
        let bad = ArtifactVersionBinding::Revision("  ".to_owned());
        assert!(bad.validate().is_err());
        let long = ArtifactVersionBinding::Revision("r".repeat(129));
        assert!(long.validate().is_err());
        let ok = ArtifactVersionBinding::Revision("v3".to_owned());
        assert!(ok.validate().is_ok());
        assert!(ArtifactVersionBinding::IdentityOnly.validate().is_ok());
    }

    #[test]
    fn self_edges_are_denied() {
        let endpoint = GraphEndpoint::Experiment(OpaqueId::new("exp-1"));
        let err = ProjectGraphEdge::new(
            header("edge-1"),
            OpaqueId::new("proj-1"),
            endpoint.clone(),
            ProjectGraphPredicate::References,
            endpoint,
        )
        .unwrap_err();
        assert!(err.contains("self-edge"));
    }

    #[test]
    fn duplicate_identity_is_deterministic() {
        let mk_ref = || {
            ProjectArtifactRef::new(header("ref-1"), OpaqueId::new("proj-1"), None, descriptor())
                .unwrap()
        };
        let (a, b) = (mk_ref(), mk_ref());
        assert!(a.same_attachment_as(&b));
        let mut other_desc = descriptor();
        other_desc.object_id = OpaqueId::new("src-2");
        let other =
            ProjectArtifactRef::new(header("ref-2"), OpaqueId::new("proj-1"), None, other_desc)
                .unwrap();
        assert!(!a.same_attachment_as(&other));
    }

    #[test]
    fn neighbor_query_bounds_are_enforced() {
        let base = GraphNeighborQuery {
            project_id: OpaqueId::new("proj-1"),
            start: GraphEndpoint::Experiment(OpaqueId::new("exp-1")),
            predicates: None,
            direction: None,
            limit: None,
            cursor: None,
        };
        assert!(base.validate().is_ok());
        assert_eq!(base.effective_limit(), GRAPH_NEIGHBOR_LIMIT_DEFAULT);
        let over = GraphNeighborQuery {
            limit: Some(GRAPH_NEIGHBOR_LIMIT_MAX + 1),
            ..base.clone()
        };
        assert!(over.validate().is_err());
        let zero = GraphNeighborQuery {
            limit: Some(0),
            ..base.clone()
        };
        assert!(zero.validate().is_ok());
        assert_eq!(zero.effective_limit(), GRAPH_NEIGHBOR_LIMIT_DEFAULT);
        let huge = GraphNeighborQuery {
            limit: Some(10_000),
            ..base.clone()
        };
        assert_eq!(huge.effective_limit(), GRAPH_NEIGHBOR_LIMIT_MAX);
        let bad_cursor = GraphNeighborQuery {
            cursor: Some("c".repeat(GRAPH_CURSOR_MAX_BYTES + 1)),
            ..base
        };
        assert!(bad_cursor.validate().is_err());
    }

    #[test]
    fn context_page_bounds_are_enforced() {
        let summary = ProjectSummary {
            project_id: OpaqueId::new("proj-1"),
            name: "Study".to_owned(),
            status: ProjectStatus::Active,
            revision: 1,
            experiment_count: 0,
            active_ref_count: 0,
            active_edge_count: 0,
        };
        let mk_ctx = |refs: usize, edges: usize| ProjectContext {
            project: summary.clone(),
            experiment: None,
            authorized_artifact_refs: (0..refs)
                .map(|i| ResolvedArtifactRef {
                    reference: ProjectArtifactRef::new(
                        header(&format!("ref-{i}")),
                        OpaqueId::new("proj-1"),
                        None,
                        descriptor(),
                    )
                    .unwrap(),
                    resolution: ReferenceResolution::Current,
                })
                .collect(),
            graph_slice: Vec::new(),
            refs_total: refs as u64,
            graph_total: edges as u64,
            refs_next_cursor: None,
            graph_next_cursor: None,
        };
        assert!(mk_ctx(1, 0).validate().is_ok());
        assert!(mk_ctx(PROJECT_CONTEXT_REFS_MAX + 1, 0).validate().is_err());
    }

    #[test]
    fn authority_structs_reject_unknown_fields() {
        let json = serde_json::json!({
            "header": {
                "id": "proj-1",
                "schema_version": 1,
                "realm_id": "realm-1",
                "authority_scope_id": "scope-1"
            },
            "revision": 1,
            "name": "Study",
            "description": null,
            "status": "active",
            "injected": true
        });
        assert!(serde_json::from_value::<Project>(json).is_err());
    }

    #[test]
    fn predicate_round_trips_through_snake_case() {
        let edge = ProjectGraphEdge::new(
            header("edge-1"),
            OpaqueId::new("proj-1"),
            GraphEndpoint::Experiment(OpaqueId::new("exp-1")),
            ProjectGraphPredicate::ExperimentInput,
            GraphEndpoint::Artifact(descriptor()),
        )
        .unwrap();
        let json = serde_json::to_value(&edge).unwrap();
        assert_eq!(json["predicate"], "experiment_input");
        let back: ProjectGraphEdge = serde_json::from_value(json).unwrap();
        assert_eq!(back, edge);
    }

    #[test]
    fn scope_match_is_exact() {
        let project = Project::new(header("proj-1"), "Study".to_owned(), None).unwrap();
        assert!(project.scope_matches(&RealmId::new("realm-1"), &AuthorityScopeId::new("scope-1")));
        assert!(!project.scope_matches(&RealmId::new("other"), &AuthorityScopeId::new("scope-1")));
    }
}
