//! Spec 021 minimum lovable trusted workflow journey runner.
//!
//! Composes existing CoreFacade capabilities; does not invent a second authority path.

use std::path::Path;

use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{AuthorityScopeId, DigestSha256, OpaqueId, RealmId, VaultId};
use medscale_contracts::workflow::{ImportPreview, JourneyReport, JourneyStep, JourneyStepResult};
use medscale_contracts::{AUTHORITY_SCHEMA_VERSION, MEDSCALE_PRODUCT_NAME, MEDSCALE_VERSION};
use serde_json::json;

use crate::CoreFacade;
use crate::doctor::{build_doctor_report, privacy_proof_artifact_present};

/// Configuration for the synthetic minimum lovable journey.
#[derive(Debug, Clone)]
pub struct JourneyConfig {
    pub vault_id: String,
    pub vault_root: String,
    pub fixture_path: String,
    pub subject: String,
    pub backup_dir: String,
    pub restore_dir: String,
    /// When false, preview is Rejected instead of Accepted.
    pub accept: bool,
}

/// Run the documented synthetic end-to-end journey against one CoreFacade.
pub fn run_minimum_lovable_journey(
    facade: &CoreFacade,
    cfg: &JourneyConfig,
) -> Result<JourneyReport, AuthorityError> {
    let vault_id = VaultId::new(&cfg.vault_id);
    let realm_id = RealmId::new("workflow-realm");
    let scope_id = AuthorityScopeId::new("workflow-scope");
    let mut next_req = 0u64;
    let mut steps = Vec::new();
    let mut session_id: Option<OpaqueId> = None;

    let mut req = |session_id: &Option<OpaqueId>,
                   capability: Capability,
                   body: RequestBody|
     -> AuthorityRequest {
        next_req += 1;
        let mut r = AuthorityRequest::new(
            OpaqueId::new(format!("journey-req-{next_req}")),
            vault_id.clone(),
            realm_id.clone(),
            scope_id.clone(),
            capability,
            body,
        );
        r.session_id = session_id.clone();
        r
    };

    // INSTALL — identity / version surface
    steps.push(JourneyStepResult {
        step: JourneyStep::Install,
        ok: true,
        detail: json!({
            "product": MEDSCALE_PRODUCT_NAME,
            "version": MEDSCALE_VERSION,
            "schema_version": AUTHORITY_SCHEMA_VERSION,
        }),
    });

    // START OFFLINE — acquire lease + open Strict client session (Spec 024)
    let lease = facade
        .dispatch(req(
            &session_id,
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("journey-cli"),
                holder_id_hint: Some(OpaqueId::new("journey-holder")),
            },
        ))
        .result?;
    let ResponseBody::Lease { holder_id, .. } = lease else {
        return Err(AuthorityError::InvalidArgument {
            message: "expected lease".to_owned(),
        });
    };
    let opened = facade
        .dispatch(req(
            &session_id,
            Capability::OpenSession,
            RequestBody::OpenSession {
                holder_id,
                granted: Capability::operator_grants(),
                ttl_ticks: 1_000_000,
            },
        ))
        .result?;
    let ResponseBody::Session {
        session_id: sid, ..
    } = opened
    else {
        return Err(AuthorityError::InvalidArgument {
            message: "expected session".to_owned(),
        });
    };
    session_id = Some(sid);
    steps.push(JourneyStepResult {
        step: JourneyStep::StartOffline,
        ok: true,
        detail: json!({ "local_only": true, "egress": "DEFAULT_DENY" }),
    });

    // INSPECT PRIVACY / CAPABILITY
    let doctor = build_doctor_report(
        Some(&cfg.vault_root),
        false,
        privacy_proof_artifact_present(),
    );
    let workflow_ok = doctor.workflow.workflow_ready_base && !doctor.workflow.release_ready;
    steps.push(JourneyStepResult {
        step: JourneyStep::InspectPrivacyCapability,
        ok: workflow_ok && doctor.local_only && !doctor.real_phi_authorized,
        detail: json!({
            "workflow_ready_base": doctor.workflow.workflow_ready_base,
            "release_ready": doctor.workflow.release_ready,
            "synthetic_only": doctor.synthetic_only,
            "local_only": doctor.local_only,
        }),
    });
    if !workflow_ok {
        return Err(AuthorityError::InvalidArgument {
            message: "doctor workflow honesty failed".to_owned(),
        });
    }

    // LOAD SYNTHETIC
    facade
        .dispatch(req(
            &session_id,
            Capability::OpenSyntheticVault,
            RequestBody::OpenSyntheticVault {
                vault_root: cfg.vault_root.clone(),
            },
        ))
        .result?;
    steps.push(JourneyStepResult {
        step: JourneyStep::LoadSynthetic,
        ok: true,
        detail: json!({ "vault_root": cfg.vault_root }),
    });

    // IMPORT FHIR
    let bytes = std::fs::read(&cfg.fixture_path).map_err(|e| AuthorityError::InvalidArgument {
        message: format!("fixture read: {e}"),
    })?;
    let resource: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| AuthorityError::InvalidArgument {
            message: format!("fixture json: {e}"),
        })?;
    let ingested = facade
        .dispatch(req(
            &session_id,
            Capability::IngestFhirSynthetic,
            RequestBody::IngestFhirSynthetic {
                media_type: "application/fhir+json".to_owned(),
                bytes: bytes.clone(),
                fhir_version_hint: Some("4.0.1".to_owned()),
                attach_validator_fixture_id: None,
            },
        ))
        .result?;
    let ResponseBody::Ingested { receipt } = ingested else {
        return Err(AuthorityError::InvalidArgument {
            message: "expected ingest receipt".to_owned(),
        });
    };
    let source_id = receipt.source_id.ok_or(AuthorityError::InvalidArgument {
        message: format!("ingest outcome {:?}", receipt.outcome),
    })?;
    let content_digest = receipt.content_digest.clone();
    steps.push(JourneyStepResult {
        step: JourneyStep::ImportFhir,
        ok: true,
        detail: json!({
            "source_id": source_id.as_str(),
            "digest": content_digest.as_ref().map(DigestSha256::to_hex),
        }),
    });

    // PREVIEW — proposal + loss-aware export (not yet promoted)
    let resource_type = resource
        .get("resourceType")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_owned();
    let claim_kind = resource_type.to_ascii_lowercase();
    let subject_ref = OpaqueId::new(&cfg.subject);
    let created = facade
        .dispatch(req(
            &session_id,
            Capability::CreateProposal,
            RequestBody::CreateProposal {
                subject_ref: Some(subject_ref.clone()),
                claim_kind: claim_kind.clone(),
                payload: json!({ "resource": resource.clone() }),
                evidence_refs: vec![source_id.clone()],
            },
        ))
        .result?;
    let ResponseBody::Created {
        object_id: proposal_id,
    } = created
    else {
        return Err(AuthorityError::InvalidArgument {
            message: "expected proposal".to_owned(),
        });
    };
    let export = facade
        .dispatch(req(
            &session_id,
            Capability::ExportFhirLossAware,
            RequestBody::ExportFhirLossAware {
                resource: resource.clone(),
            },
        ))
        .result?;
    let ResponseBody::FhirLossAwareExport { export } = export else {
        return Err(AuthorityError::InvalidArgument {
            message: "expected loss-aware export".to_owned(),
        });
    };
    let preview = ImportPreview {
        source_id: source_id.clone(),
        proposal_id: proposal_id.clone(),
        subject_ref: subject_ref.clone(),
        claim_kind: claim_kind.clone(),
        resource_type: resource_type.clone(),
        loss_aware_export: export.clone(),
        promoted: false,
    };
    steps.push(JourneyStepResult {
        step: JourneyStep::Preview,
        ok: true,
        detail: serde_json::to_value(&preview).unwrap_or(json!({})),
    });

    // ACCEPT / REJECT
    let mut assertion_id: Option<OpaqueId> = None;
    if cfg.accept {
        let promoted = facade
            .dispatch(req(
                &session_id,
                Capability::PromoteProposal,
                RequestBody::PromoteProposal {
                    proposal_id: proposal_id.clone(),
                    authorized_by: OpaqueId::new("journey-operator"),
                    subject_ref: subject_ref.clone(),
                },
            ))
            .result?;
        let ResponseBody::Promoted {
            assertion_id: aid, ..
        } = promoted
        else {
            return Err(AuthorityError::InvalidArgument {
                message: "expected promote".to_owned(),
            });
        };
        assertion_id = Some(aid.clone());
        steps.push(JourneyStepResult {
            step: JourneyStep::AcceptOrReject,
            ok: true,
            detail: json!({ "decision": "accept", "assertion_id": aid.as_str() }),
        });
    } else {
        let rejected = facade
            .dispatch(req(
                &session_id,
                Capability::RejectProposal,
                RequestBody::RejectProposal {
                    proposal_id: proposal_id.clone(),
                    actor: OpaqueId::new("journey-operator"),
                    rationale: "synthetic reject path".to_owned(),
                },
            ))
            .result?;
        let ResponseBody::Rejected { audit_id, .. } = rejected else {
            return Err(AuthorityError::InvalidArgument {
                message: "expected reject".to_owned(),
            });
        };
        steps.push(JourneyStepResult {
            step: JourneyStep::AcceptOrReject,
            ok: true,
            detail: json!({ "decision": "reject", "audit_id": audit_id.as_str() }),
        });
        // Reject path still completes remaining non-clinical steps where possible;
        // presentation may be empty — still exercise export/disclosure/backup on source.
    }

    // TIMELINE / BRIEF / COVERAGE
    let timeline = facade
        .dispatch(req(
            &session_id,
            Capability::GetTimeline,
            RequestBody::GetTimeline {
                subject_ref: subject_ref.clone(),
            },
        ))
        .result?;
    let brief = facade
        .dispatch(req(
            &session_id,
            Capability::GetBrief,
            RequestBody::GetBrief {
                subject_ref: subject_ref.clone(),
            },
        ))
        .result?;
    let coverage = facade
        .dispatch(req(
            &session_id,
            Capability::GetCoverage,
            RequestBody::GetCoverage {
                subject_ref: subject_ref.clone(),
            },
        ))
        .result?;
    let (tl_events, brief_sections, cov_slots) = match (&timeline, &brief, &coverage) {
        (
            ResponseBody::Timeline { body: tl, .. },
            ResponseBody::Brief { body: br, .. },
            ResponseBody::Coverage { body: cv, .. },
        ) => (tl.events.len(), br.sections.identity.len(), cv.slots.len()),
        _ => {
            return Err(AuthorityError::InvalidArgument {
                message: "expected presentation bodies".to_owned(),
            });
        }
    };
    let presentation_ok = if cfg.accept {
        tl_events > 0 && brief_sections > 0 && cov_slots > 0
    } else {
        true
    };
    steps.push(JourneyStepResult {
        step: JourneyStep::TimelineBriefCoverage,
        ok: presentation_ok,
        detail: json!({
            "timeline_events": tl_events,
            "brief_identity_fields": brief_sections,
            "coverage_slots": cov_slots,
        }),
    });
    if !presentation_ok {
        return Err(AuthorityError::InvalidArgument {
            message: "expected non-empty presentation after accept".to_owned(),
        });
    }

    // SOURCE DRILL-DOWN (accept path only when assertion present)
    if cfg.accept {
        let dd = facade
            .dispatch(req(
                &session_id,
                Capability::DrillDownPresentation,
                RequestBody::DrillDownPresentation {
                    subject_ref: subject_ref.clone(),
                    field_key: "patient.birthDate".to_owned(),
                    assertion_id: assertion_id.clone(),
                },
            ))
            .result?;
        let ResponseBody::DrillDown { result } = dd else {
            return Err(AuthorityError::InvalidArgument {
                message: "expected drill-down".to_owned(),
            });
        };
        steps.push(JourneyStepResult {
            step: JourneyStep::SourceDrillDown,
            ok: true,
            detail: json!({
                "field_key": result.field_key,
                "source_id": source_id.as_str(),
            }),
        });
    } else {
        steps.push(JourneyStepResult {
            step: JourneyStep::SourceDrillDown,
            ok: true,
            detail: json!({ "skipped": true, "reason": "rejected_proposal" }),
        });
    }

    // CLOSE
    facade
        .dispatch(req(
            &session_id,
            Capability::CloseVault,
            RequestBody::CloseVault,
        ))
        .result?;
    steps.push(JourneyStepResult {
        step: JourneyStep::Close,
        ok: true,
        detail: json!({ "closed": true }),
    });

    // REOPEN (same process / same facade instance — two-process covered by integration test)
    facade
        .dispatch(req(
            &session_id,
            Capability::OpenSyntheticVault,
            RequestBody::OpenSyntheticVault {
                vault_root: cfg.vault_root.clone(),
            },
        ))
        .result?;
    steps.push(JourneyStepResult {
        step: JourneyStep::Reopen,
        ok: true,
        detail: json!({ "reopened": true }),
    });

    // VERIFY SAME RECORD
    let visibility = facade
        .dispatch(req(
            &session_id,
            Capability::ReadCanonicalVisibility,
            RequestBody::ReadCanonicalVisibility {
                source_id: source_id.clone(),
            },
        ))
        .result?;
    let ResponseBody::Visibility {
        visible,
        content_digest: vis_digest,
        ..
    } = visibility
    else {
        return Err(AuthorityError::InvalidArgument {
            message: "expected visibility".to_owned(),
        });
    };
    let digest_ok = match (&content_digest, &vis_digest) {
        (Some(a), b) => a == b,
        _ => false,
    };
    steps.push(JourneyStepResult {
        step: JourneyStep::VerifySameRecord,
        ok: visible && digest_ok,
        detail: json!({
            "visible": visible,
            "digest_match": digest_ok,
            "source_id": source_id.as_str(),
        }),
    });
    if !(visible && digest_ok) {
        return Err(AuthorityError::InvalidArgument {
            message: "post-reopen record verification failed".to_owned(),
        });
    }

    // EXPORT
    let export2 = facade
        .dispatch(req(
            &session_id,
            Capability::ExportFhirLossAware,
            RequestBody::ExportFhirLossAware {
                resource: resource.clone(),
            },
        ))
        .result?;
    let ResponseBody::FhirLossAwareExport { export: export2 } = export2 else {
        return Err(AuthorityError::InvalidArgument {
            message: "expected export".to_owned(),
        });
    };
    let export_bytes = serde_json::to_vec(&export2.exported_json).unwrap_or_default();
    let export_digest = DigestSha256::of(&export_bytes);
    steps.push(JourneyStepResult {
        step: JourneyStep::Export,
        ok: !export2.full_conformance_claimed,
        detail: json!({
            "resource_type": export2.resource_type,
            "field_losses": export2.field_losses.len(),
            "full_conformance_claimed": export2.full_conformance_claimed,
            "export_digest": export_digest.to_hex(),
        }),
    });

    // DISCLOSURE RECORD
    let mut artifact_refs = vec![source_id.clone()];
    if let Some(aid) = &assertion_id {
        artifact_refs.push(aid.clone());
    }
    let disclosure = facade
        .dispatch(req(
            &session_id,
            Capability::AppendDisclosure,
            RequestBody::AppendDisclosure {
                purpose: "synthetic_journey_export".to_owned(),
                scope: "subject_brief_bundle".to_owned(),
                subject_ref: Some(subject_ref.clone()),
                artifact_refs,
                export_digest: Some(export_digest.clone()),
                note: Some("Spec 021 READY_BASE synthetic disclosure".to_owned()),
            },
        ))
        .result?;
    let ResponseBody::DisclosureAppended { record } = disclosure else {
        return Err(AuthorityError::InvalidArgument {
            message: "expected disclosure".to_owned(),
        });
    };
    let disclosure_ok = record.synthetic_only && !record.release_ready_claimed;
    steps.push(JourneyStepResult {
        step: JourneyStep::DisclosureRecord,
        ok: disclosure_ok,
        detail: json!({
            "disclosure_id": record.disclosure_id.as_str(),
            "purpose": record.purpose,
            "release_ready_claimed": record.release_ready_claimed,
        }),
    });
    if !disclosure_ok {
        return Err(AuthorityError::InvalidArgument {
            message: "disclosure honesty failed".to_owned(),
        });
    }

    // BACKUP
    facade
        .dispatch(req(
            &session_id,
            Capability::BackupVault,
            RequestBody::BackupVault {
                destination: cfg.backup_dir.clone(),
            },
        ))
        .result?;
    let backup_manifest = Path::new(&cfg.backup_dir).join("BACKUP_MANIFEST.json");
    let backup_ok = backup_manifest.is_file()
        || Path::new(&cfg.backup_dir).join("manifest.json").is_file()
        || Path::new(&cfg.backup_dir).is_dir();
    steps.push(JourneyStepResult {
        step: JourneyStep::Backup,
        ok: backup_ok,
        detail: json!({ "destination": cfg.backup_dir }),
    });

    // VERIFY BACKUP — destination non-empty
    let verify_ok = Path::new(&cfg.backup_dir)
        .read_dir()
        .map(|mut d| d.next().is_some())
        .unwrap_or(false);
    steps.push(JourneyStepResult {
        step: JourneyStep::VerifyBackup,
        ok: verify_ok,
        detail: json!({ "non_empty": verify_ok }),
    });
    if !verify_ok {
        return Err(AuthorityError::InvalidArgument {
            message: "backup verify failed".to_owned(),
        });
    }

    // RESTORE
    let restored = facade
        .dispatch(req(
            &session_id,
            Capability::RestoreVault,
            RequestBody::RestoreVault {
                source: cfg.backup_dir.clone(),
                destination: cfg.restore_dir.clone(),
            },
        ))
        .result?;
    let ResponseBody::Restored {
        sources_restored, ..
    } = restored
    else {
        return Err(AuthorityError::InvalidArgument {
            message: "expected restore".to_owned(),
        });
    };
    steps.push(JourneyStepResult {
        step: JourneyStep::Restore,
        ok: sources_restored >= 1,
        detail: json!({
            "sources_restored": sources_restored,
            "destination": cfg.restore_dir,
        }),
    });
    if sources_restored < 1 {
        return Err(AuthorityError::InvalidArgument {
            message: "restore produced no sources".to_owned(),
        });
    }

    Ok(JourneyReport {
        schema_version: 1,
        synthetic_only: true,
        workflow_ready_base: true,
        release_ready: false,
        subject_ref: cfg.subject.clone(),
        source_id: Some(source_id.as_str().to_owned()),
        assertion_id: assertion_id.map(|a| a.as_str().to_owned()),
        disclosure_id: Some(record.disclosure_id.as_str().to_owned()),
        steps,
    })
}
