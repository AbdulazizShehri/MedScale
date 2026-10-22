//! Versioned authority facade request/response envelopes.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::AUTHORITY_SCHEMA_VERSION;
use crate::actions::{CreateExternalActionIntentRequest, NphiesInvokeRequest, OutboxEntry};
use crate::analytics::{
    CohortCriterion, CohortDefinition, DerivedTable, QueryReceipt, QueryRequest, QueryView,
    ReplayReport, ResultTableDoc, StatisticKind, StatisticResult,
};
use crate::audio::{
    AudioEvidenceRef, AudioRouteRequest, AudioRouteStatus, AudioSession, AudioSource,
    CaptureBackendKind, CaptureState, PcmFormat, TranscriptReceipt, TranscriptRevision,
    TranscriptView,
};
use crate::browse::{
    BrowseAllowlistEntry, BrowseRequest, BrowseRouteStatus, BrowseSession, BrowseSessionView,
};
use crate::collaboration::{
    ActivityRecord, AnchorTarget, ApprovalDecision, ApprovalDecisionOutcome, ApprovalKind,
    ApprovalRequest, MembershipRole, Message, MessageEdit, NoteDocument, NoteRevision,
    ParticipantIdentity, ParticipantKind, Room, RoomMembership, RoomStatus, Task, TaskStatus,
    ThreadRef, ThreadStatus,
};
use crate::compute::{ComputeJob, ComputeJobRequest, ComputeJobView, ComputeStatus};
use crate::data_sources::{
    DataSourceManifest, DataSourceSummary, DataViewKind, DatasetReleaseSummary, FilterExpr,
    RefreshReceipt, ReleaseManifest, SavedDataView, SavedViewSummary, SnapshotRowPage,
    SnapshotSummary, SortKey, SourceLocator, SourceSchema, TransformOp, TransformationReceipt,
    ViewState,
};
use crate::documents::{
    AsrStubRequest, DocumentIntakeRequest, DocumentIntakeResult, MediaStubResult, OcrStubRequest,
};
use crate::evidence::{LexicalRetrieveRequest, LexicalRetrieveResult};
use crate::extensions::{ExtensionCapability, ExtensionLifecycleReceipt, ExtensionPublisher};
use crate::external_artifact::{
    MescArtifactAdmitRequest, MescArtifactVerifyRequest, MescVerifyReport,
};
use crate::hub::{
    DeviceIdentity, HubChallenge, HubEvent, HubEventPage, HubHandshake, HubIdentity, HubInvitation,
    HubInvitationCode, HubLink, HubOutboxEntry, HubSession, HubStatus, SyncEnvelope, SyncIntent,
    SyncOutcome,
};
use crate::ingest::{BackupManifest, IngestReceipt};
use crate::knowledge::{
    CanvasOp, CanvasRevision, CanvasSummary, CanvasView, IndexManifest, IndexStatus,
    RetrievalReceipt, RetrievalRequest, RetrievalResult,
};
use crate::medagent::{
    AgentCapabilityManifest, AgentIdentity, AgentProposal, AgentRun, AgentRunState, AgentTurn,
    ContextManifest, RunReceipt, ToolInvocation, ToolKind, ToolReceipt,
};
use crate::model_fleet::{
    AgentLane, AgentLaneStatus, ComparisonReport, FleetRun, FleetRunState, LaneRunRef,
};
use crate::network::{EgressAllowlistEntry, NetworkBrokerRequest, NetworkBrokerResult};
use crate::objects::{AmendmentKind, DigestSha256, EffectState, MedicalTime, OpaqueId, VaultId};
use crate::online_packs::OnlinePackAcquireRequest;
use crate::packs::{
    PackAdmitResult, PackEvaluationRequest, PackEvaluationResult, PackGenerationRequest,
    PackManifestV0, PackPromotionState,
};
use crate::presentation::{DrillDownResult, SubjectBriefV1, SubjectCoverageV1, SubjectTimelineV1};
use crate::privacy_gate::{
    ArtifactClassification, DataClass, DeidReceipt, EffectiveClassification, EgressBoundary,
    EgressDecision, PrivacyPolicyProfile, ProfileRule, PseudonymMapRef, ReidentificationAudit,
};
use crate::project_graph::{
    ArtifactDescriptor, Experiment, ExperimentSummary, GraphDirection, GraphEndpoint,
    GraphNeighborPage, Project, ProjectArtifactRef, ProjectContext, ProjectGraphEdge,
    ProjectGraphPredicate, ProjectStatus, ProjectSummary, ReferenceResolution, ResolvedArtifactRef,
};
use crate::r_workspace::{
    RLaunchReceipt, RLaunchRequest, RPublishRequest, RPublishView, RRunReceipt, RRunRequest,
    RStageRequest, RWorkspace, RWorkspaceHistory, RWorkspaceStatus, WorkspaceInspection,
};
use crate::workflow::DisclosureRecord;

/// Capability required to execute a facade operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    AcquireLease,
    ReleaseLease,
    Ping,
    CreateSourceRecord,
    CreateDerivedArtifact,
    CreateProposal,
    PromoteProposal,
    CreateIdentityAssertion,
    DecideIdentityMerge,
    AppendAudit,
    TransitionEffect,
    ReadObject,
    OpenSyntheticVault,
    CloseVault,
    IngestFhirSynthetic,
    AttachValidatorEvidence,
    RebuildProjection,
    ReadCanonicalVisibility,
    VerifyBlob,
    BackupVault,
    RestoreVault,
    RunBlobGc,
    GetTimeline,
    GetBrief,
    GetCoverage,
    DrillDownPresentation,
    CreateEncryptedVault,
    OpenEncryptedVault,
    CloseEncryptedVault,
    NetworkBrokerInvoke,
    SetEgressAllowlist,
    PacksInstallLocal,
    PacksList,
    PacksPromote,
    PacksEvaluateLocal,
    DocumentIntake,
    OcrStub,
    AsrStub,
    RetrieveLexical,
    CreateExternalActionIntent,
    ListOutbox,
    NphiesInvoke,
    OnlinePackAcquire,
    MescArtifactAdmit,
    MescArtifactVerify,
    OpenSession,
    RevokeSession,
    AmendAssertion,
    GetFhirSupportMatrix,
    ExportFhirLossAware,
    RejectProposal,
    AppendDisclosure,
    ListDisclosures,
    ProjectCreate,
    ProjectRead,
    ProjectUpdate,
    ProjectArchive,
    ExperimentCreate,
    ExperimentRead,
    ExperimentUpdate,
    ExperimentArchive,
    ProjectArtifactAttach,
    ProjectArtifactDetach,
    ProjectGraphRead,
    ProjectGraphMutate,
    DataSourceCreate,
    DataSourceRead,
    DataSourceUpdate,
    DataSourceArchive,
    SnapshotImport,
    SnapshotPreview,
    SnapshotRead,
    SnapshotRefresh,
    SavedViewCreate,
    SavedViewRead,
    SavedViewUpdate,
    TransformExecute,
    DatasetReleaseCreate,
    DatasetReleaseRead,
    // Spec 076: Collaboration Substrate (T076-03 slice: participant, room,
    // membership only; thread/message/task/note/approval/activity
    // capabilities land in later slices).
    ParticipantRegister,
    ParticipantRead,
    ParticipantRevoke,
    RoomCreate,
    RoomRead,
    RoomUpdate,
    RoomArchive,
    RoomMembershipManage,
    RoomMembershipRead,
    // Spec 076 T076-04/05/06 slice: thread, message, task.
    ThreadCreate,
    ThreadRead,
    ThreadResolve,
    MessagePost,
    MessageRead,
    MessageEdit,
    TaskCreate,
    TaskRead,
    TaskUpdate,
    // Spec 076 T076-07/08/09 slice: note, approval, activity.
    NoteCreate,
    NoteRead,
    NoteUpdate,
    ApprovalRequestCreate,
    ApprovalRequestRead,
    ApprovalDecide,
    ApprovalWithdraw,
    ActivityRead,
    // Spec 077: MedAgent Workbench (T077-03 slice: AgentIdentity +
    // AgentCapabilityManifest only; ContextManifest/AgentRun/tool/receipt/
    // proposal capabilities land in later slices).
    AgentIdentityRegister,
    AgentIdentityRead,
    AgentIdentityRevoke,
    // Spec 077 T077-04 slice.
    ContextManifestCreate,
    ContextManifestRead,
    // Spec 077 T077-05 slice.
    AgentRunCreate,
    AgentRunRead,
    AgentRunStart,
    AgentRunCancel,
    // Spec 077 T077-06 slice.
    AgentToolInvoke,
    // Spec 077 T077-07 slice.
    AgentRunExecute,
    // Spec 077 T077-08 slice.
    AgentRunComplete,
    AgentRunFail,
    // Spec 078: Model Fleet + Compare (T078-03 slice: AgentLane).
    AgentLaneCreate,
    AgentLaneRead,
    AgentLaneRetire,
    // Spec 078 T078-04 slice: FleetRun lifecycle.
    FleetRunCreate,
    FleetRunRead,
    FleetRunDispatch,
    FleetRunExecuteLane,
    FleetRunCancel,
    // Spec 078 T078-05/06 slice: comparison + history.
    ComparisonCompute,
    ComparisonRead,
    // Spec 079: Privacy Gate. `PrivacyReidentify` is deliberately absent from
    // `operator_grants`: a session must request it explicitly.
    PrivacyClassify,
    PrivacyRead,
    PrivacyProfileCreate,
    PrivacyProfileRevoke,
    PrivacyTransform,
    PrivacyReceiptRevoke,
    PrivacyMapCreate,
    PrivacyMapRevoke,
    PrivacyReidentify,
    PrivacyEgressEvaluate,
    // Spec 080: Governed Browse.
    BrowseAllowlistManage,
    BrowseRead,
    BrowseRun,
    BrowseCancel,
    // Spec 081: AudioFlow Foundation.
    AudioImport,
    AudioRead,
    AudioCapture,
    AudioTranscribe,
    AudioCorrect,
    // Spec 082: Analytics Gate.
    AnalyticsQuery,
    AnalyticsRead,
    AnalyticsCohort,
    // Spec 083: Knowledge + Research Canvas.
    KnowledgeIndex,
    KnowledgeSearch,
    KnowledgeRead,
    KnowledgeCanvas,
    // Spec 084: MedScale Hub foundation. `HubBootstrap` ops (enroll,
    // challenge, handshake) run before a session exists; `HubSync` is only
    // ever granted to a device session opened by a handshake.
    HubAdmin,
    HubRead,
    HubBootstrap,
    HubSync,
    HubClient,
    // Spec 085: MedScale Compute (local bounded worker).
    ComputeSubmit,
    ComputeRun,
    ComputeRead,
    // Spec 086: R Workspace (staging, external launch, explicit publish).
    RWorkspaceStage,
    RWorkspaceRun,
    RWorkspacePublish,
    RWorkspaceRead,
    // Spec 087: Community Extensions (declarative only).
    ExtensionAdmin,
    ExtensionInvoke,
    ExtensionRead,
    // Spec 088: AudioFlow Advanced huddles.
    HuddleAct,
    HuddleRead,
    // Spec 089: Research Packs (declarative domain semantics).
    PackAdmin,
    PackRead,
    // Spec 090: institutional adapters (explicit, optional external writes).
    AdapterAdmin,
    AdapterRead,
    // Spec 091: federation (signed bundles between trusted institutions).
    FederationAdmin,
    FederationRead,
    // Spec 103: local model catalog (read-only metadata; no network).
    ModelCatalogRead,
}

