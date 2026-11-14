//! Spec 016 durable trusted-record qualification (restart / writer / failure).

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::CoreFacade;
use medscale_storage::{SqliteMetaStore, WriterLock};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("medscale-016-{name}-{}", std::process::id()));
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
    let opened = facade.dispatch(req(
        Capability::OpenSyntheticVault,
        RequestBody::OpenSyntheticVault {
            vault_root: root.to_owned(),
        },
    ));
    assert!(opened.result.is_ok(), "{opened:?}");
}

fn write_parent_vault(root: &Path) -> (String, String, usize) {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    open_leased(&facade, root.to_str().unwrap());
    let bytes = fs::read(fixture("fixtures/synthetic/fhir/r4/valid/patient-min.json")).unwrap();
    let accepted = facade.dispatch(req(
        Capability::IngestFhirSynthetic,
        RequestBody::IngestFhirSynthetic {
            media_type: "application/fhir+json".to_owned(),
            bytes,
            fhir_version_hint: Some("4.0.1".to_owned()),
            attach_validator_fixture_id: Some("patient-min".to_owned()),
        },
    ));
    let (source_id, digest_hex) = match accepted.result.unwrap() {
        ResponseBody::Ingested { receipt } => (
            receipt.source_id.unwrap().as_str().to_owned(),
            receipt.content_digest.unwrap().to_hex(),
        ),
        other => panic!("{other:?}"),
    };
    let proposal = facade.dispatch(req(
        Capability::CreateProposal,
        RequestBody::CreateProposal {
            subject_ref: Some(OpaqueId::new("subj")),
            claim_kind: "note".to_owned(),
            payload: serde_json::json!({"text": "synthetic"}),
            evidence_refs: vec![OpaqueId::new(source_id.clone())],
        },
    ));
    let proposal_id = match proposal.result.unwrap() {
        ResponseBody::Created { object_id } => object_id,
        other => panic!("{other:?}"),
    };
    assert!(
        facade
            .dispatch(req(
                Capability::PromoteProposal,
                RequestBody::PromoteProposal {
                    proposal_id,
                    authorized_by: OpaqueId::new("actor"),
                    subject_ref: OpaqueId::new("subj"),
                },
            ))
            .result
            .is_ok()
    );
    assert!(
        facade
            .dispatch(req(Capability::CloseVault, RequestBody::CloseVault))
            .result
            .is_ok()
    );
    let meta = SqliteMetaStore::open(root).expect("meta open for count");
    let object_count = meta.list_authority_objects().unwrap().len();
    assert!(
        object_count >= 3,
        "expected durable objects, got {object_count}"
    );
    drop(meta);
    (source_id, digest_hex, object_count)
}

fn verify_child_vault(root: &Path, source_id: &str, digest_hex: &str, min_objects: usize) {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    open_leased(&facade, root.to_str().unwrap());
    let meta = SqliteMetaStore::open(root).expect("child meta");
    let objects = meta.list_authority_objects().unwrap();
    assert!(
        objects.len() >= min_objects,
        "child objects {} < {min_objects}",
        objects.len()
    );
    assert!(
        objects.iter().any(|o| o.object_id == source_id),
        "missing source object"
    );
    let source = objects
        .iter()
        .find(|o| o.object_id == source_id)
        .expect("source row");
    assert_eq!(
        source.content_digest_hex.as_deref(),
        Some(digest_hex),
        "digest mismatch across process"
    );
    // Empty-memory facade without open cannot see objects — already using new CoreFacade.
    let _ = facade.dispatch(req(Capability::CloseVault, RequestBody::CloseVault));
    drop(meta);
}

#[test]
fn durable_restart_two_process_016() {
    if let Ok(root) = std::env::var("MEDSCALE_016_VAULT") {
        let source_id = std::env::var("MEDSCALE_016_SOURCE").unwrap();
        let digest = std::env::var("MEDSCALE_016_DIGEST").unwrap();
        let min_objects: usize = std::env::var("MEDSCALE_016_OBJECTS")
            .unwrap()
            .parse()
            .unwrap();
        verify_child_vault(Path::new(&root), &source_id, &digest, min_objects);
        return;
    }

    let root = tmp_dir("restart");
    let (source_id, digest_hex, object_count) = write_parent_vault(&root);

    let exe = std::env::current_exe().expect("current exe");
    let status = Command::new(&exe)
        .env("MEDSCALE_016_VAULT", root.to_str().unwrap())
        .env("MEDSCALE_016_SOURCE", &source_id)
        .env("MEDSCALE_016_DIGEST", &digest_hex)
        .env("MEDSCALE_016_OBJECTS", object_count.to_string())
        .env("RUST_TEST_THREADS", "1")
        .args(["durable_restart_two_process_016", "--exact", "--nocapture"])
        .status()
        .expect("spawn child");
    assert!(status.success(), "child process failed: {status}");
}

#[test]
fn writer_lock_second_process_denied_016() {
    let root = tmp_dir("writer");
    let _lock = WriterLock::try_acquire(&root).expect("first lock");
    let err = WriterLock::try_acquire(&root).expect_err("second must fail");
    assert!(err.to_string().contains("writer held"), "unexpected {err}");
}

#[test]
fn digest_mismatch_refuses_016() {
    let root = tmp_dir("digest");
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    open_leased(&facade, root.to_str().unwrap());
    let bytes = fs::read(fixture("fixtures/synthetic/fhir/r4/valid/patient-min.json")).unwrap();
    let accepted = facade.dispatch(req(
        Capability::IngestFhirSynthetic,
        RequestBody::IngestFhirSynthetic {
            media_type: "application/fhir+json".to_owned(),
            bytes: bytes.clone(),
            fhir_version_hint: Some("4.0.1".to_owned()),
            attach_validator_fixture_id: None,
        },
    ));
    let digest = match accepted.result.unwrap() {
        ResponseBody::Ingested { receipt } => receipt.content_digest.unwrap(),
        other => panic!("{other:?}"),
    };
    assert!(
        facade
            .dispatch(req(Capability::CloseVault, RequestBody::CloseVault))
            .result
            .is_ok()
    );

    // Corrupt blob bytes after close.
    let blob_path = root.join("blobs").join(digest.to_hex());
    fs::write(&blob_path, b"corrupted-not-original-bytes").unwrap();

    let facade2 = CoreFacade::new_legacy_lease_only_engineering();
    let opened = {
        assert!(
            facade2
                .dispatch(req(
                    Capability::AcquireLease,
                    RequestBody::AcquireLease {
                        client_id: OpaqueId::new("client2"),
                        holder_id_hint: None,
                    },
                ))
                .result
                .is_ok()
        );
        facade2.dispatch(req(
            Capability::OpenSyntheticVault,
            RequestBody::OpenSyntheticVault {
                vault_root: root.to_str().unwrap().to_owned(),
            },
        ))
    };
    assert!(
        opened.result.is_err(),
        "corrupt blob must refuse open/load, got {opened:?}"
    );
}

#[test]
fn same_process_reopen_retains_objects_016() {
    let root = tmp_dir("reopen");
    let (source_id, digest_hex, object_count) = write_parent_vault(&root);
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    open_leased(&facade, root.to_str().unwrap());
    let meta = SqliteMetaStore::open(&root).unwrap();
    assert!(meta.list_authority_objects().unwrap().len() >= object_count);
    assert!(
        meta.list_authority_objects()
            .unwrap()
            .iter()
            .any(|o| o.object_id == source_id
                && o.content_digest_hex.as_deref() == Some(digest_hex.as_str()))
    );
}
