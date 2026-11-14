//! Spec 021 minimum lovable workflow (Trusted V1 Q07).

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::{CoreFacade, JourneyConfig, build_doctor_report, run_minimum_lovable_journey};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn work_root(name: &str) -> PathBuf {
    let base = if Path::new(r"D:\medscale-tmp").exists() || Path::new(r"D:\").exists() {
        PathBuf::from(r"D:\medscale-tmp")
    } else {
        std::env::temp_dir()
    };
    let dir = base.join(format!("medscale-021-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn req(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new("req-021"),
        VaultId::new("journey-vault"),
        RealmId::new("workflow-realm"),
        AuthorityScopeId::new("workflow-scope"),
        capability,
        body,
    )
}

#[test]
fn doctor_workflow_honesty_021() {
    let report = build_doctor_report(None, false, false);
    assert!(report.workflow.present);
    assert!(report.workflow.workflow_ready_base);
    assert!(!report.workflow.release_ready);
    assert!(report.workflow.synthetic_only);
    assert!(report.workflow.disclosure_append_supported);
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.contains("WORKFLOW_READY_BASE") && n.contains("RELEASE_READY=false"))
    );
}

#[test]
fn journey_accept_backup_restore_021() {
    let root = work_root("accept");
    let vault = root.join("vault");
    let backup = root.join("backup");
    let restore = root.join("restore");
    fs::create_dir_all(&vault).unwrap();

    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let cfg = JourneyConfig {
        vault_id: "journey-vault".to_owned(),
        vault_root: vault.to_string_lossy().into_owned(),
        fixture_path: fixture("fixtures/synthetic/fhir/r4/presentation/patient-golden.json")
            .to_string_lossy()
            .into_owned(),
        subject: "journey-subject".to_owned(),
        backup_dir: backup.to_string_lossy().into_owned(),
        restore_dir: restore.to_string_lossy().into_owned(),
        accept: true,
    };
    let report = run_minimum_lovable_journey(&facade, &cfg).expect("journey");
    assert!(report.is_honest_ready_base());
    assert!(report.source_id.is_some());
    assert!(report.assertion_id.is_some());
    assert!(report.disclosure_id.is_some());
    assert!(report.steps.iter().all(|s| s.ok));
    assert!(backup.join("manifest.json").is_file());
    assert!(restore.exists());
}

#[test]
fn journey_reject_no_assertion_021() {
    let root = work_root("reject");
    let vault = root.join("vault");
    let backup = root.join("backup");
    let restore = root.join("restore");
    fs::create_dir_all(&vault).unwrap();

    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let cfg = JourneyConfig {
        vault_id: "journey-vault".to_owned(),
        vault_root: vault.to_string_lossy().into_owned(),
        fixture_path: fixture("fixtures/synthetic/fhir/r4/presentation/patient-golden.json")
            .to_string_lossy()
            .into_owned(),
        subject: "reject-subject".to_owned(),
        backup_dir: backup.to_string_lossy().into_owned(),
        restore_dir: restore.to_string_lossy().into_owned(),
        accept: false,
    };
    let report = run_minimum_lovable_journey(&facade, &cfg).expect("reject journey");
    assert!(report.is_honest_ready_base());
    assert!(report.source_id.is_some());
    assert!(report.assertion_id.is_none());
    assert!(report.disclosure_id.is_some());
    let decision = report
        .steps
        .iter()
        .find(|s| s.step == medscale_contracts::workflow::JourneyStep::AcceptOrReject)
        .expect("accept/reject step");
    assert_eq!(decision.detail["decision"], "reject");
}