impl Capability {
    /// Lease/session bootstrap ops that must work before a client session exists.
    #[must_use]
    pub const fn is_session_bootstrap(self) -> bool {
        matches!(
            self,
            Self::AcquireLease
                | Self::ReleaseLease
                | Self::OpenSession
                | Self::RevokeSession
                | Self::HubBootstrap
        )
    }

    /// Read-only / health ops that do not require a client session under Strict mode.
    #[must_use]
    pub const fn is_read_or_health(self) -> bool {
        matches!(
            self,
            Self::Ping
                | Self::ReadObject
                | Self::GetTimeline
                | Self::GetBrief
                | Self::GetCoverage
                | Self::DrillDownPresentation
                | Self::PacksList
                | Self::ListOutbox
                | Self::ListDisclosures
                | Self::ReadCanonicalVisibility
                | Self::VerifyBlob
                | Self::GetFhirSupportMatrix
                | Self::ProjectRead
                | Self::ExperimentRead
                | Self::ProjectGraphRead
                | Self::DataSourceRead
                | Self::SnapshotPreview
                | Self::SnapshotRead
                | Self::SavedViewRead
                | Self::DatasetReleaseRead
                | Self::ParticipantRead
                | Self::RoomRead
                | Self::RoomMembershipRead
                | Self::ThreadRead
                | Self::MessageRead
                | Self::TaskRead
                | Self::NoteRead
                | Self::ApprovalRequestRead
                | Self::ActivityRead
                | Self::AgentIdentityRead
                | Self::ContextManifestRead
                | Self::AgentRunRead
                | Self::AgentLaneRead
                | Self::FleetRunRead
                | Self::ComparisonRead
                | Self::PrivacyRead
                | Self::BrowseRead
                | Self::AudioRead
                | Self::AnalyticsRead
                | Self::KnowledgeRead
                | Self::HubRead
                | Self::ComputeRead
                | Self::RWorkspaceRead
                | Self::ExtensionRead
                | Self::HuddleRead
                | Self::PackRead
                | Self::AdapterRead
                | Self::FederationRead
                | Self::ModelCatalogRead
        )
    }

    /// True when Spec 024 Strict enforcement requires a live `session_id`.
    #[must_use]
    pub const fn requires_client_session(self) -> bool {
        !self.is_session_bootstrap() && !self.is_read_or_health()
    }

    /// Broad grant set for CLI / IPC operator sessions (synthetic READY_BASE).
    #[must_use]
    pub fn operator_grants() -> Vec<Self> {
        vec![
            Self::Ping,
            Self::CreateSourceRecord,
            Self::CreateDerivedArtifact,
            Self::CreateProposal,
            Self::PromoteProposal,
            Self::CreateIdentityAssertion,
            Self::DecideIdentityMerge,
            Self::AppendAudit,
            Self::TransitionEffect,
            Self::ReadObject,
            Self::OpenSyntheticVault,
            Self::CloseVault,
            Self::IngestFhirSynthetic,
            Self::AttachValidatorEvidence,
            Self::RebuildProjection,
            Self::ReadCanonicalVisibility,
            Self::VerifyBlob,
            Self::BackupVault,
            Self::RestoreVault,
            Self::RunBlobGc,
            Self::GetTimeline,
            Self::GetBrief,
            Self::GetCoverage,
            Self::DrillDownPresentation,
            Self::CreateEncryptedVault,
            Self::OpenEncryptedVault,
            Self::CloseEncryptedVault,
            Self::NetworkBrokerInvoke,
            Self::SetEgressAllowlist,
            Self::PacksInstallLocal,
            Self::PacksList,
            Self::PacksPromote,
            Self::PacksEvaluateLocal,
            Self::DocumentIntake,
            Self::OcrStub,
            Self::AsrStub,
            Self::RetrieveLexical,
            Self::CreateExternalActionIntent,
            Self::ListOutbox,
            Self::NphiesInvoke,
            Self::OnlinePackAcquire,
            Self::MescArtifactAdmit,
            Self::MescArtifactVerify,
            Self::AmendAssertion,
            Self::GetFhirSupportMatrix,
            Self::ExportFhirLossAware,
            Self::RejectProposal,
            Self::AppendDisclosure,
            Self::ListDisclosures,
            Self::RevokeSession,
            Self::ProjectCreate,
            Self::ProjectRead,
            Self::ProjectUpdate,
            Self::ProjectArchive,
            Self::ExperimentCreate,
            Self::ExperimentRead,
            Self::ExperimentUpdate,
            Self::ExperimentArchive,
            Self::ProjectArtifactAttach,
            Self::ProjectArtifactDetach,
            Self::ProjectGraphRead,
            Self::ProjectGraphMutate,
            Self::DataSourceCreate,
            Self::DataSourceRead,
            Self::DataSourceUpdate,
            Self::DataSourceArchive,
            Self::SnapshotImport,
            Self::SnapshotPreview,
            Self::SnapshotRead,
            Self::SnapshotRefresh,
            Self::SavedViewCreate,
            Self::SavedViewRead,
            Self::SavedViewUpdate,
            Self::TransformExecute,
            Self::DatasetReleaseCreate,
            Self::DatasetReleaseRead,
            Self::ParticipantRegister,
            Self::ParticipantRead,
            Self::ParticipantRevoke,
            Self::RoomCreate,
            Self::RoomRead,
            Self::RoomUpdate,
            Self::RoomArchive,
            Self::RoomMembershipManage,
            Self::RoomMembershipRead,
            Self::ThreadCreate,
            Self::ThreadRead,
            Self::ThreadResolve,
            Self::MessagePost,
            Self::MessageRead,
            Self::MessageEdit,
            Self::TaskCreate,
            Self::TaskRead,
            Self::TaskUpdate,
            Self::NoteCreate,
            Self::NoteRead,
            Self::NoteUpdate,
            Self::ApprovalRequestCreate,
            Self::ApprovalRequestRead,
            Self::ApprovalDecide,
            Self::ApprovalWithdraw,
            Self::ActivityRead,
            Self::AgentIdentityRegister,
            Self::AgentIdentityRead,
            Self::AgentIdentityRevoke,
            Self::ContextManifestCreate,
            Self::ContextManifestRead,
            Self::AgentRunCreate,
            Self::AgentRunRead,
            Self::AgentRunStart,
            Self::AgentRunCancel,
            Self::AgentToolInvoke,
            Self::AgentRunExecute,
            Self::AgentRunComplete,
            Self::AgentRunFail,
            Self::AgentLaneCreate,
            Self::AgentLaneRead,
            Self::AgentLaneRetire,
            Self::FleetRunCreate,
            Self::FleetRunRead,
            Self::FleetRunDispatch,
            Self::FleetRunExecuteLane,
            Self::FleetRunCancel,
            Self::ComparisonCompute,
            Self::ComparisonRead,
            Self::PrivacyClassify,
            Self::PrivacyRead,
            Self::PrivacyProfileCreate,
            Self::PrivacyProfileRevoke,
            Self::PrivacyTransform,
            Self::PrivacyReceiptRevoke,
            Self::PrivacyMapCreate,
            Self::PrivacyMapRevoke,
            Self::PrivacyEgressEvaluate,
            Self::BrowseAllowlistManage,
            Self::BrowseRead,
            Self::BrowseRun,
            Self::BrowseCancel,
            Self::AudioImport,
            Self::AudioRead,
            Self::AudioCapture,
            Self::AudioTranscribe,
            Self::AudioCorrect,
            Self::AnalyticsQuery,
            Self::AnalyticsRead,
            Self::AnalyticsCohort,
            Self::KnowledgeIndex,
            Self::KnowledgeSearch,
            Self::KnowledgeRead,
            Self::KnowledgeCanvas,
            Self::HubAdmin,
            Self::HubRead,
            Self::HubClient,
            Self::ComputeSubmit,
            Self::ComputeRun,
            Self::ComputeRead,
            Self::RWorkspaceStage,
            Self::RWorkspaceRun,
            Self::RWorkspacePublish,
            Self::RWorkspaceRead,
            Self::ExtensionAdmin,
            Self::ExtensionInvoke,
            Self::ExtensionRead,
            Self::HuddleAct,
            Self::HuddleRead,
            Self::PackAdmin,
            Self::PackRead,
            Self::AdapterAdmin,
            Self::AdapterRead,
            Self::FederationAdmin,
            Self::FederationRead,
            Self::ModelCatalogRead,
        ]
    }
}

