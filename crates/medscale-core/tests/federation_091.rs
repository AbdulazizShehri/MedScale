//! Spec 091 federation integration tests: a bounded two-institution
//! synthetic scenario. Each institution is its own vault and `CliSession`;
//! bundles move between them as plain strings (out of band). There is no
//! network and no central service.

use medscale_contracts::federation::{
    FederationActRequest, FederationActResult, FederationBundle, FederationRefusal, ImportedState,
};
use medscale_contracts::objects::{DigestSha256, OpaqueId};
use medscale_contracts::privacy_gate::DataClass;
use medscale_core::CliSession;

struct Inst {
    s: CliSession,
    project: OpaqueId,
    key: String,
}

fn institution(test: &str, id: &str) -> Inst {
    let dir =
        std::env::temp_dir().join(format!("medscale-091c-{test}-{id}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = CliSession::connect(&format!("vault-091-{test}-{id}")).unwrap();
    s.open_synthetic_vault(&dir.display().to_string()).unwrap();
    let project = s
        .project_create("shared-study".to_owned(), None)
        .unwrap()
        .header
        .id;
    let key = s
        .federation_act(FederationActRequest::CreateIdentity {
            institution_id: id.to_owned(),
        })
        .unwrap()
        .identity
        .unwrap()
        .public_key_hex;
    Inst { s, project, key }
}

fn trust(a: &mut Inst, peer: &str, key: &str, ceiling: DataClass) {
    a.s.federation_act(FederationActRequest::TrustPeer {
        institution_id: peer.to_owned(),
        public_key_hex: key.to_owned(),
        ceiling,
    })
    .unwrap();
}

fn artifact(i: &mut Inst, bytes: &[u8], class: Option<DataClass>) -> OpaqueId {
    let id =
        i.s.create_source_record("application/json".to_owned(), bytes.to_vec())
            .unwrap();
    if let Some(c) = class {
        i.s.privacy_classify(i.project.clone(), id.clone(), c, None)
            .unwrap();
    }
    id
}

fn export(
    i: &mut Inst,
    to: &str,
    ids: Vec<OpaqueId>,
    tombstones: Vec<DigestSha256>,
) -> FederationActResult {
    let project_id = i.project.clone();
    i.s.federation_act(FederationActRequest::Export {
        project_id,
        to_institution: to.to_owned(),
        artifact_ids: ids,
        tombstones,
    })
    .unwrap()
}

fn import(i: &mut Inst, bundle: &str) -> FederationActResult {
    let project_id = i.project.clone();
    i.s.federation_act(FederationActRequest::Import {
        project_id,
        bundle_json: bundle.to_owned(),
    })
    .unwrap()
}

fn pair(test: &str) -> (Inst, Inst) {
    let mut a = institution(test, "hospital-a");
    let mut b = institution(test, "hospital-b");
    let (ka, kb) = (a.key.clone(), b.key.clone());
    trust(&mut a, "hospital-b", &kb, DataClass::ExternalDeidentified);
    trust(&mut b, "hospital-a", &ka, DataClass::ExternalDeidentified);
    (a, b)
}

#[test]
fn signed_bundles_carry_provenance_and_never_overwrite() {
    let (mut a, mut b) = pair("exchange");
    let art = artifact(
        &mut a,
        br#"{"n":42}"#,
        Some(DataClass::ExternalDeidentified),
    );
    let out = export(&mut a, "hospital-b", vec![art.clone()], Vec::new());
    assert_eq!(out.receipt.refusal, None);
    let bundle = out.bundle_json.unwrap();
    let got = import(&mut b, &bundle);
    assert_eq!(got.receipt.refusal, None);
    assert_eq!(got.imported.len(), 1);
    let item = &got.imported[0];
    assert_eq!(item.from_institution, "hospital-a");
    assert_eq!(item.provenance[0].institution_id, "hospital-a");
    assert_eq!(item.provenance[0].artifact_id, art);
    assert_eq!(item.digest, DigestSha256::of(br#"{"n":42}"#));

    // Replaying the same bundle is refused.
    assert_eq!(
        import(&mut b, &bundle).receipt.refusal,
        Some(FederationRefusal::Replay)
    );
    // A second bundle with the same item adds nothing (no overwrite).
    let again = export(&mut a, "hospital-b", vec![art], Vec::new())
        .bundle_json
        .unwrap();
    let r = import(&mut b, &again);
    assert!(r.receipt.refusal.is_none() && r.imported.is_empty());
    assert_eq!(b.s.federation_get().unwrap().imported.len(), 1);

    // Withdrawal: a tombstone erases the item's bytes at the receiver.
    let tomb = export(&mut a, "hospital-b", Vec::new(), vec![item.digest.clone()])
        .bundle_json
        .unwrap();
    let r = import(&mut b, &tomb);
    assert_eq!(r.receipt.tombstoned, vec![item.digest.clone()]);
    let view = b.s.federation_get().unwrap();
    assert_eq!(view.imported[0].state, ImportedState::Tombstoned);
}

#[test]
fn policy_denies_phi_and_over_ceiling_data() {
    let (mut a, _b) = pair("policy");
    let phi = artifact(&mut a, b"names and mrns", None);
    let team = artifact(&mut a, b"team notes", Some(DataClass::TeamProtected));
    for id in [phi, team] {
        let r = export(&mut a, "hospital-b", vec![id], Vec::new());
        assert_eq!(r.receipt.refusal, Some(FederationRefusal::PolicyDenied));
        assert!(r.bundle_json.is_none());
    }
    // A peer ceiling of local_phi cannot be granted.
    let key = a.key.clone();
    assert!(
        a.s.federation_act(FederationActRequest::TrustPeer {
            institution_id: "hospital-c".to_owned(),
            public_key_hex: key,
            ceiling: DataClass::LocalPhi,
        })
        .is_err()
    );
    // Unknown peers get nothing.
    let public = artifact(&mut a, b"{}", Some(DataClass::Public));
    assert_eq!(
        export(&mut a, "hospital-z", vec![public], Vec::new())
            .receipt
            .refusal,
        Some(FederationRefusal::PeerUnknown)
    );
}

#[test]
fn forged_misaddressed_and_revoked_exchanges_are_refused() {
    let (mut a, mut b) = pair("forgery");
    let mut c = institution("forgery", "hospital-c");
    let kb = b.key.clone();
    trust(&mut c, "hospital-b", &kb, DataClass::ExternalDeidentified);
    let art = artifact(&mut a, b"{}", Some(DataClass::Public));
    let bundle = export(&mut a, "hospital-b", vec![art], Vec::new())
        .bundle_json
        .unwrap();

    // Delivered to the wrong institution.
    assert_eq!(
        import(&mut c, &bundle).receipt.refusal,
        Some(FederationRefusal::WrongRecipient)
    );
    // Body edited after signing.
    let mut edited: FederationBundle = serde_json::from_str(&bundle).unwrap();
    edited.body_json = edited.body_json.replace("\"seq\":1", "\"seq\":9");
    assert_eq!(
        import(&mut b, &serde_json::to_string(&edited).unwrap())
            .receipt
            .refusal,
        Some(FederationRefusal::SignatureInvalid)
    );
    // Garbage.
    assert_eq!(
        import(&mut b, "not a bundle").receipt.refusal,
        Some(FederationRefusal::BundleInvalid)
    );
    // After revocation nothing is accepted from, or sent to, the peer.
    b.s.federation_act(FederationActRequest::RevokePeer {
        institution_id: "hospital-a".to_owned(),
    })
    .unwrap();
    assert_eq!(
        import(&mut b, &bundle).receipt.refusal,
        Some(FederationRefusal::PeerRevoked)
    );
    let back = artifact(&mut b, b"{}", Some(DataClass::Public));
    assert_eq!(
        export(&mut b, "hospital-a", vec![back], Vec::new())
            .receipt
            .refusal,
        Some(FederationRefusal::PeerRevoked)
    );
    let view = b.s.federation_get().unwrap();
    assert!(view.receipts.iter().filter(|r| r.refusal.is_some()).count() >= 4);
}
