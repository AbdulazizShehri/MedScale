//! Spec 020 FHIR interchange qualification (Trusted V1 Q08).

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::fhir::{FhirSupportMatrix, SupportLevel, loss_aware_export};
use medscale_contracts::objects::{AuthorityScopeId, LossClass, OpaqueId, RealmId, VaultId};
use medscale_contracts::{AUTHORITY_SCHEMA_VERSION, FHIR_R4_VERSION};
use medscale_core::{CoreFacade, build_doctor_report};
use serde_json::json;

fn req(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new("req"),
        VaultId::new("vault-1"),
        RealmId::new("realm-a"),
        AuthorityScopeId::new("scope-a"),
        capability,
        body,
    )
}

#[test]
fn matrix_honesty_axes_separated() {
    let m = FhirSupportMatrix::trusted_v1_ready_base();
    assert!(m.is_honest_ready_base());
    assert_eq!(m.fhir_version, FHIR_R4_VERSION);
    assert!(!m.full_conformance_claimed);
    assert!(!m.release_ready);
    assert!(!m.validator_is_authority);

    let patient = m.status_for("Patient");
    assert_eq!(patient.lexical, SupportLevel::Qualified);
    assert_eq!(patient.structural, SupportLevel::Partial);
    assert_eq!(patient.profile, SupportLevel::Unsupported);
    assert_eq!(patient.terminology, SupportLevel::Unsupported);
    assert_eq!(patient.references, SupportLevel::Unsupported);
    assert_eq!(patient.provenance, SupportLevel::Partial);
    assert_eq!(patient.clinical_interpretation, SupportLevel::Unsupported);

    let observation = m.status_for("Observation");
    assert_eq!(observation.terminology, SupportLevel::Partial);
    assert_eq!(observation.structural, SupportLevel::Partial);

    let condition = m.status_for("Condition");
    assert_eq!(condition.structural, SupportLevel::Partial);

    let other = m.status_for("MedicationRequest");
    assert_eq!(other.structural, SupportLevel::Unsupported);
    assert_eq!(other.clinical_interpretation, SupportLevel::Unsupported);
}

#[test]
fn doctor_embeds_honest_matrix() {
    let report = build_doctor_report(None, false, false);
    assert!(report.fhir_interchange.present);
    assert!(report.fhir_interchange.ready_base);
    assert!(!report.fhir_interchange.full_conformance_claimed);
    assert!(!report.fhir_interchange.validator_is_authority);
    assert!(!report.fhir_interchange.release_ready);
    assert!(report.fhir_support_matrix.is_honest_ready_base());
    assert_eq!(report.fhir_support_matrix.resources.len(), 3);
}

#[test]
fn facade_get_fhir_support_matrix() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let resp = facade.dispatch(req(
        Capability::GetFhirSupportMatrix,
        RequestBody::GetFhirSupportMatrix,
    ));
    match resp.result.expect("matrix") {
        ResponseBody::FhirSupportMatrix { matrix } => {
            assert!(matrix.is_honest_ready_base());
            assert_eq!(matrix.fhir_version, FHIR_R4_VERSION);
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn loss_aware_export_stub_documents_losses() {
    let resource = json!({
        "resourceType": "Observation",
        "id": "o1",
        "status": "final",
        "code": {"coding": [{"code": "8867-4"}]},
        "valueQuantity": {"value": 72, "unit": "/min", "code": "/min"},
        "interpretation": [{"text": "synthetic"}],
        "note": [{"text": "unsupported on READY_BASE export"}]
    });
    let export = loss_aware_export(&resource);
    assert!(!export.full_conformance_claimed);
    assert!(export.exported_json.get("valueQuantity").is_some());
    assert!(export.exported_json.get("interpretation").is_none());
    assert!(
        export
            .unsupported_fields_preserved
            .iter()
            .any(|p| p == "Observation.interpretation")
    );
    assert!(
        export
            .field_losses
            .iter()
            .any(|l| { l.fhir_path == "Observation.note" && l.loss_class == LossClass::Lossy })
    );
    assert!(export.field_losses.iter().any(|l| {
        l.fhir_path.contains("clinical_interpretation") && l.loss_class == LossClass::Unknown
    }));
}

#[test]
fn facade_export_fhir_loss_aware() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let resource = json!({
        "resourceType": "Patient",
        "id": "p1",
        "birthDate": "1990",
        "gender": "unknown",
        "address": [{"city": "Synthetic"}]
    });
    let resp = facade.dispatch(req(
        Capability::ExportFhirLossAware,
        RequestBody::ExportFhirLossAware { resource },
    ));
    match resp.result.expect("export") {
        ResponseBody::FhirLossAwareExport { export } => {
            assert_eq!(export.resource_type, "Patient");
            assert!(!export.full_conformance_claimed);
            assert!(
                export
                    .unsupported_fields_preserved
                    .contains(&"Patient.gender".to_owned())
            );
            assert!(
                export
                    .unsupported_fields_preserved
                    .contains(&"Patient.address".to_owned())
            );
            assert!(export.exported_json.get("birthDate").is_some());
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn capability_mismatch_rejected() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let mut bad = req(Capability::Ping, RequestBody::GetFhirSupportMatrix);
    bad.schema_version = AUTHORITY_SCHEMA_VERSION;
    let resp = facade.dispatch(bad);
    assert!(resp.result.is_err());
}
