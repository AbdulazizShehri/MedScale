//! Spec 109: through Core, a failed seal leaves the vault open and retryable,
//! and an open that discards an unsealed session reports it. Synthetic only.

use medscale_contracts::actions::CreateExternalActionIntentRequest;
use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::objects::{AuthorityScopeId, DigestSha256, OpaqueId, RealmId, VaultId};
use medscale_core::{CliSession, CoreFacade};
use std::fs;
use std::path::{Path, PathBuf};

const PASS: &str = "passphrase-ok";

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-109c-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn req(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new("req"),
        VaultId::new("vault-109"),
        RealmId::new("realm"),
        AuthorityScopeId::new("scope"),
        capability,
        body,
    )
}

fn connected() -> CoreFacade {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
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
    facade
}

fn add_intent(facade: &CoreFacade, payload: &[u8]) {
    let created = facade.dispatch(req(
        Capability::CreateExternalActionIntent,
        RequestBody::CreateExternalActionIntent {
            request: CreateExternalActionIntentRequest {
                actor: OpaqueId::new("actor"),
                action: "claim_submit".to_owned(),
                target_refs: vec![],
                payload_digest: DigestSha256::of(payload),
            },
        },
    ));
    assert!(created.result.is_ok(), "{created:?}");
}

fn close(facade: &CoreFacade) -> bool {
    facade
        .dispatch(req(
            Capability::CloseEncryptedVault,
            RequestBody::CloseEncryptedVault,
        ))
        .result
        .is_ok()
}

fn open(facade: &CoreFacade, root: &Path) {
    let opened = facade.dispatch(req(
        Capability::OpenEncryptedVault,
        RequestBody::OpenEncryptedVault {
            vault_root: root.display().to_string(),
            passphrase: Some(PASS.to_owned()),
            recovery_code: None,
        },
    ));
    assert!(opened.result.is_ok(), "{opened:?}");
}

#[test]
fn a_failed_close_stays_open_and_a_retry_seals_both_writes() {
    let root = tmp_dir("retry");
    {
        let facade = connected();
        let created = facade.dispatch(req(
            Capability::CreateEncryptedVault,
            RequestBody::CreateEncryptedVault {
                vault_root: root.display().to_string(),
                passphrase: PASS.to_owned(),
            },
        ));
        assert!(created.result.is_ok(), "{created:?}");
        add_intent(&facade, b"before-the-failure");

        fs::create_dir(root.join("meta.sealed.tmp")).unwrap();
        assert!(!close(&facade), "the seal must fail");
        assert!(root.join("meta.work.sqlite3").exists());
        // Only a vault still in the slot can persist this write.
        add_intent(&facade, b"after-the-failure");

        fs::remove_dir(root.join("meta.sealed.tmp")).unwrap();
        assert!(close(&facade), "the retry must seal");
        assert!(!root.join("meta.work.sqlite3").exists());
    }

    let facade = connected();
    open(&facade, &root);
    let listed = facade.dispatch(req(Capability::ListOutbox, RequestBody::ListOutbox));
    let ResponseBody::Outbox { entries } = listed.result.unwrap() else {
        panic!("expected outbox");
    };
    assert_eq!(entries.len(), 2, "both writes were sealed");
    assert!(close(&facade));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn an_unsealed_session_is_reported_on_the_next_open() {
    let root = tmp_dir("crash");
    let root_text = root.display().to_string();
    let mut session = CliSession::connect("v-109").unwrap();
    session.create_encrypted_vault(&root_text, PASS).unwrap();
    session.close_encrypted_vault().unwrap();
    session.open_encrypted_vault(&root_text, PASS).unwrap();
    assert!(!session.discarded_interrupted_session());
    drop(session); // A crash: nothing sealed this session.

    let mut reopened = CliSession::connect("v-109").unwrap();
    reopened.open_encrypted_vault(&root_text, PASS).unwrap();
    assert!(reopened.discarded_interrupted_session());
    reopened.close_encrypted_vault().unwrap();
    fs::remove_dir_all(&root).unwrap();
}
