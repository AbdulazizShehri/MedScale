//! OpenMed v4.0.0 Wave 3 clinical agent governance kernel, MCP security/SSRF
//! policy, and DUA-guarded evaluation harness (zero new external crates).

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::net::IpAddr;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const MAX_ACTION_GRAPH_NODES: usize = 10_000;
pub const MAX_ACTION_DEPENDENCIES: usize = 64;
pub const MAX_APPROVAL_LIFETIME_SECS: i64 = 86_400;
pub const MAX_ENDPOINT_LEN: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEvalErrorKind {
    Validation,
    Unauthorized,
    Replay,
    SsrfBlocked,
    DuaViolation,
    BlockedExternalRights,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{kind:?}: {message}")]
pub struct AgentEvalError {
    pub kind: AgentEvalErrorKind,
    pub message: String,
}

impl AgentEvalError {
    #[must_use]
    pub fn new(kind: AgentEvalErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}

fn sha256_prefixed(data: &[u8]) -> String {
    format!("sha256:{}", hex_lower(&Sha256::digest(data)))
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut block = [0u8; 64];
    if key.len() > 64 {
        let d = Sha256::digest(key);
        block[..32].copy_from_slice(&d);
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5Cu8; 64];
    for i in 0..64 {
        ipad[i] ^= block[i];
        opad[i] ^= block[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(msg);
    let id = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(id);
    let od = outer.finalize();
    let mut res = [0u8; 32];
    res.copy_from_slice(&od);
    res
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/* ───────────────────────── ActionGraph DAG Validator ───────────────────────── */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionNodeInput {
    pub action_id: String,
    pub tool_id: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionGraphFinding {
    pub reason_code: String,
    pub action_id: Option<String>,
    pub dependency_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionGraphReport {
    pub valid: bool,
    pub node_count: usize,
    pub edge_count: usize,
    pub order: Vec<String>,
    pub findings: Vec<ActionGraphFinding>,
}

fn valid_action_ident(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
        && s.as_bytes()[0].is_ascii_alphanumeric()
}

fn valid_tool_id(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("tool:") else {
        return false;
    };
    let Some((ns_name, ver)) = rest.split_once('@') else {
        return false;
    };
    let Some((ns, name)) = ns_name.split_once('/') else {
        return false;
    };
    ns.contains('.') && !name.is_empty() && ver.split('.').count() == 3
}

pub fn validate_action_graph(
    nodes: &[ActionNodeInput],
) -> Result<ActionGraphReport, AgentEvalError> {
    if nodes.len() > MAX_ACTION_GRAPH_NODES {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::Validation,
            "action graph exceeds 10,000 node bound",
        ));
    }
    let mut findings = Vec::new();
    let mut seen_ids = BTreeSet::new();
    let mut node_map: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for n in nodes {
        if !valid_action_ident(&n.action_id) || !valid_tool_id(&n.tool_id) {
            return Err(AgentEvalError::new(
                AgentEvalErrorKind::Validation,
                "invalid action_id or canonical tool_id",
            ));
        }
        if n.depends_on.len() > MAX_ACTION_DEPENDENCIES {
            return Err(AgentEvalError::new(
                AgentEvalErrorKind::Validation,
                "node exceeds 64 dependencies",
            ));
        }
        if !seen_ids.insert(n.action_id.clone()) {
            findings.push(ActionGraphFinding {
                reason_code: "duplicate_action_id".to_owned(),
                action_id: Some(n.action_id.clone()),
                dependency_id: None,
            });
            continue;
        }
        let mut seen_deps = BTreeSet::new();
        let mut clean_deps = Vec::new();
        for d in &n.depends_on {
            if !valid_action_ident(d) {
                return Err(AgentEvalError::new(
                    AgentEvalErrorKind::Validation,
                    "invalid dependency identifier",
                ));
            }
            if d == &n.action_id {
                findings.push(ActionGraphFinding {
                    reason_code: "self_dependency".to_owned(),
                    action_id: Some(n.action_id.clone()),
                    dependency_id: Some(d.clone()),
                });
            } else if !seen_deps.insert(d.clone()) {
                findings.push(ActionGraphFinding {
                    reason_code: "duplicate_dependency".to_owned(),
                    action_id: Some(n.action_id.clone()),
                    dependency_id: Some(d.clone()),
                });
            } else {
                clean_deps.push(d.clone());
            }
        }
        node_map.insert(n.action_id.clone(), clean_deps);
    }

    let mut in_degree: BTreeMap<String, usize> =
        node_map.keys().map(|k| (k.clone(), 0usize)).collect();
    let mut adj: BTreeMap<String, Vec<String>> =
        node_map.keys().map(|k| (k.clone(), Vec::new())).collect();
    let mut edge_count = 0usize;

    for (id, deps) in &node_map {
        for dep in deps {
            if node_map.contains_key(dep) {
                *in_degree.get_mut(id).expect("node key exists") += 1;
                adj.get_mut(dep).expect("dep key exists").push(id.clone());
                edge_count += 1;
            } else {
                findings.push(ActionGraphFinding {
                    reason_code: "missing_dependency".to_owned(),
                    action_id: Some(id.clone()),
                    dependency_id: Some(dep.clone()),
                });
            }
        }
    }

    let mut heap: BinaryHeap<Reverse<String>> = BinaryHeap::new();
    for (k, &deg) in &in_degree {
        if deg == 0 {
            heap.push(Reverse(k.clone()));
        }
    }

    let mut order = Vec::with_capacity(node_map.len());
    while let Some(Reverse(cur)) = heap.pop() {
        order.push(cur.clone());
        if let Some(children) = adj.get(&cur) {
            for ch in children {
                let d = in_degree.get_mut(ch).expect("child exists");
                *d = d.saturating_sub(1);
                if *d == 0 {
                    heap.push(Reverse(ch.clone()));
                }
            }
        }
    }

    if order.len() < node_map.len() {
        for (k, &deg) in &in_degree {
            if deg > 0 {
                findings.push(ActionGraphFinding {
                    reason_code: "dependency_cycle".to_owned(),
                    action_id: Some(k.clone()),
                    dependency_id: None,
                });
            }
        }
    }

    Ok(ActionGraphReport {
        valid: findings.is_empty(),
        node_count: node_map.len(),
        edge_count,
        order,
        findings,
    })
}

/* ───────────────────────── ActionPhase FSM & Single-Use ApprovalToken ───────────────────────── */

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionPhase {
    Queued,
    Preflight,
    Ready,
    Running,
    WaitingReview,
    Completed,
    Aborted,
}

impl ActionPhase {
    pub fn parse(raw: &str) -> Result<Self, AgentEvalError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "queued" => Ok(Self::Queued),
            "preflight" => Ok(Self::Preflight),
            "ready" => Ok(Self::Ready),
            "running" => Ok(Self::Running),
            "waiting-review" | "waiting_review" => Ok(Self::WaitingReview),
            "completed" => Ok(Self::Completed),
            "aborted" => Ok(Self::Aborted),
            _ => Err(AgentEvalError::new(
                AgentEvalErrorKind::Validation,
                format!("unknown action phase `{raw}`"),
            )),
        }
    }
}

pub fn validate_action_transition(
    current: ActionPhase,
    target: ActionPhase,
    reviewed: bool,
) -> Result<(), AgentEvalError> {
    let allowed = match (current, target) {
        (ActionPhase::Queued, ActionPhase::Preflight | ActionPhase::Aborted) => true,
        (
            ActionPhase::Preflight,
            ActionPhase::Ready | ActionPhase::WaitingReview | ActionPhase::Aborted,
        ) => true,
        (ActionPhase::Ready, ActionPhase::Running | ActionPhase::Aborted) => true,
        (
            ActionPhase::Running,
            ActionPhase::WaitingReview | ActionPhase::Completed | ActionPhase::Aborted,
        ) => true,
        (ActionPhase::WaitingReview, ActionPhase::Preflight) => {
            if !reviewed {
                return Err(AgentEvalError::new(
                    AgentEvalErrorKind::Unauthorized,
                    "review_required: waiting-review -> preflight requires reviewed=true",
                ));
            }
            true
        }
        (ActionPhase::WaitingReview, ActionPhase::Aborted) => true,
        _ => false,
    };
    if !allowed {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::Validation,
            format!("invalid action phase transition {current:?} -> {target:?}"),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalToken {
    pub schema_version: String,
    pub key_id: String,
    pub action_digest: String,
    pub reviewer_role: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub nonce: String,
    pub signature: String,
}

fn approval_signing_payload(
    key_id: &str,
    action_digest: &str,
    reviewer_role: &str,
    issued_at: i64,
    expires_at: i64,
    nonce: &str,
) -> String {
    format!("{key_id}|{action_digest}|{reviewer_role}|{issued_at}|{expires_at}|{nonce}")
}

pub fn issue_approval_token(
    secret: &[u8],
    key_id: &str,
    action_digest: &str,
    reviewer_role: &str,
    issued_at: i64,
    expires_at: i64,
    nonce: &str,
) -> Result<ApprovalToken, AgentEvalError> {
    if !action_digest.starts_with("sha256:") || action_digest.len() != 71 {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::Validation,
            "action_digest must be sha256:<64 hex>",
        ));
    }
    if expires_at <= issued_at || (expires_at - issued_at) > MAX_APPROVAL_LIFETIME_SECS {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::Validation,
            "invalid approval token lifetime",
        ));
    }
    let payload = approval_signing_payload(
        key_id,
        action_digest,
        reviewer_role,
        issued_at,
        expires_at,
        nonce,
    );
    let sig = format!(
        "hmac-sha256:{}",
        hex_lower(&hmac_sha256(secret, payload.as_bytes()))
    );
    Ok(ApprovalToken {
        schema_version: "medscale_clinical.agent.approval_token.v2".to_owned(),
        key_id: key_id.to_owned(),
        action_digest: action_digest.to_owned(),
        reviewer_role: reviewer_role.to_owned(),
        issued_at,
        expires_at,
        nonce: nonce.to_owned(),
        signature: sig,
    })
}

pub fn verify_and_consume_approval_token(
    secret: &[u8],
    token: &ApprovalToken,
    expected_action_digest: &str,
    expected_reviewer_role: &str,
    now_epoch_s: i64,
    consumed_nonces: &mut BTreeSet<String>,
) -> Result<(), AgentEvalError> {
    if now_epoch_s < token.issued_at || now_epoch_s >= token.expires_at {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::Unauthorized,
            "approval token expired or not yet valid",
        ));
    }
    let payload = approval_signing_payload(
        &token.key_id,
        &token.action_digest,
        &token.reviewer_role,
        token.issued_at,
        token.expires_at,
        &token.nonce,
    );
    let expected_sig = format!(
        "hmac-sha256:{}",
        hex_lower(&hmac_sha256(secret, payload.as_bytes()))
    );
    if !ct_eq(token.signature.as_bytes(), expected_sig.as_bytes()) {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::Unauthorized,
            "invalid approval token signature",
        ));
    }
    if token.action_digest != expected_action_digest
        || token.reviewer_role != expected_reviewer_role
    {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::Unauthorized,
            "approval token action_digest or reviewer_role mismatch",
        ));
    }
    if !consumed_nonces.insert(token.nonce.clone()) {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::Replay,
            "approval token nonce replay detected",
        ));
    }
    Ok(())
}