#[test]
fn reject_and_list_disclosures_facade_021() {
    let root = work_root("facade");
    let vault = root.join("vault");
    fs::create_dir_all(&vault).unwrap();
    let facade = CoreFacade::new_legacy_lease_only_engineering();

    assert!(
        facade
            .dispatch(req(
                Capability::AcquireLease,
                RequestBody::AcquireLease {
                    client_id: OpaqueId::new("cli-021"),
                    holder_id_hint: None,
                },
            ))
            .result
            .is_ok()
    );
    assert!(
        facade
            .dispatch(req(
                Capability::OpenSyntheticVault,
                RequestBody::OpenSyntheticVault {
                    vault_root: vault.to_string_lossy().into_owned(),
                },
            ))
            .result
            .is_ok()
    );

    let created = facade
        .dispatch(req(
            Capability::CreateProposal,
            RequestBody::CreateProposal {
                subject_ref: Some(OpaqueId::new("s")),
                claim_kind: "note".to_owned(),
                payload: serde_json::json!({"text": "synthetic"}),
                evidence_refs: vec![],
            },
        ))
        .result
        .unwrap();
    let ResponseBody::Created {
        object_id: proposal_id,
    } = created
    else {
        panic!("proposal");
    };

    let rejected = facade
        .dispatch(req(
            Capability::RejectProposal,
            RequestBody::RejectProposal {
                proposal_id: proposal_id.clone(),
                actor: OpaqueId::new("op"),
                rationale: "no".to_owned(),
            },
        ))
        .result
        .unwrap();
    let ResponseBody::Rejected { .. } = rejected else {
        panic!("reject");
    };

    let disclosure = facade
        .dispatch(req(
            Capability::AppendDisclosure,
            RequestBody::AppendDisclosure {
                purpose: "test".to_owned(),
                scope: "unit".to_owned(),
                subject_ref: None,
                artifact_refs: vec![proposal_id],
                export_digest: None,
                note: Some("synthetic".to_owned()),
            },
        ))
        .result
        .unwrap();
    let ResponseBody::DisclosureAppended { record } = disclosure else {
        panic!("disclosure");
    };
    assert!(record.synthetic_only);
    assert!(!record.release_ready_claimed);

    let listed = facade
        .dispatch(req(
            Capability::ListDisclosures,
            RequestBody::ListDisclosures,
        ))
        .result
        .unwrap();
    let ResponseBody::DisclosureList { records } = listed else {
        panic!("list");
    };
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].disclosure_id, record.disclosure_id);
}

#[test]
fn workflow_two_process_reopen_021() {
    if let Ok(root) = std::env::var("MEDSCALE_021_VAULT") {
        let source_id = std::env::var("MEDSCALE_021_SOURCE").unwrap();
        let digest = std::env::var("MEDSCALE_021_DIGEST").unwrap();
        let facade = CoreFacade::new_legacy_lease_only_engineering();
        assert!(
            facade
                .dispatch(req(
                    Capability::AcquireLease,
                    RequestBody::AcquireLease {
                        client_id: OpaqueId::new("child-021"),
                        holder_id_hint: None,
                    },
                ))
                .result
                .is_ok()
        );
        assert!(
            facade
                .dispatch(req(
                    Capability::OpenSyntheticVault,
                    RequestBody::OpenSyntheticVault {
                        vault_root: root.clone(),
                    },
                ))
                .result
                .is_ok()
        );
        let visibility = facade
            .dispatch(req(
                Capability::ReadCanonicalVisibility,
                RequestBody::ReadCanonicalVisibility {
                    source_id: OpaqueId::new(source_id),
                },
            ))
            .result
            .unwrap();
        match visibility {
            ResponseBody::Visibility {
                visible,
                content_digest,
                ..
            } => {
                assert!(visible);
                assert_eq!(content_digest.to_hex(), digest);
            }
            other => panic!("unexpected {other:?}"),
        }
        return;
    }

    let root = work_root("twoproc");
    let vault = root.join("vault");
    let backup = root.join("backup");
    let restore = root.join("restore");
    fs::create_dir_all(&vault).unwrap();

    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let cfg = JourneyConfig {
        vault_id: "journey-vault".to_owned(),
        vault_root: vault.to_string_lossy().into_owned(),
        fixture_path: fixture("fixtures/synthetic/fhir/r4/presentation/patient-golden.json")
            .to_string_lossy()
            .into_owned(),
        subject: "twoproc-subject".to_owned(),
        backup_dir: backup.to_string_lossy().into_owned(),
        restore_dir: restore.to_string_lossy().into_owned(),
        accept: true,
    };
    let report = run_minimum_lovable_journey(&facade, &cfg).expect("parent journey");
    let source_id = report.source_id.clone().expect("source");
    // Digest from verify step detail or re-read visibility
    let visibility = facade
        .dispatch(req(
            Capability::ReadCanonicalVisibility,
            RequestBody::ReadCanonicalVisibility {
                source_id: OpaqueId::new(source_id.clone()),
            },
        ))
        .result
        .unwrap();
    let digest = match visibility {
        ResponseBody::Visibility { content_digest, .. } => content_digest.to_hex(),
        other => panic!("{other:?}"),
    };
    assert!(
        facade
            .dispatch(req(Capability::CloseVault, RequestBody::CloseVault))
            .result
            .is_ok()
    );
    drop(facade);

    let exe = std::env::current_exe().expect("current exe");
    let status = Command::new(&exe)
        .env("MEDSCALE_021_VAULT", vault.to_str().unwrap())
        .env("MEDSCALE_021_SOURCE", &source_id)
        .env("MEDSCALE_021_DIGEST", &digest)
        .env("RUST_TEST_THREADS", "1")
        .args(["workflow_two_process_reopen_021", "--exact", "--nocapture"])
        .status()
        .expect("spawn child");
    assert!(status.success(), "child process failed: {status}");
}
