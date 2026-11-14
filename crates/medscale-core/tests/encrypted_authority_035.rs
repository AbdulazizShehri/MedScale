//! Spec 035 EncryptedVault authority graph sync (Q02/Q03 residual).

use medscale_contracts::actions::CreateExternalActionIntentRequest;
use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, EffectState, OpaqueId, RealmId, VaultId,
};
use medscale_core::{CoreFacade, build_doctor_report, privacy_proof_artifact_present};
use std::fs;
use std::path::PathBuf;

fn tmp_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("medscale-035-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
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

fn acquire(facade: &CoreFacade) {
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
}

#[test]
fn doctor_encrypted_authority_sync_axis() {
    let report = build_doctor_report(None, false, privacy_proof_artifact_present());
    assert!(report.vault_privacy.encrypted_authority_sync_qualified);
    assert!(!report.vault_privacy.private_data_ready);
}

#[test]
fn encrypted_vault_authority_survives_reopen_035() {
    let root = tmp_dir("enc-auth");
    let root_s = root.to_str().unwrap();
    let passphrase = "spec-035-synthetic-passphrase";
    let digest = DigestSha256::of(b"spec-035-payload");

    let action_id = {
        let facade = CoreFacade::new_legacy_lease_only_engineering();
        acquire(&facade);
        let created = facade.dispatch(req(
            Capability::CreateEncryptedVault,
            RequestBody::CreateEncryptedVault {
                vault_root: root_s.to_owned(),
                passphrase: passphrase.to_owned(),
            },
        ));
        assert!(created.result.is_ok(), "{created:?}");
        let intent = facade.dispatch(req(
            Capability::CreateExternalActionIntent,
            RequestBody::CreateExternalActionIntent {
                request: CreateExternalActionIntentRequest {
                    actor: OpaqueId::new("actor"),
                    action: "claim_submit".to_owned(),
                    target_refs: vec![],
                    payload_digest: digest.clone(),
                },
            },
        ));
        let action_id = match intent.result.unwrap() {
            ResponseBody::Created { object_id } => object_id,
            other => panic!("{other:?}"),
        };
        assert!(
            facade
                .dispatch(req(
                    Capability::TransitionEffect,
                    RequestBody::TransitionEffect {
                        action_id: action_id.clone(),
                        to: EffectState::Sent,
                        reconcile_token: None,
                    },
                ))
                .result
                .is_ok()
        );
        assert!(
            facade
                .dispatch(req(
                    Capability::CloseEncryptedVault,
                    RequestBody::CloseEncryptedVault,
                ))
                .result
                .is_ok()
        );
        action_id
    };

    let facade = CoreFacade::new_legacy_lease_only_engineering();
    acquire(&facade);
    let opened = facade.dispatch(req(
        Capability::OpenEncryptedVault,
        RequestBody::OpenEncryptedVault {
            vault_root: root_s.to_owned(),
            passphrase: Some(passphrase.to_owned()),
            recovery_code: None,
            discard_leftover: false,
        },
    ));
    assert!(opened.result.is_ok(), "{opened:?}");
    let listed = facade.dispatch(req(Capability::ListOutbox, RequestBody::ListOutbox));
    let ResponseBody::Outbox { entries } = listed.result.unwrap() else {
        panic!("expected outbox");
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action_id, action_id);
    assert_eq!(entries[0].effect_state, EffectState::Sent);
    assert_eq!(entries[0].payload_digest, digest);
    let _ = facade.dispatch(req(
        Capability::CloseEncryptedVault,
        RequestBody::CloseEncryptedVault,
    ));
    let _ = fs::remove_dir_all(&root);
}
