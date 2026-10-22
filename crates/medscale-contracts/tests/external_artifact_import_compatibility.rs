//! Public import-path compatibility during MedScale artifact module naming migration.
use medscale_contracts::{external_artifact, mesc};

#[test]
fn legacy_public_import_remains_byte_compatible() {
    let renamed = external_artifact::ExternalArtifactDoctorStatus::separate_project();
    let legacy: mesc::MescArtifactDoctorStatus = renamed.clone();
    assert_eq!(
        serde_json::to_vec(&renamed).unwrap(),
        serde_json::to_vec(&legacy).unwrap()
    );
    assert!(legacy.is_honest_ready_base());
}
