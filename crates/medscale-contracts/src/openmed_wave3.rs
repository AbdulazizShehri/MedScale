//! OpenMed v4.0.0 Wave 3 authority contracts (multimodal/DICOM, healthcare
//! interop, clinical agent governance kernel, MCP SSRF/consent, DUA eval).

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveMemberDto {
    pub path: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub link_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultimodalV4ProcessRequest {
    pub format: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub base64_payload: Option<String>,
    #[serde(default)]
    pub archive_members: Vec<ArchiveMemberDto>,
    #[serde(default = "default_true")]
    pub scrub: bool,
    #[serde(default)]
    pub policy_name: Option<String>,
}

impl Default for MultimodalV4ProcessRequest {
    fn default() -> Self {
        Self {
            format: String::new(),
            text: None,
            base64_payload: None,
            archive_members: Vec::new(),
            scrub: true,
            policy_name: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultimodalV4ProcessResponse {
    pub format: String,
    pub status: String,
    pub extracted_text: String,
    pub scrubbed_text: String,
    pub output_base64: Option<String>,
    pub output_sha256: String,
    pub removed_segments: Vec<String>,
    pub residual_findings: Vec<Value>,
    pub residual_clean: bool,
    pub archive_report: Option<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlackoutRectDto {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DicomV4DeidentifyRequest {
    pub base64_payload: String,
    #[serde(default)]
    pub uid_hmac_secret: Option<String>,
    #[serde(default)]
    pub date_shift_days: i64,
    #[serde(default)]
    pub blackout_rects: Vec<BlackoutRectDto>,
    #[serde(default)]
    pub policy_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DicomV4DeidentifyResponse {
    pub transfer_syntax_uid: String,
    pub explicit_vr_dataset: bool,
    pub tags_scrubbed: Vec<String>,
    pub uids_remapped: usize,
    pub sr_items_scrubbed: usize,
    pub blackout_regions_applied: usize,
    pub residual_clean: bool,
    pub input_sha256: String,
    pub output_sha256: String,
    pub output_base64: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InteropV4ProcessRequest {
    pub operation: String,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub raw_text: Option<String>,
    #[serde(default)]
    pub date_shift_days: i64,
    #[serde(default)]
    pub reference_year: Option<i32>,
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub required_scopes: Vec<String>,
    #[serde(default)]
    pub declared_scopes: Vec<String>,
    #[serde(default)]
    pub bridge_name: Option<String>,
    #[serde(default)]
    pub policy_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteropV4ProcessResponse {
    pub operation: String,
    pub status: String,
    pub result: Value,
    pub warnings: Vec<String>,
    pub redaction_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentV4GovernRequest {
    pub operation: String,
    #[serde(default)]
    pub nodes: Vec<Value>,
    #[serde(default)]
    pub current_phase: Option<String>,
    #[serde(default)]
    pub target_phase: Option<String>,
    #[serde(default)]
    pub reviewed: bool,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default = "default_true")]
    pub strict: bool,
    #[serde(default)]
    pub action_digest: Option<String>,
    #[serde(default)]
    pub reviewer_role: Option<String>,
    #[serde(default)]
    pub nonce: Option<String>,
    #[serde(default)]
    pub token: Option<Value>,
    #[serde(default)]
    pub requirements: Vec<Value>,
    #[serde(default)]
    pub evidence: Vec<Value>,
    #[serde(default)]
    pub trial_id: Option<String>,
    #[serde(default)]
    pub criteria: Vec<Value>,
}

impl Default for AgentV4GovernRequest {
    fn default() -> Self {
        Self {
            operation: String::new(),
            nodes: Vec::new(),
            current_phase: None,
            target_phase: None,
            reviewed: false,
            text: None,
            strict: true,
            action_digest: None,
            reviewer_role: None,
            nonce: None,
            token: None,
            requirements: Vec::new(),
            evidence: Vec::new(),
            trial_id: None,
            criteria: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentV4GovernResponse {
    pub operation: String,
    pub allowed: bool,
    pub result: Value,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McpV4PolicyCheckRequest {
    pub operation: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub resolved_ips: Vec<String>,
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    #[serde(default)]
    pub allow_loopback: bool,
    #[serde(default)]
    pub client: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub arguments: Value,
    #[serde(default)]
    pub receipt: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpV4PolicyCheckResponse {
    pub operation: String,
    pub allowed: bool,
    pub result: Value,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EvalV4RunRequest {
    pub operation: String,
    #[serde(default)]
    pub gold_spans: Vec<Value>,
    #[serde(default)]
    pub pred_spans: Vec<Value>,
    #[serde(default)]
    pub gold_relations: Vec<Value>,
    #[serde(default)]
    pub pred_relations: Vec<Value>,
    #[serde(default)]
    pub calibration_points: Vec<Value>,
    #[serde(default)]
    pub n_bins: Option<usize>,
    #[serde(default)]
    pub train_texts: Vec<String>,
    #[serde(default)]
    pub eval_texts: Vec<String>,
    #[serde(default)]
    pub dataset_name: Option<String>,
    #[serde(default)]
    pub dataset_path: Option<String>,
    #[serde(default)]
    pub network_fetch: bool,
    #[serde(default)]
    pub terminology_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalV4RunResponse {
    pub operation: String,
    pub passed: bool,
    pub result: Value,
}

const fn default_true() -> bool {
    true
}
