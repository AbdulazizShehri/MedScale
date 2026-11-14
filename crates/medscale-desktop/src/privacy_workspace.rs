//! Privacy Gate view-models (Spec 079, T079-08).
//!
//! Desktop reads and mutates privacy state only through the Core-owned
//! `CliSession` (facade authority; never storage or key material directly).
//! Every function maps one typed Core result to plain view-model rows plus
//! an explicit status string; no product data is synthesized. Reuses the
//! `desktop-projects` session Specs 074-078 already open.
//!
//! Scope: the Project's classifications, de-identification receipts and
//! egress decisions, an egress check for one artifact and boundary, and
//! receipt revocation. Profile creation, transforms and re-identification
//! stay CLI-only in this slice, the same pattern Specs 077/078 used for
//! identity and lane creation.

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_contracts::privacy_gate::{DeidReceipt, EgressBoundary, EgressDecision};
use medscale_core::CliSession;

/// One classification row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassificationRowVm {
    pub artifact_id: String,
    pub data_class: String,
    pub basis: String,
    pub revision: u64,
}

/// One de-identification receipt row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReceiptRowVm {
    pub id: String,
    pub source: String,
    pub output: String,
    pub status: String,
    pub residual: String,
    pub revision: u64,
}

/// One egress decision row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DecisionRowVm {
    pub artifact_id: String,
    pub boundary: String,
    pub outcome: String,
    pub reason: String,
}

/// Everything the Privacy route shows for one Project.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrivacyOverviewVm {
    pub classifications: Vec<ClassificationRowVm>,
    pub receipts: Vec<ReceiptRowVm>,
    pub decisions: Vec<DecisionRowVm>,
}

/// Maps a typed Core error to an explicit status (no payload leak).
#[must_use]
pub fn status_message(err: &AuthorityError) -> &'static str {
    match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope => "Denied by Core authority",
        AuthorityError::NotFound => "Missing: not found in this vault",
        AuthorityError::Conflict { .. } => "Conflict: stale revision or already revoked",
        AuthorityError::InvalidArgument { .. } => "Invalid: rejected before any write",
        AuthorityError::Corrupt { .. } => "Corrupt: integrity check failed",
        AuthorityError::UnsupportedSchema { .. } => "Unsupported: unknown schema value",
        AuthorityError::ExternalGateRequired { .. } => {
            "Blocked: requires an explicit gate not granted here"
        }
        _ => "Unavailable: vault or Core not ready",
    }
}

fn receipt_row(r: &DeidReceipt) -> ReceiptRowVm {
    ReceiptRowVm {
        id: r.header.id.as_str().to_owned(),
        source: r.source_artifact_id.as_str().to_owned(),
        output: r.output_artifact_id.as_str().to_owned(),
        status: r.status.as_str().to_owned(),
        residual: r.residual.status.as_str().to_owned(),
        revision: r.revision,
    }
}

fn decision_row(d: &EgressDecision) -> DecisionRowVm {
    DecisionRowVm {
        artifact_id: d.artifact_id.as_str().to_owned(),
        boundary: d.boundary.as_str().to_owned(),
        outcome: d.outcome.as_str().to_owned(),
        reason: d.reason.as_str().to_owned(),
    }
}

/// Loads the Project's classifications, receipts and newest-first decisions.
pub fn refresh(
    session: &mut CliSession,
    project_id: &str,
) -> Result<PrivacyOverviewVm, AuthorityError> {
    let project = OpaqueId::new(project_id);
    let classifications = session
        .privacy_classification_list(project.clone())?
        .iter()
        .map(|c| ClassificationRowVm {
            artifact_id: c.artifact_id.as_str().to_owned(),
            data_class: c.data_class.as_str().to_owned(),
            basis: c.basis.as_str().to_owned(),
            revision: c.revision,
        })
        .collect();
    let receipts = session
        .privacy_receipt_list(project.clone())?
        .iter()
        .map(receipt_row)
        .collect();
    let decisions = session
        .privacy_egress_list(project, None)?
        .iter()
        .rev()
        .map(decision_row)
        .collect();
    Ok(PrivacyOverviewVm {
        classifications,
        receipts,
        decisions,
    })
}