/// Request body variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum RequestBody {
    AcquireLease {
        client_id: OpaqueId,
        holder_id_hint: Option<OpaqueId>,
    },
    ReleaseLease {
        holder_id: OpaqueId,
    },
    Ping,
    CreateSourceRecord {
        media_type: String,
        bytes: Vec<u8>,
    },
    CreateDerivedArtifact {
        source_id: OpaqueId,
        transform_id: String,
        transform_version: String,
        bytes: Vec<u8>,
    },
    CreateProposal {
        subject_ref: Option<OpaqueId>,
        claim_kind: String,
        payload: Value,
        evidence_refs: Vec<OpaqueId>,
    },
    PromoteProposal {
        proposal_id: OpaqueId,
        authorized_by: OpaqueId,
        subject_ref: OpaqueId,
    },
    CreateIdentityAssertion {
        subject_id: OpaqueId,
        identifier_system: String,
        identifier_value: String,
    },
    DecideIdentityMerge {
        surviving_subject_id: OpaqueId,
        merged_subject_ids: Vec<OpaqueId>,
        authorized_by: OpaqueId,
        rationale: String,
    },
    AppendAudit {
        actor: OpaqueId,
        action: String,
        target_refs: Vec<OpaqueId>,
        detail: Option<Value>,
    },
    TransitionEffect {
        action_id: OpaqueId,
        to: EffectState,
        reconcile_token: Option<String>,
    },
    ReadObject {
        object_id: OpaqueId,
    },
    OpenSyntheticVault {
        vault_root: String,
    },
    CloseVault,
    IngestFhirSynthetic {
        media_type: String,
        bytes: Vec<u8>,
        fhir_version_hint: Option<String>,
        attach_validator_fixture_id: Option<String>,
    },
    AttachValidatorEvidence {
        source_id: OpaqueId,
        evaluator: String,
        outcome: String,
        issue_codes: Vec<String>,
    },
    RebuildProjection {
        kind: String,
        built_from: Vec<OpaqueId>,
    },
    ReadCanonicalVisibility {
        source_id: OpaqueId,
    },
    VerifyBlob {
        digest: DigestSha256,
    },
    BackupVault {
        destination: String,
    },
    RestoreVault {
        source: String,
        destination: String,
    },
    RunBlobGc,
    GetTimeline {
        subject_ref: OpaqueId,
    },
    GetBrief {
        subject_ref: OpaqueId,
    },
    GetCoverage {
        subject_ref: OpaqueId,
    },
    DrillDownPresentation {
        subject_ref: OpaqueId,
        field_key: String,
        assertion_id: Option<OpaqueId>,
    },
    CreateEncryptedVault {
        vault_root: String,
        passphrase: String,
    },
    OpenEncryptedVault {
        vault_root: String,
        passphrase: Option<String>,
        recovery_code: Option<String>,
    },
    CloseEncryptedVault,
    NetworkBrokerInvoke {
        request: NetworkBrokerRequest,
    },
    SetEgressAllowlist {
        entries: Vec<EgressAllowlistEntry>,
    },
    PacksInstallLocal {
        local_path: String,
    },
    PacksList,
    PacksPromote {
        pack_id: OpaqueId,
        to: PackPromotionState,
    },
    PacksEvaluateLocal {
        request: PackEvaluationRequest,
    },
    /// Spec 105: bounded local text generation (evidence-only proposal).
    PacksGenerateLocal {
        request: PackGenerationRequest,
    },
    /// Spec 103: list the prepared models resident in Core.
    PacksResidency,
    /// Spec 103: unload a Pack's prepared models (`None`: every unpinned one).
    PacksUnloadLocal {
        pack_id: Option<OpaqueId>,
    },
    DocumentIntake {
        request: DocumentIntakeRequest,
    },
    OcrStub {
        request: OcrStubRequest,
    },
    AsrStub {
        request: AsrStubRequest,
    },
    RetrieveLexical {
        request: LexicalRetrieveRequest,
    },
    CreateExternalActionIntent {
        request: CreateExternalActionIntentRequest,
    },
    ListOutbox,
    NphiesInvoke {
        request: NphiesInvokeRequest,
    },
    OnlinePackAcquire {
        request: OnlinePackAcquireRequest,
    },
    MescArtifactAdmit {
        request: MescArtifactAdmitRequest,
    },
    MescArtifactVerify {
        request: MescArtifactVerifyRequest,
    },
    OpenSession {
        holder_id: OpaqueId,
        granted: Vec<Capability>,
        ttl_ticks: u64,
    },
    RevokeSession {
        session_id: OpaqueId,
    },
    AmendAssertion {
        prior_assertion_id: OpaqueId,
        authorized_by: OpaqueId,
        payload: Value,
        effective_time: Option<MedicalTime>,
        kind: AmendmentKind,
        rationale: String,
    },
    GetFhirSupportMatrix,
    ExportFhirLossAware {
        resource: Value,
    },
    /// Spec 021: loss-aware FHIR export of every assertion for one subject,
    /// with its disclosure record appended in the same Core step.
    ExportSubjectLossAware {
        subject_ref: OpaqueId,
        purpose: String,
    },
    RejectProposal {
        proposal_id: OpaqueId,
        actor: OpaqueId,
        rationale: String,
    },
    AppendDisclosure {
        purpose: String,
        scope: String,
        subject_ref: Option<OpaqueId>,
        artifact_refs: Vec<OpaqueId>,
        export_digest: Option<DigestSha256>,
        note: Option<String>,
    },
    ListDisclosures,
    // Spec 074: every mutation flows through Core authority paths. Surfaces
    // never write project storage directly.
    ProjectCreate {
        name: String,
        description: Option<String>,
    },
    ProjectGet {
        project_id: OpaqueId,
    },
    ProjectList {
        status: Option<ProjectStatus>,
        limit: Option<u32>,
        cursor: Option<String>,
    },
    ProjectUpdate {
        project_id: OpaqueId,
        expected_revision: u64,
        name: Option<String>,
        description: Option<Option<String>>,
    },
    ProjectArchive {
        project_id: OpaqueId,
        expected_revision: u64,
    },
    ProjectRestore {
        project_id: OpaqueId,
        expected_revision: u64,
    },
    ExperimentCreate {
        project_id: OpaqueId,
        name: String,
        description: Option<String>,
    },
    ExperimentGet {
        experiment_id: OpaqueId,
    },
    ExperimentList {
        project_id: OpaqueId,
        limit: Option<u32>,
        cursor: Option<String>,
    },
    ExperimentUpdate {
        experiment_id: OpaqueId,
        expected_revision: u64,
        name: Option<String>,
        description: Option<Option<String>>,
    },
    ExperimentArchive {
        experiment_id: OpaqueId,
        expected_revision: u64,
    },
    ProjectAttach {
        project_id: OpaqueId,
        experiment_id: Option<OpaqueId>,
        artifact: ArtifactDescriptor,
    },
    ProjectDetach {
        ref_id: OpaqueId,
        expected_revision: u64,
    },
    ProjectListRefs {
        project_id: OpaqueId,
        experiment_id: Option<OpaqueId>,
        active_only: bool,
        limit: Option<u32>,
        cursor: Option<String>,
    },
    GraphEdgeCreate {
        project_id: OpaqueId,
        subject: GraphEndpoint,
        predicate: ProjectGraphPredicate,
        object: GraphEndpoint,
    },
    GraphEdgeRemove {
        edge_id: OpaqueId,
        expected_revision: u64,
    },
    GraphNeighbors {
        project_id: OpaqueId,
        start: GraphEndpoint,
        predicates: Option<Vec<ProjectGraphPredicate>>,
        direction: Option<GraphDirection>,
        limit: Option<u32>,
        cursor: Option<String>,
    },
    ProjectContextResolve {
        project_id: OpaqueId,
        experiment_id: Option<OpaqueId>,
        refs_limit: Option<u32>,
        graph_limit: Option<u32>,
    },
    ProjectSummaryQuery {
        project_id: OpaqueId,
    },
    // Spec 075: every mutation flows through Core authority paths. Surfaces
    // never write data-source storage directly.
    DataSourceCreate {
        project_id: OpaqueId,
        display_name: String,
        locator: SourceLocator,
        credential_ref: Option<OpaqueId>,
    },
    DataSourceGet {
        source_id: OpaqueId,
    },
    DataSourceList {
        project_id: OpaqueId,
        limit: Option<u32>,
        cursor: Option<String>,
    },
    DataSourceUpdate {
        source_id: OpaqueId,
        expected_revision: u64,
        display_name: Option<String>,
        credential_ref: Option<Option<OpaqueId>>,
    },
    DataSourceArchive {
        source_id: OpaqueId,
        expected_revision: u64,
    },
    SnapshotImport {
        source_id: OpaqueId,
    },
    SnapshotPreview {
        source_id: OpaqueId,
        max_rows: Option<u32>,
    },
    SnapshotGet {
        snapshot_id: OpaqueId,
    },
    SnapshotList {
        source_id: OpaqueId,
        limit: Option<u32>,
        cursor: Option<String>,
    },
    SnapshotRows {
        snapshot_id: OpaqueId,
        limit: Option<u32>,
        cursor: Option<String>,
        filters: Vec<FilterExpr>,
        sort: Vec<SortKey>,
    },
    SnapshotRefresh {
        source_id: OpaqueId,
        allow_schema_change: bool,
    },
    SavedViewCreate {
        snapshot_id: OpaqueId,
        view_kind: DataViewKind,
        state: ViewState,
    },
    SavedViewGet {
        view_id: OpaqueId,
    },
    SavedViewList {
        snapshot_id: OpaqueId,
        limit: Option<u32>,
        cursor: Option<String>,
    },
    SavedViewUpdate {
        view_id: OpaqueId,
        expected_revision: u64,
        state: ViewState,
    },
    TransformExecute {
        input_snapshot_ids: Vec<OpaqueId>,
        ops: Vec<TransformOp>,
    },
    DatasetReleaseCreate {
        snapshot_id: OpaqueId,
        version: String,
        split_group: Option<String>,
        annotation_schema_ref: Option<String>,
        rights_state: crate::data_sources::RightsState,
    },
    DatasetReleaseGet {
        release_id: OpaqueId,
    },
    DatasetReleaseList {
        project_id: OpaqueId,
        limit: Option<u32>,
        cursor: Option<String>,
    },
    // Spec 076: every mutation flows through Core authority paths. Surfaces
    // never write collaboration storage directly. T076-03 slice only.
    ParticipantRegister {
        holder_id: OpaqueId,
        kind: ParticipantKind,
        display_name: String,
        agent_profile_ref: Option<OpaqueId>,
    },
    ParticipantGet {
        participant_id: OpaqueId,
    },
    ParticipantRevoke {
        participant_id: OpaqueId,
        expected_revision: u64,
    },
    RoomCreate {
        project_id: OpaqueId,
        experiment_id: Option<OpaqueId>,
        name: String,
    },
    RoomGet {
        room_id: OpaqueId,
    },
    RoomList {
        project_id: OpaqueId,
        status: Option<RoomStatus>,
        limit: Option<u32>,
        cursor: Option<String>,
    },
    RoomRename {
        room_id: OpaqueId,
        expected_revision: u64,
        name: String,
    },
    RoomArchive {
        room_id: OpaqueId,
        expected_revision: u64,
    },
    RoomMembershipAdd {
        room_id: OpaqueId,
        participant_id: OpaqueId,
        role: MembershipRole,
    },
    RoomMembershipList {
        room_id: OpaqueId,
    },
    RoomMembershipRemove {
        room_id: OpaqueId,
        membership_id: OpaqueId,
        expected_revision: u64,
    },
    // Spec 076 T076-04/05/06 slice: thread, message, task.
    ThreadOpen {
        room_id: OpaqueId,
        anchor: AnchorTarget,
    },
    ThreadGet {
        thread_id: OpaqueId,
    },
    ThreadList {
        room_id: OpaqueId,
        limit: Option<u32>,
        cursor: Option<String>,
    },
    ThreadSetStatus {
        thread_id: OpaqueId,
        expected_revision: u64,
        status: ThreadStatus,
    },
    MessagePost {
        thread_id: OpaqueId,
        body: String,
    },
    MessageList {
        thread_id: OpaqueId,
        limit: Option<u32>,
        after_seq: Option<u64>,
    },
    MessageEditBody {
        message_id: OpaqueId,
        new_body: String,
    },
    MessageDelete {
        message_id: OpaqueId,
    },
    TaskCreate {
        room_id: OpaqueId,
        anchor: Option<AnchorTarget>,
        title: String,
        description: Option<String>,
    },
    TaskGet {
        task_id: OpaqueId,
    },
    TaskList {
        room_id: OpaqueId,
        limit: Option<u32>,
        cursor: Option<String>,
    },
    TaskUpdate {
        task_id: OpaqueId,
        expected_revision: u64,
        status: TaskStatus,
        assignee_participant_id: Option<OpaqueId>,
    },
    // Spec 076 T076-07/08/09 slice: note, approval, activity.
    NoteCreate {
        room_id: OpaqueId,
        title: String,
        body: String,
    },
    NoteGet {
        note_id: OpaqueId,
    },
    NoteListRevisions {
        note_id: OpaqueId,
    },
    NoteEdit {
        note_id: OpaqueId,
        expected_revision: u64,
        body: String,
    },
    ApprovalRequestCreate {
        room_id: OpaqueId,
        anchor: AnchorTarget,
        kind: ApprovalKind,
        assignee_participant_ids: Vec<OpaqueId>,
        blind_until_closed: bool,
    },
    ApprovalRequestGet {
        request_id: OpaqueId,
    },
    ApprovalRequestWithdraw {
        request_id: OpaqueId,
        expected_revision: u64,
    },
    ApprovalDecide {
        request_id: OpaqueId,
        outcome: ApprovalDecisionOutcome,
        rationale: Option<String>,
    },
    ApprovalDecisionList {
        request_id: OpaqueId,
    },
    ActivityList {
        room_id: OpaqueId,
        limit: Option<u32>,
        after_seq: Option<u64>,
    },
    // Spec 077: MedAgent Workbench. Every mutation flows through Core
    // authority paths; surfaces never write medagent storage directly.
    // T077-03 slice only (AgentIdentity + AgentCapabilityManifest);
    // ContextManifest/AgentRun/tool/receipt/proposal land in later slices.
    AgentIdentityRegister {
        project_id: OpaqueId,
        pack_id: OpaqueId,
        display_name: String,
        granted_tool_kinds: Vec<ToolKind>,
    },
    AgentIdentityGet {
        agent_id: OpaqueId,
    },
    AgentIdentityList {
        project_id: OpaqueId,
        limit: Option<u32>,
    },
    AgentIdentityRevoke {
        agent_id: OpaqueId,
        expected_revision: u64,
    },
    // Spec 077 T077-04 slice.
    ContextManifestCreate {
        project_id: OpaqueId,
        selected_artifacts: Vec<ArtifactDescriptor>,
    },
    ContextManifestGet {
        context_id: OpaqueId,
    },
    // Spec 077 T077-05 slice.
    AgentRunCreate {
        project_id: OpaqueId,
        agent_identity_id: OpaqueId,
        context_manifest_id: OpaqueId,
        prompt: String,
    },
    AgentRunGet {
        run_id: OpaqueId,
    },
    AgentRunList {
        project_id: OpaqueId,
        agent_id: Option<OpaqueId>,
        status: Option<AgentRunState>,
        limit: Option<u32>,
    },
    AgentRunStart {
        run_id: OpaqueId,
        expected_revision: u64,
    },
    AgentRunCancel {
        run_id: OpaqueId,
        expected_revision: u64,
    },
    AgentRunTurnList {
        run_id: OpaqueId,
    },
    // Spec 077 T077-06 slice.
    AgentToolInvoke {
        run_id: OpaqueId,
        kind: ToolKind,
        arguments: Value,
    },
    // Spec 077 T077-07 slice.
    AgentRunExecute {
        run_id: OpaqueId,
        /// Local pack directory path; never persisted (`PackManifestV0` is
        /// purely content-addressed), so it is caller-supplied on every
        /// call, exactly like `PacksEvaluateLocal`.
        local_path: String,
        max_tokens: u32,
        /// Mirrors `PackEvaluationRequest.synthetic_only`: real PHI
        /// flowing through this local model runtime requires a later,
        /// explicit gate this spec does not grant.
        synthetic_only: bool,
    },
    // Spec 077 T077-08 slice.
    AgentRunComplete {
        run_id: OpaqueId,
        expected_revision: u64,
    },
    AgentRunFail {
        run_id: OpaqueId,
        expected_revision: u64,
        failure_reason: String,
    },
    // Spec 078 T078-03 slice: AgentLane + LanePolicy.
    AgentLaneCreate {
        project_id: OpaqueId,
        agent_identity_id: OpaqueId,
        context_manifest_id: OpaqueId,
        role_label: String,
        /// `None` inherits the identity's full grant; `Some` must be a
        /// non-empty subset (`security.md` T2).
        granted_tool_kinds: Option<Vec<ToolKind>>,
        /// `None` inherits the full context manifest; `Some` must be a
        /// non-empty subset (`security.md` T2).
        context_artifact_ids: Option<Vec<OpaqueId>>,
    },
    AgentLaneGet {
        lane_id: OpaqueId,
    },
    AgentLaneList {
        project_id: OpaqueId,
        status: Option<AgentLaneStatus>,
        limit: Option<u32>,
    },
    AgentLaneRetire {
        lane_id: OpaqueId,
        expected_revision: u64,
    },
    // Spec 078 T078-04 slice: FleetRun lifecycle.
    FleetRunCreate {
        project_id: OpaqueId,
        task_prompt: String,
    },
    FleetRunGet {
        fleet_run_id: OpaqueId,
    },
    FleetRunList {
        project_id: OpaqueId,
        status: Option<FleetRunState>,
        limit: Option<u32>,
    },
    /// Binds and starts one real Spec 077 `AgentRun` per lane.
    FleetRunDispatch {
        fleet_run_id: OpaqueId,
        expected_revision: u64,
        lane_ids: Vec<OpaqueId>,
    },
    /// Executes one dispatched lane through Spec 077 `execute_agent_run`.
    FleetRunExecuteLane {
        fleet_run_id: OpaqueId,
        lane_id: OpaqueId,
        local_path: String,
        max_tokens: usize,
        synthetic_only: bool,
    },
    FleetRunCancel {
        fleet_run_id: OpaqueId,
        expected_revision: u64,
    },
    // Spec 078 T078-05/06 slice: comparison + history.
    ComparisonCompute {
        fleet_run_id: OpaqueId,
    },
    ComparisonReportList {
        fleet_run_id: OpaqueId,
    },
    // Spec 079: Privacy Gate.
    /// Declares an artifact's class in a Project. `expected_revision` is
    /// `None` for the first classification and required afterwards.
    PrivacyClassify {
        project_id: OpaqueId,
        artifact_id: OpaqueId,
        data_class: DataClass,
        expected_revision: Option<u64>,
    },
    PrivacyClassificationGet {
        project_id: OpaqueId,
        artifact_id: OpaqueId,
    },
    PrivacyClassificationList {
        project_id: OpaqueId,
    },
    PrivacyProfileCreate {
        project_id: OpaqueId,
        name: String,
        target_class: DataClass,
        rules: Vec<ProfileRule>,
        use_model_recognizer: bool,
    },
    PrivacyProfileGet {
        profile_id: OpaqueId,
    },
    PrivacyProfileList {
        project_id: OpaqueId,
    },
    PrivacyProfileRevoke {
        profile_id: OpaqueId,
        expected_revision: u64,
    },
    PrivacyMapCreate {
        project_id: OpaqueId,
    },
    PrivacyMapList {
        project_id: OpaqueId,
    },
    PrivacyMapRevoke {
        map_id: OpaqueId,
        expected_revision: u64,
    },
    /// Transforms a source into a new de-identified derived artifact.
    PrivacyTransform {
        project_id: OpaqueId,
        source_artifact_id: OpaqueId,
        profile_id: OpaqueId,
        pseudonym_map_id: Option<OpaqueId>,
        /// Admitted local model Pack id + directory for the model recognizer.
        model_pack_id: Option<OpaqueId>,
        model_pack_path: Option<String>,
        synthetic_only: bool,
    },
    PrivacyReceiptGet {
        receipt_id: OpaqueId,
    },
    PrivacyReceiptList {
        project_id: OpaqueId,
    },
    PrivacyReceiptRevoke {
        receipt_id: OpaqueId,
        expected_revision: u64,
    },
    PrivacyReidentify {
        map_id: OpaqueId,
        pseudonym: String,
        reason: String,
    },
    PrivacyReidentificationAuditList {
        map_id: OpaqueId,
    },
    /// The egress decision every boundary must request before sending.
    PrivacyEgressEvaluate {
        project_id: OpaqueId,
        artifact_id: OpaqueId,
        boundary: EgressBoundary,
    },
    PrivacyEgressDecisionList {
        project_id: OpaqueId,
        artifact_id: Option<OpaqueId>,
    },
    // Spec 080: Governed Browse.
    BrowseAllowlistAdd {
        project_id: OpaqueId,
        host: String,
        path_prefix: String,
    },
    BrowseAllowlistList {
        project_id: OpaqueId,
    },
    BrowseAllowlistDisable {
        entry_id: OpaqueId,
        expected_revision: u64,
    },
    BrowseRouteList,
    /// Runs one read-only Browse request under the full policy.
    BrowseRun {
        request: BrowseRequest,
    },
    BrowseSessionGet {
        session_id: OpaqueId,
    },
    BrowseSessionList {
        project_id: OpaqueId,
    },
    /// Cancels a session awaiting human takeover.
    BrowseSessionCancel {
        session_id: OpaqueId,
        expected_revision: u64,
    },
    // Spec 081: AudioFlow Foundation (local only).
    /// Imports a 16-bit PCM WAV file as an immutable source.
    AudioImport {
        project_id: OpaqueId,
        label: String,
        wav: Vec<u8>,
    },
    AudioSourceGet {
        source_id: OpaqueId,
    },
    AudioSourceList {
        project_id: OpaqueId,
    },
    /// Starts a capture. Only an explicit request starts one.
    AudioCaptureStart {
        project_id: OpaqueId,
        label: String,
        backend: CaptureBackendKind,
        format: PcmFormat,
    },
    /// Pushes PCM frames into a recording scripted capture.
    AudioCaptureAppend {
        session_id: OpaqueId,
        expected_revision: u64,
        frames: Vec<u8>,
    },
    /// Pause (`paused`), resume (`recording`) or cancel (`cancelled`).
    AudioCaptureTransition {
        session_id: OpaqueId,
        expected_revision: u64,
        to: CaptureState,
    },
    AudioCaptureStop {
        session_id: OpaqueId,
        expected_revision: u64,
    },
    AudioCaptureGet {
        session_id: OpaqueId,
    },
    AudioCaptureList {
        project_id: OpaqueId,
    },
    AudioRouteList,
    AudioTranscribe {
        request: AudioRouteRequest,
    },
    AudioTranscriptList {
        source_id: OpaqueId,
    },
    AudioTranscriptGet {
        revision_id: OpaqueId,
    },
    AudioReceiptList {
        source_id: OpaqueId,
    },
    /// Corrects segments of the latest revision into a new revision.
    AudioTranscriptCorrect {
        revision_id: OpaqueId,
        /// `(segment seq, corrected text)`.
        edits: Vec<(u32, String)>,
        reason: String,
    },
    AudioEvidenceGet {
        revision_id: OpaqueId,
        segment_seq: u32,
    },
    // Spec 082: Analytics Gate (read-only over exact snapshots).
    /// Runs one read-only SQL query over bound snapshots.
    AnalyticsQuery {
        request: QueryRequest,
    },
    /// Re-runs a receipt against its pinned inputs; writes nothing.
    AnalyticsReplay {
        receipt_id: OpaqueId,
    },
    AnalyticsReceiptGet {
        receipt_id: OpaqueId,
    },
    AnalyticsReceiptList {
        project_id: OpaqueId,
    },
    AnalyticsResultGet {
        result_id: OpaqueId,
    },
    AnalyticsStatistics {
        result_id: OpaqueId,
        column: String,
        kinds: Vec<StatisticKind>,
    },
    AnalyticsCohortCreate {
        project_id: OpaqueId,
        label: String,
        snapshot_id: OpaqueId,
        criteria: Vec<CohortCriterion>,
    },
    AnalyticsCohortList {
        project_id: OpaqueId,
    },
    AnalyticsCohortRun {
        cohort_id: OpaqueId,
        max_rows: Option<u32>,
    },
    // Spec 083: Knowledge + Research Canvas (lexical index is a projection).
    /// Builds the next index version from the Project's current sources.
    KnowledgeIndexBuild {
        project_id: OpaqueId,
    },
    KnowledgeIndexStatus {
        project_id: OpaqueId,
    },
    /// Runs one lexical retrieval and records its receipt.
    KnowledgeSearch {
        request: RetrievalRequest,
    },
    KnowledgeReceiptGet {
        receipt_id: OpaqueId,
    },
    KnowledgeReceiptList {
        project_id: OpaqueId,
    },
    CanvasCreate {
        project_id: OpaqueId,
        title: String,
    },
    /// Applies edits to `expected_revision`, writing revision n+1.
    CanvasEdit {
        canvas_id: OpaqueId,
        expected_revision: u32,
        ops: Vec<CanvasOp>,
    },
    /// A revision (latest by default) resolved live, with inspection.
    CanvasGet {
        canvas_id: OpaqueId,
        revision: Option<u32>,
    },
    CanvasList {
        project_id: OpaqueId,
    },
    // Spec 084: MedScale Hub foundation (local transport only).
    /// Gives this vault its Hub role.
    HubInit,
    HubInvite {
        project_id: OpaqueId,
        display_name: String,
    },
    HubInvitationRevoke {
        invitation_id: OpaqueId,
    },
    HubDeviceRevoke {
        device_id: OpaqueId,
    },
    HubStatus,
    /// Redeems an invitation (bootstrap; proves key possession).
    HubEnroll {
        token_hex: String,
        public_key_hex: String,
        signature_hex: String,
    },
    HubChallenge {
        device_id: OpaqueId,
    },
    HubHandshake {
        handshake: HubHandshake,
    },
    /// Applies envelopes in order (device session only).
    HubSubmit {
        envelopes: Vec<SyncEnvelope>,
    },
    HubPull {
        after: u64,
        limit: u32,
    },
    /// Client: generates a device key and signs the enrollment proof.
    HubJoinPrepare {
        code: HubInvitationCode,
    },
    HubJoinComplete {
        link_id: OpaqueId,
        endpoint: String,
        code: Box<HubInvitationCode>,
        device: Box<DeviceIdentity>,
    },
    HubQueue {
        link_id: OpaqueId,
        intent: SyncIntent,
    },
    HubSignHandshake {
        link_id: OpaqueId,
        challenge: HubChallenge,
    },
    HubRecordOutcomes {
        link_id: OpaqueId,
        outcomes: Vec<(u64, SyncOutcome)>,
    },
    HubMirrorAppend {
        link_id: OpaqueId,
        page: HubEventPage,
    },
    HubLinkList,
    HubLinkGet {
        link_id: OpaqueId,
    },
    HubOutboxList {
        link_id: OpaqueId,
        pending_only: bool,
    },
    HubMirrorList {
        link_id: OpaqueId,
        after: u64,
        limit: u32,
    },
    // Spec 085: MedScale Compute (closed job kinds; no arbitrary code).
    ComputeSubmit {
        request: Box<ComputeJobRequest>,
    },
    /// Runs one queued job in the MedScale worker (synchronous).
    ComputeRun {
        job_id: OpaqueId,
    },
    /// Cancels a queued job.
    ComputeCancel {
        job_id: OpaqueId,
    },
    /// Marks jobs orphaned in `running` as `interrupted`.
    ComputeRecover,
    ComputeJobGet {
        job_id: OpaqueId,
    },
    ComputeJobList {
        project_id: OpaqueId,
    },
    ComputeStatus,
    // Spec 086: R Workspace. Paths and programs come from host
    // configuration, never from a request.
    RWorkspaceStage {
        request: Box<RStageRequest>,
    },
    RWorkspaceInspect {
        workspace_id: OpaqueId,
    },
    RWorkspaceLaunch {
        request: RLaunchRequest,
    },
    /// Recorded and refused: managed R execution is not admitted.
    RWorkspaceRun {
        request: RRunRequest,
    },
    RWorkspacePublish {
        request: Box<RPublishRequest>,
    },
    RWorkspaceGet {
        workspace_id: OpaqueId,
    },
    RWorkspaceList {
        project_id: OpaqueId,
    },
    RWorkspacePublished {
        table_id: OpaqueId,
    },
    RWorkspaceStatus,
    // Spec 087: Community Extensions. Packs travel as bytes; no request
    // carries a path, and no extension code runs.
    ExtensionTrustPublisher {
        publisher_id: String,
        key_hex: String,
    },
    ExtensionRevokePublisher {
        publisher_id: String,
    },
    ExtensionRevokeRelease {
        digest: DigestSha256,
    },
    ExtensionInstall {
        project_id: OpaqueId,
        pack_json: String,
        upgrade: bool,
    },
    ExtensionRollback {
        project_id: OpaqueId,
        extension_id: String,
    },
    ExtensionSetEnabled {
        project_id: OpaqueId,
        extension_id: String,
        enabled: bool,
    },
    ExtensionUninstall {
        project_id: OpaqueId,
        extension_id: String,
    },
    /// `ceiling = None` revokes the capability.
    ExtensionGrant {
        project_id: OpaqueId,
        extension_id: String,
        capability: ExtensionCapability,
        ceiling: Option<crate::privacy_gate::DataClass>,
    },
    ExtensionInvoke {
        project_id: OpaqueId,
        extension_id: String,
        command: String,
        target: Option<OpaqueId>,
    },
    ExtensionList {
        project_id: OpaqueId,
    },
    // Spec 088: AudioFlow Advanced huddles (consent-gated acts).
    HuddleAct {
        act: Box<crate::huddles::HuddleActRequest>,
    },
    HuddleGet {
        huddle_id: OpaqueId,
    },
    HuddleList {
        project_id: OpaqueId,
    },
    // Spec 089: Research Packs.
    PackAct {
        act: Box<crate::research_packs::PackActRequest>,
    },
    PackCatalog,
    PackInstallGet {
        project_id: OpaqueId,
        pack_id: String,
    },
    PackArtifactGet {
        artifact_id: OpaqueId,
    },
    PackArtifactList {
        project_id: OpaqueId,
        pack_id: String,
    },
    // Spec 090: institutional adapters.
    AdapterAct {
        act: Box<crate::institutional::AdapterActRequest>,
    },
    AdapterGet {
        adapter_id: OpaqueId,
    },
    AdapterList {
        project_id: OpaqueId,
    },
    // Spec 091: federation.
    FederationAct {
        act: Box<crate::federation::FederationActRequest>,
    },
    FederationGet,
    // Spec 103: read-only query over a locally supplied catalog snapshot.
    ModelCatalogQuery {
        query: Box<crate::model_catalog::ModelCatalogQueryRequest>,
    },
}

