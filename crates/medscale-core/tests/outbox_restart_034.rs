//! Spec 034 durable outbox restart qualification (Q12 residual).
//!
//! Proves ExternalActionIntent / outbox projection survives SyntheticVault close→reopen.
//! UNKNOWN still requires reconcile; NPHIES remains gated. No live partner transport.

use medscale_contracts::actions::CreateExternalActionIntentRequest;
use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, EffectState, OpaqueId, RealmId, VaultId,
};
use medscale_core::{CoreFacade, build_doctor_report, privacy_proof_artifact_present};
use std::fs;
use std::path::PathBuf;

fn tmp_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("medscale-034-{name}-{}", std::process::id()));
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

#[test]
fn doctor_outbox_restart_axis_honest() {
    let m = medscale_contracts::actions::ControlledActionsDoctorStatus::ready_base();
    assert!(m.is_honest_ready_base());
    assert!(m.outbox_restart_qualified);
    assert!(!m.nphies_authorized);
    assert!(!m.unknown_blind_retry);
    let report = build_doctor_report(None, false, privacy_proof_artifact_present());
    assert!(report.controlled_actions.is_honest_ready_base());
}

#[test]
fn outbox_survives_synthetic_vault_restart_034() {
    let root = tmp_dir("outbox-restart");
    let root_s = root.to_str().unwrap();
    let digest = DigestSha256::of(b"spec-034-approved-payload");

    let action_id = {
        let facade = CoreFacade::new_legacy_lease_only_engineering();
        open_leased(&facade, root_s);
        let created = facade.dispatch(req(
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
        let action_id = match created.result.unwrap() {
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
                    Capability::TransitionEffect,
                    RequestBody::TransitionEffect {
                        action_id: action_id.clone(),
                        to: EffectState::Unknown,
                        reconcile_token: None,
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
        action_id
    };

    // New process-equivalent facade: empty memory until open reloads durable audit intents.
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    open_leased(&facade, root_s);
    let listed = facade.dispatch(req(Capability::ListOutbox, RequestBody::ListOutbox));
    let ResponseBody::Outbox { entries } = listed.result.unwrap() else {
        panic!("expected outbox");
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action_id, action_id);
    assert_eq!(entries[0].effect_state, EffectState::Unknown);
    assert_eq!(entries[0].payload_digest, digest);

    let denied = facade.dispatch(req(
        Capability::TransitionEffect,
        RequestBody::TransitionEffect {
            action_id: action_id.clone(),
            to: EffectState::Pending,
            reconcile_token: None,
        },
    ));
    assert_eq!(denied.result, Err(AuthorityError::UnknownRequiresReconcile));

    assert!(
        facade
            .dispatch(req(
                Capability::TransitionEffect,
                RequestBody::TransitionEffect {
                    action_id,
                    to: EffectState::Pending,
                    reconcile_token: Some("reconciled-after-restart".to_owned()),
                },
            ))
            .result
            .is_ok()
    );

    let listed2 = facade.dispatch(req(Capability::ListOutbox, RequestBody::ListOutbox));
    let ResponseBody::Outbox { entries } = listed2.result.unwrap() else {
        panic!("expected outbox");
    };
    assert_eq!(entries[0].effect_state, EffectState::Pending);

    let _ = facade.dispatch(req(Capability::CloseVault, RequestBody::CloseVault));
    let _ = fs::remove_dir_all(&root);
}
