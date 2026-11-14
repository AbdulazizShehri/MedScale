//! Spec 036 MESC synthetic verifier READY_BASE (does not clear MESC_RELEASED_ARTIFACT).
//!
//! Fixtures are written at runtime (LF bytes) so Windows checkout EOL cannot break digests.
//! Each test uses a unique temp directory so parallel cargo tests do not race.

use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::external_artifact::{
    MescAdmissionState, MescArtifactAdmitRequest, MescArtifactVerifyRequest, MescVerifyReason,
};
use medscale_contracts::objects::{AuthorityScopeId, DigestSha256, OpaqueId, RealmId, VaultId};
use medscale_core::{CoreFacade, build_doctor_report};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

fn req(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new("req"),
        VaultId::new("v"),
        RealmId::new("r"),
        AuthorityScopeId::new("s"),
        capability,
        body,
    )
}

fn tmp_root(label: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "mesc-036-it-{}-{}-{}",
        label,
        std::process::id(),
        n
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_good(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    let model = b"synthetic-model-bytes-v1";
    let sbom = br#"{"bomFormat":"CycloneDX","specVersion":"1.5","components":[]}"#;
    let notice = b"Synthetic NOTICE - no redistribution rights claimed.\n";
    fs::write(dir.join("model.bin"), model).unwrap();
    fs::write(dir.join("sbom.json"), sbom).unwrap();
    fs::write(dir.join("NOTICE"), notice).unwrap();
    let manifest = serde_json::json!({
        "schema_version": 1,
        "producer_id": "synthetic.medscale.mesc",
        "release_id": "syn-036-001",
        "release_tag": "syn-v0.0.1",
        "source_commit": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "source_tree": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "model_id": "syn-model",
        "tokenizer_id": "syn-tok",
        "base_model_id": "syn-base",
        "corpus_id": "syn-corpus",
        "training_receipt_digest_hex": DigestSha256::of(b"train-receipt").to_hex(),
        "evaluation_receipt_digest_hex": DigestSha256::of(b"eval-receipt").to_hex(),
        "sbom_path": "sbom.json",
        "sbom_digest_hex": DigestSha256::of(sbom).to_hex(),
        "rights_license": "SYNTHETIC-NO-RIGHTS",
        "rights_notice_path": "NOTICE",
        "provenance_note": "synthetic fixture for Spec 036; not a real MESC release",
        "limitations": ["not a real MESC release", "product admit remains gate-blocked"],
        "runtime_requirements": "offline-fixture-only",
        "epoch": 1,
        "artifacts": [
            {
                "kind": "model_weights",
                "path": "model.bin",
                "byte_length": model.len() as u64,
                "sha256_hex": DigestSha256::of(model).to_hex()
            },
            {
                "kind": "sbom",
                "path": "sbom.json",
                "byte_length": sbom.len() as u64,
                "sha256_hex": DigestSha256::of(sbom).to_hex()
            },
            {
                "kind": "notice",
                "path": "NOTICE",
                "byte_length": notice.len() as u64,
                "sha256_hex": DigestSha256::of(notice).to_hex()
            }
        ]
    });
    fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
}

fn clone_good(root: &Path, name: &str) -> PathBuf {
    let src = root.join("synthetic-good");
    let dst = root.join(name);
    fs::create_dir_all(&dst).unwrap();
    for entry in fs::read_dir(&src).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), dst.join(entry.file_name())).unwrap();
    }
    dst
}

fn prepare_adversarial(root: &Path) {
    write_good(&root.join("synthetic-good"));

    clone_good(root, "missing-manifest");
    fs::remove_file(root.join("missing-manifest/manifest.json")).unwrap();

    let d = clone_good(root, "digest-mismatch");
    fs::write(
        d.join("model.bin"),
        b"X".repeat(b"synthetic-model-bytes-v1".len()),
    )
    .unwrap();

    let d = clone_good(root, "size-mismatch");
    let mut m: serde_json::Value =
        serde_json::from_slice(&fs::read(d.join("manifest.json")).unwrap()).unwrap();
    m["artifacts"][0]["byte_length"] = serde_json::json!(1);
    fs::write(
        d.join("manifest.json"),
        serde_json::to_vec_pretty(&m).unwrap(),
    )
    .unwrap();

    clone_good(root, "missing-file");
    fs::remove_file(root.join("missing-file/model.bin")).unwrap();

    let d = clone_good(root, "bad-schema");
    let mut m: serde_json::Value =
        serde_json::from_slice(&fs::read(d.join("manifest.json")).unwrap()).unwrap();
    m["schema_version"] = serde_json::json!(99);
    fs::write(
        d.join("manifest.json"),
        serde_json::to_vec_pretty(&m).unwrap(),
    )
    .unwrap();

    let d = clone_good(root, "unknown-field");
    let mut m: serde_json::Value =
        serde_json::from_slice(&fs::read(d.join("manifest.json")).unwrap()).unwrap();
    m["evil_extra"] = serde_json::json!(true);
    fs::write(
        d.join("manifest.json"),
        serde_json::to_vec_pretty(&m).unwrap(),
    )
    .unwrap();

    let d = clone_good(root, "duplicate-path");
    let mut m: serde_json::Value =
        serde_json::from_slice(&fs::read(d.join("manifest.json")).unwrap()).unwrap();
    let first = m["artifacts"][0].clone();
    m["artifacts"].as_array_mut().unwrap().push(first);
    fs::write(
        d.join("manifest.json"),
        serde_json::to_vec_pretty(&m).unwrap(),
    )
    .unwrap();

    clone_good(root, "missing-rights");
    fs::remove_file(root.join("missing-rights/NOTICE")).unwrap();

    clone_good(root, "missing-sbom");
    fs::remove_file(root.join("missing-sbom/sbom.json")).unwrap();
}