/* ───────────────────────── InjectionGuard & Clinician Draft Workflows ───────────────────────── */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InjectionFinding {
    pub pattern_id: String,
    pub severity: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InjectionScanResult {
    pub blocked: bool,
    pub findings: Vec<InjectionFinding>,
    pub quarantined_text: String,
}

#[must_use]
pub fn scan_prompt_injection(text: &str, strict: bool) -> InjectionScanResult {
    let lower = text.to_ascii_lowercase();
    let checks: &[(&str, &str, &[&str])] = &[
        (
            "instruction_override",
            "high",
            &[
                "ignore previous instructions",
                "ignore all previous instructions",
                "disregard previous instructions",
                "bypass system instructions",
                "new system instructions",
                "act as unrestricted",
            ],
        ),
        (
            "tool_name_spoofing",
            "high",
            &[
                "<tool_call",
                "<function_call",
                "invoke mcp tool",
                "call mcp tool",
                "execute mcp",
            ],
        ),
        (
            "delimiter_breakout",
            "high",
            &[
                "### system:",
                "<system>",
                "[/inst]",
                "<<sys>>",
                "developer:",
            ],
        ),
        (
            "data_exfiltration",
            "critical",
            &[
                "exfiltrate",
                "export vault to http",
                "upload patient data to",
            ],
        ),
    ];

    let mut findings = Vec::new();
    let mut pattern_ids = BTreeSet::new();
    for &(pid, sev, cues) in checks {
        for cue in cues {
            if let Some(pos) = lower.find(cue) {
                findings.push(InjectionFinding {
                    pattern_id: pid.to_owned(),
                    severity: sev.to_owned(),
                    start: pos,
                    end: pos + cue.len(),
                });
                pattern_ids.insert(pid.to_owned());
                break;
            }
        }
    }

    let blocked = strict && !findings.is_empty();
    let quarantined_text = if findings.is_empty() {
        text.to_owned()
    } else {
        let joined: Vec<String> = pattern_ids.into_iter().collect();
        format!(
            "[MEDSCALE_QUARANTINED_PROMPT_INJECTION:{}]",
            joined.join(",")
        )
    };

    InjectionScanResult {
        blocked,
        findings,
        quarantined_text,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriorAuthRequirementSpec {
    pub requirement_id: String,
    pub citation_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorAuthEvidenceItem {
    pub requirement_id: String,
    pub evidence_digest: String,
    pub citation_count: usize,
    pub contradiction_flag: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriorAuthCompletenessDraft {
    pub schema_version: String,
    pub complete_count: usize,
    pub required_count: usize,
    pub completeness_score: f64,
    pub missing_requirements: Vec<String>,
    pub clinician_review_required: bool,
    pub autonomous_decision: bool,
}

#[must_use]
pub fn score_prior_auth_completeness(
    requirements: &[PriorAuthRequirementSpec],
    evidence: &[PriorAuthEvidenceItem],
) -> PriorAuthCompletenessDraft {
    let mut complete_count = 0usize;
    let mut missing = Vec::new();
    for req in requirements {
        let matched = evidence
            .iter()
            .find(|e| e.requirement_id == req.requirement_id);
        let ok = matched.is_some_and(|m| {
            !m.contradiction_flag
                && m.evidence_digest.starts_with("sha256:")
                && (!req.citation_required || m.citation_count > 0)
        });
        if ok {
            complete_count += 1;
        } else {
            missing.push(req.requirement_id.clone());
        }
    }
    let required_count = requirements.len();
    let completeness_score = if required_count == 0 {
        1.0
    } else {
        (complete_count as f64) / (required_count as f64)
    };
    PriorAuthCompletenessDraft {
        schema_version: "medscale_clinical.agent.workflows.prior_auth_completeness.v1".to_owned(),
        complete_count,
        required_count,
        completeness_score,
        missing_requirements: missing,
        clinician_review_required: true,
        autonomous_decision: false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialCriterionEval {
    pub criterion_id: String,
    pub rule_state: String,
    pub model_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialEligibilityDraft {
    pub trial_id: String,
    pub mismatches: Vec<String>,
    pub clinician_review_required: bool,
    pub authorizes_enrollment: bool,
}

#[must_use]
pub fn build_trial_eligibility_review(
    trial_id: &str,
    criteria: &[TrialCriterionEval],
) -> TrialEligibilityDraft {
    let mismatches = criteria
        .iter()
        .filter(|c| c.rule_state != c.model_state || c.rule_state == "unknown")
        .map(|c| c.criterion_id.clone())
        .collect();
    TrialEligibilityDraft {
        trial_id: trial_id.to_owned(),
        mismatches,
        clinician_review_required: true,
        authorizes_enrollment: false,
    }
}

/* ───────────────────────── MCP Registry, ConsentReceipt & SSRF Policy ───────────────────────── */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolDescriptor {
    pub name: String,
    pub description: String,
    pub read_only: bool,
    pub requires_consent_receipt: bool,
}

#[must_use]
pub fn default_mcp_tool_registry() -> Vec<McpToolDescriptor> {
    vec![
        McpToolDescriptor {
            name: "medscale.privacy.deidentify".to_owned(),
            description: "De-identify synthetic text with policy presets & compliance receipt"
                .to_owned(),
            read_only: true,
            requires_consent_receipt: false,
        },
        McpToolDescriptor {
            name: "medscale.clinical.analyze".to_owned(),
            description: "Deterministic LOINC section, ConText, Sig & UCUM analysis".to_owned(),
            read_only: true,
            requires_consent_receipt: false,
        },
        McpToolDescriptor {
            name: "medscale.multimodal.inspect".to_owned(),
            description: "Pure-Rust archive/PDF/OOXML/media metadata scrub & verification"
                .to_owned(),
            read_only: true,
            requires_consent_receipt: false,
        },
        McpToolDescriptor {
            name: "medscale.interop.transform".to_owned(),
            description: "FHIR $de-identify, SMART audit, HL7v2/C-CDA/X12/OMOP transformation"
                .to_owned(),
            read_only: true,
            requires_consent_receipt: true,
        },
        McpToolDescriptor {
            name: "medscale.eval.score".to_owned(),
            description: "NER/relation F1, ECE & DUA-guarded evaluation harness".to_owned(),
            read_only: true,
            requires_consent_receipt: true,
        },
    ]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpConsentReceipt {
    pub receipt_id: String,
    pub client: String,
    pub tool: String,
    pub resource: String,
    pub scope: String,
    pub argument_digest: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub key_id: String,
    pub signature: String,
}

#[allow(clippy::too_many_arguments)]
pub fn issue_consent_receipt(
    secret: &[u8],
    key_id: &str,
    receipt_id: &str,
    client: &str,
    tool: &str,
    resource: &str,
    scope: &str,
    arguments_json: &Value,
    issued_at: i64,
    expires_at: i64,
) -> Result<McpConsentReceipt, AgentEvalError> {
    if expires_at <= issued_at {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::Validation,
            "consent receipt expires_at must be > issued_at",
        ));
    }
    let canon = serde_json::to_string(arguments_json).unwrap_or_default();
    let argument_digest = sha256_prefixed(canon.as_bytes());
    let signing_str = format!(
        "{receipt_id}|{client}|{tool}|{resource}|{scope}|{argument_digest}|{issued_at}|{expires_at}|{key_id}"
    );
    let signature = format!(
        "hmac-sha256:{}",
        hex_lower(&hmac_sha256(secret, signing_str.as_bytes()))
    );
    Ok(McpConsentReceipt {
        receipt_id: receipt_id.to_owned(),
        client: client.to_owned(),
        tool: tool.to_owned(),
        resource: resource.to_owned(),
        scope: scope.to_owned(),
        argument_digest,
        issued_at,
        expires_at,
        key_id: key_id.to_owned(),
        signature,
    })
}

pub fn verify_consent_receipt(
    secret: &[u8],
    receipt: &McpConsentReceipt,
    arguments_json: &Value,
    now_epoch_s: i64,
) -> Result<(), AgentEvalError> {
    if now_epoch_s < receipt.issued_at || now_epoch_s >= receipt.expires_at {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::Unauthorized,
            "consent receipt expired or not yet valid",
        ));
    }
    let canon = serde_json::to_string(arguments_json).unwrap_or_default();
    let arg_digest = sha256_prefixed(canon.as_bytes());
    if arg_digest != receipt.argument_digest {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::Unauthorized,
            "consent receipt argument_digest mismatch",
        ));
    }
    let signing_str = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        receipt.receipt_id,
        receipt.client,
        receipt.tool,
        receipt.resource,
        receipt.scope,
        receipt.argument_digest,
        receipt.issued_at,
        receipt.expires_at,
        receipt.key_id
    );
    let expected = format!(
        "hmac-sha256:{}",
        hex_lower(&hmac_sha256(secret, signing_str.as_bytes()))
    );
    if !ct_eq(receipt.signature.as_bytes(), expected.as_bytes()) {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::Unauthorized,
            "invalid consent receipt HMAC signature",
        ));
    }
    Ok(())
}

