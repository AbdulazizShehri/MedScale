//! Historical MESC interoperability compatibility plus current project separation.
//! MESC is a separate project/repository and is not a MedScale completion,
//! startup, workflow, release, or execution authority.

use medscale_contracts::external_artifact::MescArtifactDoctorStatus;
use medscale_core::build_doctor_report;

#[test]
fn doctor_reports_mesc_optional_not_required() {
    let s = MescArtifactDoctorStatus::not_configured();
    assert!(!s.required, "mesc_integration_required must be false");
    assert_eq!(s.integration_status, "NOT_CONFIGURED");
    assert!(!s.artifact_admitted);
    assert!(!s.blocks_release(), "MESC absence must not fail release");
    assert!(s.is_honest_ready_base());
}

#[test]
fn admit_path_stays_fail_closed_but_optional() {
    let s = MescArtifactDoctorStatus::gate_blocked();
    assert!(!s.required);
    assert_eq!(s.integration_status, "NOT_AVAILABLE");
    assert!(!s.blocks_release());
    assert!(s.is_honest_ready_base());
}

#[test]
fn trusted_core_starts_and_workflow_ready_with_no_mesc_artifact() {
    let report = build_doctor_report(None, false, false);
    assert!(!report.mesc_artifact.artifact_admitted);
    assert!(!report.mesc_artifact.required);
    assert!(!report.mesc_artifact.blocks_release());
    assert_eq!(report.mesc_artifact.disposition, "SEPARATE_PROJECT");
    assert_eq!(report.mesc_artifact.gate, "NONE");
    assert_eq!(report.mesc_artifact.integration_status, "OUT_OF_SCOPE");
    assert!(report.mesc_artifact.is_honest_ready_base());
    assert!(
        report.workflow.workflow_ready_base,
        "offline workflow must stay ready with no MESC artifact"
    );
}

#[test]
fn malformed_optional_artifact_verify_fails_closed() {
    let dir = std::env::temp_dir().join(format!("mesc-decouple-malformed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("manifest.json"), b"{ not json").expect("manifest");
    let err = medscale_pack::verify_mesc_release_dir(&dir).unwrap_err();
    assert_eq!(
        err.report().reason,
        medscale_contracts::external_artifact::MescVerifyReason::MalformedManifest
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn legacy_doctor_json_still_parses_without_new_fields() {
    let legacy = serde_json::json!({
        "present": true,
        "artifact_admitted": false,
        "verifier_ready_base": true,
        "python_runtime_imported": false,
        "shared_db_or_keys": false,
        "disposition": "ARTIFACT_IMPORT",
        "gate": "MESC_RELEASED_ARTIFACT"
    });
    let s: MescArtifactDoctorStatus =
        serde_json::from_value(legacy).expect("legacy doctor JSON parses");
    assert!(!s.required);
    assert_eq!(s.integration_status, "NOT_CONFIGURED");
    assert!(s.is_honest_ready_base());
}