/// Asks Core for an egress decision (persisted by Core).
pub fn check_egress(
    session: &mut CliSession,
    project_id: &str,
    artifact_id: &str,
    boundary: &str,
) -> Result<DecisionRowVm, AuthorityError> {
    let boundary = EgressBoundary::parse(boundary.trim())
        .map_err(|message| AuthorityError::InvalidArgument { message })?;
    let artifact_id = artifact_id.trim();
    if artifact_id.is_empty() {
        return Err(AuthorityError::InvalidArgument {
            message: "artifact id is required".to_owned(),
        });
    }
    let decision = session.privacy_egress_evaluate(
        OpaqueId::new(project_id),
        OpaqueId::new(artifact_id),
        boundary,
    )?;
    Ok(decision_row(&decision))
}

/// Revokes a receipt; its output can then no longer leave.
pub fn revoke_receipt(
    session: &mut CliSession,
    receipt_id: &str,
    expected_revision: u64,
) -> Result<ReceiptRowVm, AuthorityError> {
    let receipt = session.privacy_receipt_revoke(OpaqueId::new(receipt_id), expected_revision)?;
    Ok(receipt_row(&receipt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::privacy_gate::{
        DataClass, ProfileRule, SensitiveSpanKind, TransformOp,
    };

    fn session(name: &str) -> (CliSession, String) {
        let root = std::env::temp_dir().join(format!(
            "medscale-079-desktop-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut session = CliSession::connect("vault-079-desktop").unwrap();
        session.use_in_memory_privacy_keys();
        session
            .open_synthetic_vault(&root.display().to_string())
            .unwrap();
        let project = session
            .project_create("privacy".to_owned(), None)
            .unwrap()
            .header
            .id
            .as_str()
            .to_owned();
        (session, project)
    }

    #[test]
    fn privacy_view_models_flow_through_a_real_core_session() {
        let (mut session, project) = session("flow");
        let empty = refresh(&mut session, &project).unwrap();
        assert!(empty.classifications.is_empty() && empty.receipts.is_empty());

        let source = session
            .create_source_record(
                "text/plain".to_owned(),
                b"Patient: Maria Lopez, seen 2024-03-12.".to_vec(),
            )
            .unwrap();
        let rules = SensitiveSpanKind::ALL
            .iter()
            .map(|kind| ProfileRule {
                kind: *kind,
                op: TransformOp::Redact,
            })
            .collect();
        let profile = session
            .privacy_profile_create(
                OpaqueId::new(&project),
                "export".to_owned(),
                DataClass::ExternalDeidentified,
                rules,
                false,
            )
            .unwrap();
        let receipt = session
            .privacy_transform(
                OpaqueId::new(&project),
                source.clone(),
                profile.header.id,
                None,
                None,
                None,
                true,
            )
            .unwrap();

        let denied = check_egress(&mut session, &project, source.as_str(), "browse").unwrap();
        assert_eq!(denied.outcome, "deny");
        assert_eq!(denied.reason, "denied_unclassified");
        let allowed = check_egress(
            &mut session,
            &project,
            receipt.output_artifact_id.as_str(),
            "browse",
        )
        .unwrap();
        assert_eq!(allowed.outcome, "allow");
        assert!(check_egress(&mut session, &project, source.as_str(), "anywhere").is_err());
        assert!(check_egress(&mut session, &project, "  ", "browse").is_err());

        let overview = refresh(&mut session, &project).unwrap();
        assert_eq!(overview.receipts.len(), 1);
        assert_eq!(overview.receipts[0].status, "valid");
        assert_eq!(overview.classifications.len(), 1);
        assert_eq!(overview.classifications[0].basis, "deid_receipt");
        assert_eq!(overview.decisions.len(), 2);
        assert_eq!(overview.decisions[0].outcome, "allow", "newest first");

        let revoked = revoke_receipt(&mut session, receipt.header.id.as_str(), 1).unwrap();
        assert_eq!(revoked.status, "revoked");
        let err = revoke_receipt(&mut session, receipt.header.id.as_str(), 1).unwrap_err();
        assert_eq!(
            status_message(&err),
            "Conflict: stale revision or already revoked"
        );
        let after = check_egress(
            &mut session,
            &project,
            receipt.output_artifact_id.as_str(),
            "browse",
        )
        .unwrap();
        assert_eq!(after.reason, "denied_receipt_revoked");
    }

    #[test]
    fn unknown_project_reports_an_explicit_status() {
        let (mut session, _) = session("missing");
        let err = refresh(&mut session, "project-does-not-exist").unwrap_err();
        assert_eq!(status_message(&err), "Missing: not found in this vault");
    }
}