impl RequestBody {
    /// True for the Spec 074 graph mutations plus the Spec 075 snapshot
    /// import/refresh/transform ops that provably leave the in-memory
    /// authority snapshot untouched (durable sqlite rows plus
    /// content-addressed blobs and revisioned receipts only; 075 ids come
    /// from the sqlite sequence). The facade skips the frozen full-snapshot
    /// rewrite for these ops; skipping is output-identical because the
    /// snapshot bytes cannot change. Locked by the Core snapshot-stability
    /// test; any op that gains a memory write must leave this set.
    #[must_use]
    pub fn preserves_memory_snapshot(&self) -> bool {
        matches!(
            self,
            Self::ProjectAttach { .. }
                | Self::ProjectDetach { .. }
                | Self::GraphEdgeCreate { .. }
                | Self::GraphEdgeRemove { .. }
                | Self::SnapshotImport { .. }
                | Self::SnapshotRefresh { .. }
                | Self::TransformExecute { .. }
        )
    }
}

/// Successful response body variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "ok", rename_all = "snake_case")]
pub enum ResponseBody {
    Lease {
        holder_id: OpaqueId,
        vault_id: VaultId,
    },
    Pong {
        schema_version: u32,
    },
    Created {
        object_id: OpaqueId,
    },
    Promoted {
        assertion_id: OpaqueId,
        audit_id: OpaqueId,
    },
    Effect {
        action_id: OpaqueId,
        state: EffectState,
    },
    Object {
        /// JSON encoding of the object; never includes DB/key handles.
        value: Value,
    },
    Released,
    VaultOpened {
        vault_root: String,
    },
    VaultClosed,
    Ingested {
        receipt: IngestReceipt,
    },
    Visibility {
        source_id: OpaqueId,
        visible: bool,
        content_digest: DigestSha256,
        byte_length: u64,
        blob_ok: bool,
    },
    BlobVerified {
        digest: DigestSha256,
        byte_length: u64,
        state: crate::ingest::BlobState,
    },
    Backup {
        manifest: BackupManifest,
    },
    Restored {
        sources_restored: u64,
        blobs_restored: u64,
    },
    Gc {
        tombstoned: u64,
        swept: u64,
    },
    Timeline {
        body: SubjectTimelineV1,
        projection_id: Option<OpaqueId>,
    },
    Brief {
        body: SubjectBriefV1,
        projection_id: Option<OpaqueId>,
    },
    Coverage {
        body: SubjectCoverageV1,
        projection_id: Option<OpaqueId>,
    },
    DrillDown {
        result: DrillDownResult,
    },
    EncryptedVaultReady {
        vault_root: String,
        recovery_codes: Option<Vec<String>>,
    },
    NetworkBroker {
        result: NetworkBrokerResult,
    },
    AllowlistSet {
        entries: u32,
    },
    PackAdmit {
        result: PackAdmitResult,
    },
    PackList {
        packs: Vec<PackManifestV0>,
    },
    PackPromoted {
        pack_id: OpaqueId,
        state: PackPromotionState,
    },
    PackEvaluation {
        result: PackEvaluationResult,
    },
    PackResidency {
        view: Box<crate::packs::PackResidencyView>,
    },
    DocumentIntake {
        result: DocumentIntakeResult,
    },
    MediaStub {
        result: MediaStubResult,
    },
    LexicalRetrieve {
        result: LexicalRetrieveResult,
    },
    Outbox {
        entries: Vec<OutboxEntry>,
    },
    Session {
        session_id: OpaqueId,
        expires_at_tick: u64,
    },
    Amended {
        assertion_id: OpaqueId,
        amendment_id: OpaqueId,
        audit_id: OpaqueId,
    },
    FhirSupportMatrix {
        matrix: crate::fhir::FhirSupportMatrix,
    },
    FhirLossAwareExport {
        export: crate::fhir::FhirLossAwareExport,
    },
    SubjectFhirExport {
        exports: Vec<crate::fhir::FhirLossAwareExport>,
        record: DisclosureRecord,
    },
    Rejected {
        proposal_id: OpaqueId,
        audit_id: OpaqueId,
    },
    DisclosureAppended {
        record: DisclosureRecord,
    },
    DisclosureList {
        records: Vec<DisclosureRecord>,
    },
    MescVerify {
        report: MescVerifyReport,
    },
    // Spec 074 typed results (revisioned, scope-checked, no raw payloads).
    Project {
        project: Project,
    },
    ProjectList {
        projects: Vec<ProjectSummary>,
        next_cursor: Option<String>,
    },
    Experiment {
        experiment: Experiment,
    },
    ExperimentList {
        experiments: Vec<ExperimentSummary>,
        next_cursor: Option<String>,
    },
    ProjectRef {
        reference: ProjectArtifactRef,
    },
    ProjectRefList {
        refs: Vec<ResolvedArtifactRef>,
        next_cursor: Option<String>,
    },
    GraphEdge {
        edge: ProjectGraphEdge,
    },
    GraphNeighbors {
        page: GraphNeighborPage,
    },
    ProjectContext {
        context: ProjectContext,
    },
    ProjectSummary {
        summary: ProjectSummary,
    },
    // Spec 075 typed results (immutable snapshots, scope-checked, no secrets).
    // Large payloads are boxed: `large_enum_variant` keeps every variant
    // small while the payloads themselves stay owned typed values.
    DataSource {
        source: Box<DataSourceManifest>,
    },
    DataSourceList {
        sources: Vec<DataSourceSummary>,
        next_cursor: Option<String>,
    },
    SnapshotImported {
        snapshot: Box<crate::data_sources::DataSnapshot>,
        receipt: crate::data_sources::ImportReceipt,
    },
    SnapshotPreview {
        schema: SourceSchema,
        rows: Vec<Vec<crate::data_sources::CellValue>>,
    },
    Snapshot {
        snapshot: Box<crate::data_sources::DataSnapshot>,
    },
    SnapshotList {
        snapshots: Vec<SnapshotSummary>,
        next_cursor: Option<String>,
    },
    SnapshotRows {
        page: SnapshotRowPage,
    },
    SnapshotRefreshed {
        receipt: RefreshReceipt,
        snapshot: Option<Box<crate::data_sources::DataSnapshot>>,
    },
    SavedView {
        view: Box<SavedDataView>,
    },
    SavedViewList {
        views: Vec<SavedViewSummary>,
        next_cursor: Option<String>,
    },
    Transformed {
        snapshot: Box<crate::data_sources::DataSnapshot>,
        receipt: TransformationReceipt,
    },
    DatasetRelease {
        release: Box<ReleaseManifest>,
    },
    DatasetReleaseList {
        releases: Vec<DatasetReleaseSummary>,
        next_cursor: Option<String>,
    },
    // Spec 076 typed results (revisioned, membership-gated, no secrets).
    // T076-03 slice only.
    Participant {
        participant: Box<ParticipantIdentity>,
    },
    CollabRoom {
        room: Box<Room>,
    },
    CollabRoomList {
        rooms: Vec<Room>,
        next_cursor: Option<String>,
    },
    CollabMembership {
        membership: Box<RoomMembership>,
    },
    CollabMembershipList {
        memberships: Vec<RoomMembership>,
    },
    CollabThread {
        thread: Box<ThreadRef>,
        resolution: ReferenceResolution,
    },
    CollabThreadList {
        threads: Vec<ThreadRef>,
        resolutions: Vec<ReferenceResolution>,
        next_cursor: Option<String>,
    },
    CollabMessage {
        message: Box<Message>,
    },
    CollabMessageList {
        messages: Vec<Message>,
    },
    CollabMessageEdit {
        edit: Box<MessageEdit>,
    },
    CollabTask {
        task: Box<Task>,
    },
    CollabTaskList {
        tasks: Vec<Task>,
        next_cursor: Option<String>,
    },
    CollabNote {
        note: Box<NoteDocument>,
    },
    CollabNoteRevisionList {
        revisions: Vec<NoteRevision>,
    },
    CollabNoteEdit {
        note: Box<NoteDocument>,
        new_revision: Box<NoteRevision>,
        is_conflict_copy: bool,
    },
    CollabApprovalRequest {
        request: Box<ApprovalRequest>,
    },
    CollabApprovalDecision {
        decision: Box<ApprovalDecision>,
    },
    CollabApprovalDecisionList {
        decisions: Vec<ApprovalDecision>,
    },
    CollabActivityList {
        records: Vec<ActivityRecord>,
    },
    // Spec 077: MedAgent Workbench typed results. T077-03 slice only.
    MedAgentIdentity {
        identity: Box<AgentIdentity>,
        capabilities: Box<AgentCapabilityManifest>,
    },
    MedAgentIdentityList {
        identities: Vec<AgentIdentity>,
    },
    MedAgentContextManifest {
        manifest: Box<ContextManifest>,
        /// Live-recomputed, never cached (`migration.md` section 4); same
        /// order as `manifest.selected_artifacts`.
        resolutions: Vec<ReferenceResolution>,
    },
    MedAgentRun {
        run: Box<AgentRun>,
    },
    MedAgentRunList {
        runs: Vec<AgentRun>,
    },
    /// `start_agent_run`'s result: the now-`Running` run plus its
    /// auto-appended initial `PromptSubmitted` turn.
    MedAgentRunStarted {
        run: Box<AgentRun>,
        turn: Box<AgentTurn>,
    },
    /// A terminal transition's result: the run plus its committed
    /// `RunReceipt`.
    MedAgentRunTerminal {
        run: Box<AgentRun>,
        receipt: Box<RunReceipt>,
    },
    MedAgentTurnList {
        turns: Vec<AgentTurn>,
    },
    /// `receipt` is `None` for a `Refused` invocation, `Some` for
    /// `Executed`.
    MedAgentToolInvocation {
        invocation: Box<ToolInvocation>,
        receipt: Option<Box<ToolReceipt>>,
    },
    MedAgentRunExecuted {
        turn: Box<AgentTurn>,
        proposal: Box<AgentProposal>,
    },
    // Spec 078: Model Fleet + Compare typed results (T078-03 slice).
    ModelFleetLane {
        lane: Box<AgentLane>,
    },
    ModelFleetLaneList {
        lanes: Vec<AgentLane>,
    },
    ModelFleetRun {
        run: Box<FleetRun>,
        lane_run_refs: Vec<LaneRunRef>,
    },
    ModelFleetRunList {
        runs: Vec<FleetRun>,
    },
    ModelFleetLaneExecuted {
        run: Box<FleetRun>,
        lane_run: Box<AgentRun>,
        proposal: Option<Box<AgentProposal>>,
    },
    ModelFleetComparisonReport {
        report: Box<ComparisonReport>,
    },
    ModelFleetComparisonReportList {
        reports: Vec<ComparisonReport>,
    },
    // Spec 079: Privacy Gate typed results.
    PrivacyClassification {
        classification: Box<ArtifactClassification>,
    },
    PrivacyEffectiveClassification {
        effective: Box<EffectiveClassification>,
    },
    PrivacyClassificationList {
        classifications: Vec<ArtifactClassification>,
    },
    PrivacyProfile {
        profile: Box<PrivacyPolicyProfile>,
    },
    PrivacyProfileList {
        profiles: Vec<PrivacyPolicyProfile>,
    },
    PrivacyMap {
        map: Box<PseudonymMapRef>,
    },
    PrivacyMapList {
        maps: Vec<PseudonymMapRef>,
    },
    PrivacyReceipt {
        receipt: Box<DeidReceipt>,
    },
    PrivacyReceiptList {
        receipts: Vec<DeidReceipt>,
    },
    /// `value` is present only when `audit.outcome` is `returned`.
    PrivacyReidentified {
        audit: Box<ReidentificationAudit>,
        value: Option<String>,
    },
    PrivacyReidentificationAuditList {
        audits: Vec<ReidentificationAudit>,
    },
    PrivacyEgressDecision {
        decision: Box<EgressDecision>,
    },
    PrivacyEgressDecisionList {
        decisions: Vec<EgressDecision>,
    },
    // Spec 080: Governed Browse typed results.
    BrowseAllowlistEntry {
        entry: Box<BrowseAllowlistEntry>,
    },
    BrowseAllowlist {
        entries: Vec<BrowseAllowlistEntry>,
    },
    BrowseRoutes {
        routes: Vec<BrowseRouteStatus>,
    },
    BrowseSession {
        view: Box<BrowseSessionView>,
    },
    BrowseSessionList {
        sessions: Vec<BrowseSession>,
    },
    // Spec 081: AudioFlow typed results.
    AudioSource {
        source: Box<AudioSource>,
    },
    AudioSources {
        sources: Vec<AudioSource>,
    },
    AudioCapture {
        session: Box<AudioSession>,
    },
    AudioCaptureStopped {
        session: Box<AudioSession>,
        source: Box<AudioSource>,
    },
    AudioCaptures {
        sessions: Vec<AudioSession>,
    },
    AudioRoutes {
        routes: Vec<AudioRouteStatus>,
    },
    AudioTranscript {
        view: Box<TranscriptView>,
    },
    AudioTranscriptRevision {
        revision: Box<TranscriptRevision>,
    },
    AudioTranscripts {
        revisions: Vec<TranscriptRevision>,
    },
    AudioReceipts {
        receipts: Vec<TranscriptReceipt>,
    },
    AudioEvidence {
        evidence: AudioEvidenceRef,
    },
    // Spec 082: Analytics typed results.
    AnalyticsQuery {
        view: Box<QueryView>,
    },
    AnalyticsReplay {
        report: ReplayReport,
    },
    AnalyticsReceipt {
        receipt: Box<QueryReceipt>,
    },
    AnalyticsReceipts {
        receipts: Vec<QueryReceipt>,
    },
    AnalyticsResult {
        table: Box<DerivedTable>,
        doc: Box<ResultTableDoc>,
    },
    AnalyticsStatistics {
        results: Vec<StatisticResult>,
    },
    AnalyticsCohort {
        cohort: Box<CohortDefinition>,
    },
    AnalyticsCohorts {
        cohorts: Vec<CohortDefinition>,
    },
    // Spec 083: Knowledge + Research Canvas typed results.
    KnowledgeIndexBuilt {
        manifest: Box<IndexManifest>,
        status: IndexStatus,
    },
    KnowledgeIndexStatus {
        status: Option<IndexStatus>,
    },
    KnowledgeSearch {
        result: Box<RetrievalResult>,
    },
    KnowledgeReceipt {
        receipt: Box<RetrievalReceipt>,
    },
    KnowledgeReceipts {
        receipts: Vec<RetrievalReceipt>,
    },
    Canvas {
        canvas: Box<CanvasRevision>,
    },
    CanvasView {
        view: Box<CanvasView>,
    },
    Canvases {
        canvases: Vec<CanvasSummary>,
    },
    // Spec 084: MedScale Hub foundation.
    HubIdentity {
        hub: Box<HubIdentity>,
    },
    HubInvited {
        invitation: Box<HubInvitation>,
        code: Box<HubInvitationCode>,
    },
    HubInvitation {
        invitation: Box<HubInvitation>,
    },
    HubDevice {
        device: Box<DeviceIdentity>,
    },
    HubStatus {
        status: Box<HubStatus>,
    },
    HubChallenge {
        challenge: Box<HubChallenge>,
    },
    HubSession {
        session: Box<HubSession>,
    },
    HubOutcomes {
        outcomes: Vec<SyncOutcome>,
    },
    HubEvents {
        page: Box<HubEventPage>,
    },
    HubJoinPrepared {
        link_id: OpaqueId,
        public_key_hex: String,
        signature_hex: String,
    },
    HubLink {
        link: Box<HubLink>,
    },
    HubLinks {
        links: Vec<HubLink>,
    },
    HubQueued {
        entry: Box<HubOutboxEntry>,
    },
    HubHandshakeSigned {
        handshake: Box<HubHandshake>,
    },
    HubOutbox {
        entries: Vec<HubOutboxEntry>,
    },
    HubMirror {
        events: Vec<HubEvent>,
    },
    HubRecorded,
    // Spec 085: MedScale Compute.
    ComputeJob {
        view: Box<ComputeJobView>,
    },
    ComputeJobs {
        jobs: Vec<ComputeJob>,
    },
    ComputeRecovered {
        views: Vec<ComputeJobView>,
    },
    ComputeStatus {
        status: ComputeStatus,
    },
    // Spec 086: R Workspace.
    RWorkspace {
        workspace: Box<RWorkspace>,
    },
    RWorkspaceInspection {
        inspection: WorkspaceInspection,
    },
    RLaunch {
        receipt: Box<RLaunchReceipt>,
    },
    RRun {
        receipt: Box<RRunReceipt>,
    },
    RPublish {
        view: Box<RPublishView>,
    },
    RWorkspaceHistory {
        history: Box<RWorkspaceHistory>,
    },
    RWorkspaces {
        workspaces: Vec<RWorkspace>,
    },
    RWorkspaceStatus {
        status: RWorkspaceStatus,
    },
    // Spec 087: Community Extensions.
    ExtensionPublisher {
        publisher: Box<ExtensionPublisher>,
    },
    ExtensionLifecycle {
        receipt: Box<ExtensionLifecycleReceipt>,
    },
    ExtensionLifecycles {
        receipts: Vec<ExtensionLifecycleReceipt>,
    },
    ExtensionInvocation {
        outcome: Box<crate::extensions::ExtensionInvocation>,
    },
    ExtensionProject {
        view: Box<crate::extensions::ExtensionProjectView>,
    },
    // Spec 088: AudioFlow Advanced huddles.
    HuddleActed {
        result: Box<crate::huddles::HuddleActResult>,
    },
    Huddle {
        view: Box<crate::huddles::HuddleView>,
    },
    Huddles {
        huddles: Vec<crate::huddles::Huddle>,
    },
    // Spec 089: Research Packs.
    PackActed {
        result: Box<crate::research_packs::PackActResult>,
    },
    PackCatalog {
        packs: Vec<crate::research_packs::ResearchPackManifest>,
    },
    PackInstall {
        install: Box<crate::research_packs::ResearchPackInstall>,
    },
    PackArtifact {
        artifact: Box<crate::research_packs::ResearchArtifact>,
    },
    PackArtifacts {
        artifacts: Vec<crate::research_packs::ResearchArtifact>,
    },
    // Spec 090: institutional adapters.
    AdapterActed {
        result: Box<crate::institutional::AdapterActResult>,
    },
    Adapter {
        view: Box<crate::institutional::AdapterView>,
    },
    Adapters {
        adapters: Vec<crate::institutional::InstitutionalAdapter>,
    },
    // Spec 091: federation.
    FederationActed {
        result: Box<crate::federation::FederationActResult>,
    },
    Federation {
        view: Box<crate::federation::FederationView>,
    },
    // Spec 103: one page of the local model catalog.
    ModelCatalogPage {
        page: Box<crate::model_catalog::ModelCatalogPage>,
    },
}

