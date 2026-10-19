//! Spec 021: a subject's assertions export as loss-aware FHIR in one Core
//! step that also appends the disclosure record. Synthetic fixtures only.

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_core::CliSession;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic/fhir/r4/presentation")
        .join(name)
}

#[test]
fn subject_export_is_loss_aware_and_always_disclosed() {
    let dir = std::env::temp_dir().join(format!("medscale-021-export-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = CliSession::connect("export-021").unwrap();
    s.open_synthetic_vault(&dir.display().to_string()).unwrap();
    for (file, kind) in [
        ("observation-hr.json", "observation"),
        ("condition-uri.json", "condition"),
    ] {
        let path = fixture(file);
        let source = s.ingest_fhir_file(&path.display().to_string()).unwrap();
        let resource: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        s.promote_fixture_as_assertion(source, "synthetic-subject-export", kind, resource)
            .unwrap();
    }
    let before = s.disclosures().unwrap().len();

    let (exports, record) = s
        .export_subject_fhir(
            OpaqueId::new("synthetic-subject-export"),
            "synthetic interchange test".to_owned(),
        )
        .unwrap();
    assert_eq!(exports.len(), 2);
    assert!(exports.iter().all(|e| !e.full_conformance_claimed));
    assert_eq!(record.artifact_refs.len(), 2);
    assert_eq!(record.scope, "fhir_r4_loss_aware_export");
    assert!(record.export_digest.is_some());
    assert!(record.synthetic_only && !record.release_ready_claimed);
    // The disclosure is on record.
    let after = s.disclosures().unwrap();
    assert_eq!(after.len(), before + 1);
    assert!(
        after
            .iter()
            .any(|d| d.disclosure_id == record.disclosure_id)
    );

    // No assertions: nothing to export, and nothing is disclosed.
    let none = s.export_subject_fhir(OpaqueId::new("nobody"), "test".to_owned());
    assert!(matches!(none, Err(AuthorityError::NotFound)), "{none:?}");
    // An export states its purpose.
    let blank = s.export_subject_fhir(OpaqueId::new("synthetic-subject-export"), "  ".to_owned());
    assert!(
        matches!(blank, Err(AuthorityError::InvalidArgument { .. })),
        "{blank:?}"
    );
    assert_eq!(s.disclosures().unwrap().len(), before + 1);
}