fn verify(facade: &CoreFacade, path: &Path) -> ResponseBody {
    let out = facade.dispatch(req(
        Capability::MescArtifactVerify,
        RequestBody::MescArtifactVerify {
            request: MescArtifactVerifyRequest {
                release_dir: path.to_string_lossy().into_owned(),
            },
        },
    ));
    out.result.expect("dispatch ok")
}

#[test]
fn synthetic_good_verifies_without_product_admit() {
    let root = tmp_root("good");
    prepare_adversarial(&root);
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    match verify(&facade, &root.join("synthetic-good")) {
        ResponseBody::MescVerify { report } => {
            assert_eq!(report.state, MescAdmissionState::Verified);
            assert_eq!(report.reason, MescVerifyReason::Ok);
            assert!(!report.product_admit_authorized);
            assert_eq!(
                report.producer_id.as_deref(),
                Some("synthetic.medscale.mesc")
            );
        }
        other => panic!("unexpected {other:?}"),
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn adversarial_fixtures_reject_with_stable_reasons() {
    let root = tmp_root("adversarial");
    prepare_adversarial(&root);
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let cases = [
        ("missing-manifest", MescVerifyReason::MissingManifest),
        ("digest-mismatch", MescVerifyReason::DigestMismatch),
        ("size-mismatch", MescVerifyReason::SizeMismatch),
        ("missing-file", MescVerifyReason::MissingArtifactFile),
        ("bad-schema", MescVerifyReason::UnsupportedSchema),
        ("unknown-field", MescVerifyReason::UnsupportedSchema),
        ("duplicate-path", MescVerifyReason::DuplicateArtifactPath),
        ("missing-rights", MescVerifyReason::MissingRights),
        ("missing-sbom", MescVerifyReason::MissingSbom),
    ];
    for (dir, reason) in cases {
        match verify(&facade, &root.join(dir)) {
            ResponseBody::MescVerify { report } => {
                assert_eq!(report.state, MescAdmissionState::Rejected, "{dir}");
                assert_eq!(report.reason, reason, "{dir}");
                assert!(!report.product_admit_authorized);
            }
            other => panic!("{dir}: unexpected {other:?}"),
        }
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn replay_rejects_second_identical_verified_epoch() {
    let root = tmp_root("replay");
    prepare_adversarial(&root);
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    match verify(&facade, &root.join("synthetic-good")) {
        ResponseBody::MescVerify { report } => {
            assert_eq!(report.reason, MescVerifyReason::Ok);
        }
        other => panic!("unexpected {other:?}"),
    }
    match verify(&facade, &root.join("synthetic-good")) {
        ResponseBody::MescVerify { report } => {
            assert_eq!(report.reason, MescVerifyReason::ReplayRejected);
            assert!(!report.product_admit_authorized);
        }
        other => panic!("unexpected {other:?}"),
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn admit_still_gate_blocked_after_synthetic_verify() {
    let root = tmp_root("admit");
    prepare_adversarial(&root);
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let _ = verify(&facade, &root.join("synthetic-good"));
    let out = facade.dispatch(req(
        Capability::MescArtifactAdmit,
        RequestBody::MescArtifactAdmit {
            request: MescArtifactAdmitRequest {
                artifact_uri: "synthetic://not-real".to_owned(),
                content_digest: DigestSha256::of(b"x"),
                rights_uri: "synthetic://rights".to_owned(),
                sbom_digest: DigestSha256::of(b"s"),
                evaluation_digest: DigestSha256::of(b"e"),
                pack_path_required: true,
            },
        },
    ));
    assert_eq!(
        out.result,
        Err(AuthorityError::ExternalGateRequired {
            gate: "MESC_RELEASED_ARTIFACT".to_owned()
        })
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn doctor_reports_verifier_ready_base_without_admit() {
    let report = build_doctor_report(None, false, false);
    assert!(report.mesc_artifact.verifier_ready_base);
    assert!(!report.mesc_artifact.artifact_admitted);
    assert!(report.mesc_artifact.is_honest_ready_base());
}