/// Authority error vocabulary (fail closed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum AuthorityError {
    Unauthorized,
    AlreadyHeld {
        holder_id: OpaqueId,
    },
    NotHolder,
    NotHeld,
    NotFound,
    WrongScope,
    IllegalTransition,
    UnknownRequiresReconcile,
    DigestMismatch,
    LeaseRequired,
    VaultRequired,
    LexicalReject {
        reason: String,
    },
    VersionReject {
        got: Option<String>,
    },
    PathOutsideClaim,
    InvalidArgument {
        message: String,
    },
    MissingKeyMaterial,
    LeaseHeld {
        holder_id: OpaqueId,
    },
    BrokerDenied {
        reason: crate::network::BrokerReasonCode,
    },
    ExternalGateRequired {
        gate: String,
    },
    PackDenied {
        reason: crate::packs::PackAdmitReason,
    },
    SessionRequired,
    SessionExpired,
    SessionRevoked,
    SessionDenied,
    /// Optimistic-concurrency conflict: stale `expected_revision` or duplicate.
    /// No write was performed.
    Conflict {
        message: String,
    },
    /// A pinned attachment binding no longer matches its canonical target.
    StaleReference {
        message: String,
    },
    /// Durable Project Graph bytes failed integrity validation.
    Corrupt {
        message: String,
    },
    /// Unsupported durable schema or unknown future authority value (fail closed).
    UnsupportedSchema {
        message: String,
    },
    /// Dependency or target temporarily unresolvable.
    Unavailable {
        message: String,
    },
    /// Caller-requested cancellation completed before commit. No write was
    /// performed. Never a substitute for timeout/unavailable mapping.
    Cancelled {
        message: String,
    },
    /// Unexpected internal failure (never a substitute for a typed variant).
    Internal {
        message: String,
    },
}

