use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::ingest::IngestOutcome;
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::CoreFacade;
use std::fs;
use std::path::PathBuf;

fn tmp_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("medscale-h0a-{name}-{}", std::process::id()));
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
        OpaqueId::new("req"),
        VaultId::new("vault-1"),
        RealmId::new("realm"),
        AuthorityScopeId::new("scope"),
        capability,
        body,
    )
}

fn open_leased(facade: &CoreFacade, root: &str) {
    assert!(
        facade
            .dispatch(req(
                Capability::AcquireLease,
                RequestBody::AcquireLease {
                    client_id: OpaqueId::new("client"),
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
                    vault_root: root.to_owned(),
                },
            ))
            .result
            .is_ok()
    );
}

#[test]
fn lexical_fhir_ingest_accept_and_duplicate() {
    let root = tmp_dir("ingest");
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    open_leased(&facade, root.to_str().unwrap());
    let bytes = fs::read(fixture("fixtures/synthetic/fhir/r4/valid/patient-min.json")).unwrap();
    let accepted = facade.dispatch(req(
        Capability::IngestFhirSynthetic,
        RequestBody::IngestFhirSynthetic {
            media_type: "application/fhir+json".to_owned(),
            bytes: bytes.clone(),
            fhir_version_hint: Some("4.0.1".to_owned()),
            attach_validator_fixture_id: Some("patient-min".to_owned()),
        },
    ));
    match accepted.result.unwrap() {
        ResponseBody::Ingested { receipt } => {
            assert_eq!(receipt.outcome, IngestOutcome::Accepted);
            assert!(!receipt.identity_refs.is_empty());
            assert!(!receipt.evaluation_refs.is_empty());
        }
        other => panic!("{other:?}"),
    }

    let dup = facade.dispatch(req(
        Capability::IngestFhirSynthetic,
        RequestBody::IngestFhirSynthetic {
            media_type: "application/fhir+json".to_owned(),
            bytes,
            fhir_version_hint: None,
            attach_validator_fixture_id: None,
        },
    ));
    match dup.result.unwrap() {
        ResponseBody::Ingested { receipt } => {
            assert_eq!(receipt.outcome, IngestOutcome::Duplicate);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn duplicate_key_reject() {
    let root = tmp_dir("dup");
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    open_leased(&facade, root.to_str().unwrap());
    let bytes = fs::read(fixture(
        "fixtures/synthetic/fhir/r4/adversarial/duplicate-key.json",
    ))
    .unwrap();
    let resp = facade.dispatch(req(
        Capability::IngestFhirSynthetic,
        RequestBody::IngestFhirSynthetic {
            media_type: "application/fhir+json".to_owned(),
            bytes,
            fhir_version_hint: None,
            attach_validator_fixture_id: None,
        },
    ));
    assert!(matches!(
        resp.result,
        Err(AuthorityError::LexicalReject { .. })
    ));
}

#[test]
fn backup_restore_roundtrip() {
    let root = tmp_dir("vault");
    let backup = tmp_dir("backup");
    let restored = tmp_dir("restored");
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    open_leased(&facade, root.to_str().unwrap());
    let bytes = fs::read(fixture("fixtures/synthetic/fhir/r4/valid/patient-min.json")).unwrap();
    assert!(
        facade
            .dispatch(req(
                Capability::IngestFhirSynthetic,
                RequestBody::IngestFhirSynthetic {
                    media_type: "application/fhir+json".to_owned(),
                    bytes,
                    fhir_version_hint: None,
                    attach_validator_fixture_id: None,
                },
            ))
            .result
            .is_ok()
    );
    let bak = facade.dispatch(req(
        Capability::BackupVault,
        RequestBody::BackupVault {
            destination: backup.to_str().unwrap().to_owned(),
        },
    ));
    assert!(matches!(bak.result, Ok(ResponseBody::Backup { .. })));
    let rst = facade.dispatch(req(
        Capability::RestoreVault,
        RequestBody::RestoreVault {
            source: backup.to_str().unwrap().to_owned(),
            destination: restored.to_str().unwrap().to_owned(),
        },
    ));
    match rst.result.unwrap() {
        ResponseBody::Restored {
            sources_restored,
            blobs_restored,
        } => {
            assert!(sources_restored >= 1);
            assert!(blobs_restored >= 1);
        }
        other => panic!("{other:?}"),
    }
}
