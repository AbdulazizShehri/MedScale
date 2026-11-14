//! Spec 026 pack signer + anti-rollback tests.

use medscale_contracts::packs::PackAdmitReason;
use medscale_keys::{SYNTHETIC_PACK_TRUST_ROOT_ID, pack_signing_payload, sign_pack_payload};
use medscale_pack::{PackStore, admit_pack_dir};
use std::path::PathBuf;

fn signed_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../evidence/026-pack-signer-os-sandbox/fixtures/pack-fixture-signed-v1")
}

fn ner_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../evidence/008-local-ai-capability-fabric/fixtures/pack-fixture-ner-v0")
}

#[test]
fn admits_signed_synthetic_pack() {
    let m = admit_pack_dir(&signed_fixture()).expect("admit");
    assert_eq!(m.pack_id.as_str(), "pack-fixture-signed-v1");
    assert_eq!(m.pack_epoch, 1);
    assert_eq!(m.trust_root_id, SYNTHETIC_PACK_TRUST_ROOT_ID);
}

#[test]
fn admits_updated_008_fixture_with_signer() {
    let m = admit_pack_dir(&ner_fixture()).expect("admit");
    assert_eq!(m.pack_id.as_str(), "pack-fixture-ner-v0");
    assert_eq!(m.pack_epoch, 1);
}

#[test]
fn rejects_tampered_signature() {
    let dir = tempfile_signed_copy("bad-sig");
    let manifest_path = dir.join("pack.manifest.json");
    let mut raw: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    raw["signature_hex"] = serde_json::Value::String("00".repeat(64));
    std::fs::write(&manifest_path, serde_json::to_vec_pretty(&raw).unwrap()).unwrap();
    let err = admit_pack_dir(&dir).unwrap_err();
    assert_eq!(err.reason(), PackAdmitReason::InvalidSignature);
}

#[test]
fn rejects_unknown_trust_root() {
    let dir = tempfile_signed_copy("bad-root");
    let manifest_path = dir.join("pack.manifest.json");
    let mut raw: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    raw["trust_root_id"] = serde_json::Value::String("not-a-root".into());
    // Resign would still fail trust root id check before/after verify.
    let payload = pack_signing_payload(
        "pack-fixture-signed-v1",
        "0.1.0",
        1,
        raw["content_digest"].as_str().unwrap(),
        raw["rights_uri"].as_str().unwrap(),
        raw["sbom_ref"].as_str().unwrap(),
    );
    raw["signature_hex"] = serde_json::Value::String(sign_pack_payload(&payload));
    std::fs::write(&manifest_path, serde_json::to_vec_pretty(&raw).unwrap()).unwrap();
    let err = admit_pack_dir(&dir).unwrap_err();
    assert_eq!(err.reason(), PackAdmitReason::UnknownTrustRoot);
}

#[test]
fn anti_rollback_rejects_lower_epoch() {
    let m = admit_pack_dir(&signed_fixture()).unwrap();
    let mut store = PackStore::new();
    store.admit(m.clone()).unwrap();

    let mut lower = m;
    lower.pack_epoch = 0;
    lower.version = "0.0.1".into();
    let payload = pack_signing_payload(
        lower.pack_id.as_str(),
        &lower.version,
        lower.pack_epoch,
        &lower.content_digest.to_hex(),
        &lower.rights_uri,
        &lower.sbom_ref,
    );
    lower.signature_hex = sign_pack_payload(&payload);
    let err = store.admit(lower).unwrap_err();
    assert_eq!(err.reason(), PackAdmitReason::AntiRollback);
}

#[test]
fn anti_rollback_allows_higher_epoch() {
    let m = admit_pack_dir(&signed_fixture()).unwrap();
    let mut store = PackStore::new();
    store.admit(m.clone()).unwrap();

    let mut higher = m;
    higher.pack_epoch = 2;
    higher.version = "0.2.0".into();
    let payload = pack_signing_payload(
        higher.pack_id.as_str(),
        &higher.version,
        higher.pack_epoch,
        &higher.content_digest.to_hex(),
        &higher.rights_uri,
        &higher.sbom_ref,
    );
    higher.signature_hex = sign_pack_payload(&payload);
    store.admit(higher).unwrap();
    assert_eq!(store.list()[0].pack_epoch, 2);
}

fn tempfile_signed_copy(tag: &str) -> PathBuf {
    let src = signed_fixture();
    let dest = std::env::temp_dir().join(format!("medscale-026-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::copy(src.join("payload.txt"), dest.join("payload.txt")).unwrap();
    std::fs::copy(
        src.join("pack.manifest.json"),
        dest.join("pack.manifest.json"),
    )
    .unwrap();
    dest
}