/// Versioned authority request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthorityRequest {
    pub schema_version: u32,
    pub request_id: OpaqueId,
    pub vault_id: VaultId,
    pub authority_scope_id: crate::objects::AuthorityScopeId,
    pub realm_id: crate::objects::RealmId,
    pub capability: Capability,
    pub deadline_ms: u64,
    pub max_response_bytes: u64,
    /// Client session (Spec 024 Strict: required for mutating capabilities).
    /// Absent is only valid for bootstrap/read/health, or LegacyLeaseOnlyEngineering.
    #[serde(default)]
    pub session_id: Option<OpaqueId>,
    pub body: RequestBody,
}

impl AuthorityRequest {
    /// Builds a request at the current schema version.
    #[must_use]
    pub fn new(
        request_id: OpaqueId,
        vault_id: VaultId,
        realm_id: crate::objects::RealmId,
        authority_scope_id: crate::objects::AuthorityScopeId,
        capability: Capability,
        body: RequestBody,
    ) -> Self {
        Self {
            schema_version: AUTHORITY_SCHEMA_VERSION,
            request_id,
            vault_id,
            authority_scope_id,
            realm_id,
            capability,
            deadline_ms: 5_000,
            max_response_bytes: 1_048_576,
            session_id: None,
            body,
        }
    }
}

/// Versioned authority response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthorityResponse {
    pub schema_version: u32,
    pub request_id: OpaqueId,
    pub result: Result<ResponseBody, AuthorityError>,
}