fn is_prohibited_non_loopback_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_private()
                || v4.is_link_local()
                || v4.is_multicast()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || (o[0] == 100 && (64..=127).contains(&o[1]))
                || o == [169, 254, 169, 254]
                || o == [169, 254, 170, 2]
        }
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_prohibited_non_loopback_ip(IpAddr::V4(mapped));
            }
            let seg0 = v6.segments()[0];
            v6.is_unspecified()
                || v6.is_multicast()
                || (seg0 & 0xFE00) == 0xFC00
                || (seg0 & 0xFFC0) == 0xFE80
        }
    }
}

pub fn validate_upstream_endpoint(
    endpoint: &str,
    resolved_ips: &[IpAddr],
    allowed_origins: &[String],
    allow_loopback: bool,
) -> Result<Value, AgentEvalError> {
    if endpoint.len() > MAX_ENDPOINT_LEN || endpoint.trim().is_empty() {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::SsrfBlocked,
            "endpoint_rejected: empty or exceeds 4096 chars",
        ));
    }
    let (scheme, rest) = endpoint
        .split_once("://")
        .ok_or_else(|| AgentEvalError::new(AgentEvalErrorKind::SsrfBlocked, "missing_scheme"))?;
    let scheme_lc = scheme.to_ascii_lowercase();
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.is_empty() || authority.contains('@') {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::SsrfBlocked,
            "userinfo_not_allowed or missing host",
        ));
    }
    let host = if authority.starts_with('[') {
        authority
            .split_once(']')
            .map(|(h, _)| h.trim_start_matches('['))
            .unwrap_or("")
    } else {
        authority.split(':').next().unwrap_or("")
    }
    .to_ascii_lowercase();

    let host_is_loopback = matches!(host.as_str(), "127.0.0.1" | "::1" | "localhost");
    if resolved_ips.is_empty() {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::SsrfBlocked,
            "no_resolved_addresses",
        ));
    }
    let all_loopback = resolved_ips.iter().all(IpAddr::is_loopback);
    if all_loopback || host_is_loopback {
        if !allow_loopback || !all_loopback {
            return Err(AgentEvalError::new(
                AgentEvalErrorKind::SsrfBlocked,
                "loopback_not_allowed or mixed addresses",
            ));
        }
        if !matches!(scheme_lc.as_str(), "http" | "https") {
            return Err(AgentEvalError::new(
                AgentEvalErrorKind::SsrfBlocked,
                "unsupported_scheme",
            ));
        }
        return Ok(json!({
            "allowed": true,
            "is_loopback": true,
            "origin": format!("{scheme_lc}://{authority}"),
        }));
    }

    if scheme_lc != "https" {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::SsrfBlocked,
            "non_https_origin: remote endpoints must use HTTPS",
        ));
    }
    for &ip in resolved_ips {
        if ip.is_loopback() || is_prohibited_non_loopback_ip(ip) {
            return Err(AgentEvalError::new(
                AgentEvalErrorKind::SsrfBlocked,
                "prohibited_ip_address: private, link-local, metadata, or non-global target blocked",
            ));
        }
    }
    let origin = format!("https://{authority}");
    if !allowed_origins.is_empty()
        && !allowed_origins
            .iter()
            .any(|o| o.eq_ignore_ascii_case(&origin))
    {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::SsrfBlocked,
            "origin_not_allowed",
        ));
    }
    Ok(json!({
        "allowed": true,
        "is_loopback": false,
        "origin": origin,
    }))
}

/* ───────────────────────── Eval Harnesses & DUA / Terminology Path Guards ───────────────────────── */

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvalSpanItem {
    pub start: usize,
    pub end: usize,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalF1Summary {
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub tp: usize,
    pub fp: usize,
    pub fn_count: usize,
}

fn compute_f1(tp: usize, fp: usize, fn_count: usize) -> EvalF1Summary {
    let p = if tp + fp == 0 {
        0.0
    } else {
        (tp as f64) / ((tp + fp) as f64)
    };
    let r = if tp + fn_count == 0 {
        0.0
    } else {
        (tp as f64) / ((tp + fn_count) as f64)
    };
    let f1 = if p + r == 0.0 {
        0.0
    } else {
        2.0 * p * r / (p + r)
    };
    EvalF1Summary {
        precision: p,
        recall: r,
        f1,
        tp,
        fp,
        fn_count,
    }
}

#[must_use]
pub fn evaluate_span_f1(
    gold: &[EvalSpanItem],
    pred: &[EvalSpanItem],
    relaxed: bool,
) -> EvalF1Summary {
    let mut matched_gold = vec![false; gold.len()];
    let mut tp = 0usize;
    let mut fp = 0usize;
    for p in pred {
        if let Some((gi, _)) = gold.iter().enumerate().find(|(idx, g)| {
            !matched_gold[*idx]
                && g.label == p.label
                && if relaxed {
                    p.start < g.end && g.start < p.end
                } else {
                    p.start == g.start && p.end == g.end
                }
        }) {
            matched_gold[gi] = true;
            tp += 1;
        } else {
            fp += 1;
        }
    }
    let fn_count = matched_gold.iter().filter(|&&m| !m).count();
    compute_f1(tp, fp, fn_count)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalRelationItem {
    pub relation_type: String,
    pub head: EvalSpanItem,
    pub tail: EvalSpanItem,
}

#[must_use]
pub fn evaluate_relation_f1(gold: &[EvalRelationItem], pred: &[EvalRelationItem]) -> EvalF1Summary {
    let mut matched_gold = vec![false; gold.len()];
    let mut tp = 0usize;
    let mut fp = 0usize;
    for p in pred {
        if let Some((gi, _)) = gold
            .iter()
            .enumerate()
            .find(|(idx, g)| !matched_gold[*idx] && *g == p)
        {
            matched_gold[gi] = true;
            tp += 1;
        } else {
            fp += 1;
        }
    }
    let fn_count = matched_gold.iter().filter(|&&m| !m).count();
    compute_f1(tp, fp, fn_count)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationPoint {
    pub confidence: f64,
    pub correct: bool,
}

#[must_use]
pub fn compute_expected_calibration_error(points: &[CalibrationPoint], n_bins: usize) -> f64 {
    if points.is_empty() || n_bins == 0 {
        return 0.0;
    }
    let mut counts = vec![0usize; n_bins];
    let mut conf_sums = vec![0.0_f64; n_bins];
    let mut acc_sums = vec![0.0_f64; n_bins];
    for p in points {
        let c = p.confidence.clamp(0.0, 1.0);
        let idx = ((c * (n_bins as f64)).floor() as usize).min(n_bins - 1);
        counts[idx] += 1;
        conf_sums[idx] += c;
        if p.correct {
            acc_sums[idx] += 1.0;
        }
    }
    let total = points.len() as f64;
    let mut ece = 0.0_f64;
    for i in 0..n_bins {
        if counts[i] > 0 {
            let n = counts[i] as f64;
            let avg_conf = conf_sums[i] / n;
            let avg_acc = acc_sums[i] / n;
            ece += (n / total) * (avg_conf - avg_acc).abs();
        }
    }
    ece
}

#[must_use]
pub fn check_split_overlap_leakage(train_texts: &[String], eval_texts: &[String]) -> usize {
    let train_hashes: BTreeSet<String> = train_texts
        .iter()
        .map(|s| sha256_prefixed(s.trim().to_ascii_lowercase().as_bytes()))
        .collect();
    eval_texts
        .iter()
        .filter(|s| {
            train_hashes.contains(&sha256_prefixed(s.trim().to_ascii_lowercase().as_bytes()))
        })
        .count()
}

pub fn validate_dua_dataset_path(
    dataset_name: &str,
    candidate: &Path,
    repo_root: &Path,
    network_fetch: bool,
) -> Result<(), AgentEvalError> {
    if network_fetch {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::DuaViolation,
            format!("DUA dataset `{dataset_name}` strictly forbids network_fetch=true"),
        ));
    }
    let raw_str = candidate.to_string_lossy();
    let lower = raw_str.trim().to_ascii_lowercase();
    if lower.is_empty()
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("s3://")
        || lower.starts_with("hf://")
        || lower.starts_with("ftp://")
    {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::DuaViolation,
            format!("DUA dataset `{dataset_name}` must be a local filesystem path outside repo"),
        ));
    }
    if candidate
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::DuaViolation,
            "path traversal (`..`) refused in DUA dataset path",
        ));
    }
    let canon_cand = std::fs::canonicalize(candidate).unwrap_or_else(|_| candidate.to_path_buf());
    let canon_repo = std::fs::canonicalize(repo_root).unwrap_or_else(|_| repo_root.to_path_buf());
    if canon_cand.starts_with(&canon_repo) || candidate.starts_with(repo_root) {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::DuaViolation,
            format!(
                "credentialed DUA dataset `{dataset_name}` must reside outside repository root"
            ),
        ));
    }
    Ok(())
}

pub fn check_restricted_terminology_bundle(name: &str) -> Result<(), AgentEvalError> {
    let lc = name.trim().to_ascii_lowercase();
    if matches!(
        lc.as_str(),
        "snomed" | "snomed_ct" | "snomed-ct" | "cpt" | "umls_mrconso" | "umls_full" | "loinc_full"
    ) {
        return Err(AgentEvalError::new(
            AgentEvalErrorKind::BlockedExternalRights,
            format!(
                "restricted terminology `{name}` cannot be bundled in MedScale; supply external user-licensed directory"
            ),
        ));
    }
    Ok(())
}
