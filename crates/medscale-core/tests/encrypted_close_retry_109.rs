//! Spec 109: through Core, a failed seal leaves the vault open and retryable,
//! and an open that discards an unsealed session reports it. Synthetic only.

use medscale_contracts::actions::CreateExternalActionIntentRequest;
use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody, VaultOpenOutcome,
};
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
            discard_leftover: false,
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

fn create_vault(facade: &CoreFacade, root: &Path) {
    let created = facade.dispatch(req(
        Capability::CreateEncryptedVault,
        RequestBody::CreateEncryptedVault {
            vault_root: root.display().to_string(),
            passphrase: PASS.to_owned(),
        },
    ));
    assert!(created.result.is_ok(), "{created:?}");
}

fn intent_number(facade: &CoreFacade, payload: &[u8]) -> u64 {
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
    let ResponseBody::Created { object_id } = created.result.expect("created") else {
        panic!("expected created");
    };
    object_id
        .as_str()
        .rsplit('-')
        .next()
        .and_then(|n| n.parse().ok())
        .expect("a numeric id")
}

#[test]
fn recovery_keeps_committed_work_and_never_reissues_an_id() {
    let root = tmp_dir("recover");
    let before = {
        let facade = connected();
        create_vault(&facade, &root);
        intent_number(&facade, b"before-the-crash")
        // Dropped without closing: a crash.
    };
    let facade = connected();
    open(&facade, &root);
    let listed = facade.dispatch(req(Capability::ListOutbox, RequestBody::ListOutbox));
    let ResponseBody::Outbox { entries } = listed.result.unwrap() else {
        panic!("expected outbox");
    };
    assert_eq!(entries.len(), 1, "the committed write survived");
    let after = intent_number(&facade, b"after-the-crash");
    assert!(
        after > before + (1 << 31),
        "ids jump past anything a lost request could have used: {before} then {after}"
    );
    assert!(close(&facade));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_recovered_session_that_cannot_load_falls_back_to_the_seal() {
    let root = tmp_dir("bad-load");
    {
        let facade = connected();
        create_vault(&facade, &root);
        add_intent(&facade, b"sealed");
        assert!(close(&facade));
        open(&facade, &root);
        add_intent(&facade, b"unsealed");
    }
    // A row that can't load, in a work file that otherwise verifies.
    {
        let vault =
            medscale_storage::EncryptedVault::open_with_passphrase(&root, PASS, "cli-holder")
                .expect("adopt");
        vault
            .meta
            .replace_authority_snapshot(
                &[medscale_storage::AuthorityObjectRow {
                    object_id: "x".to_owned(),
                    object_class: "not-a-class".to_owned(),
                    realm_id: "realm".to_owned(),
                    authority_scope_id: "scope".to_owned(),
                    body_json: "{}".to_owned(),
                    content_digest_hex: None,
                    updated_seq: 1,
                }],
                1,
            )
            .expect("plant");
        // Dropped without closing, so the planted row stays unsealed.
    }
    let facade = connected();
    let opened = facade.dispatch(req(
        Capability::OpenEncryptedVault,
        RequestBody::OpenEncryptedVault {
            vault_root: root.display().to_string(),
            passphrase: Some(PASS.to_owned()),
            recovery_code: None,
            discard_leftover: false,
        },
    ));
    let ResponseBody::EncryptedVaultReady { open_outcome, .. } = opened.result.expect("opened")
    else {
        panic!("expected ready");
    };
    assert_eq!(
        open_outcome,
        Some(VaultOpenOutcome::DiscardedInterruptedSession)
    );
    let listed = facade.dispatch(req(Capability::ListOutbox, RequestBody::ListOutbox));
    let ResponseBody::Outbox { entries } = listed.result.unwrap() else {
        panic!("expected outbox");
    };
    assert_eq!(entries.len(), 1, "the sealed write is what opened");
    assert!(close(&facade));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_seal_that_cannot_load_changes_nothing_and_leaves_no_session_behind() {
    let root = tmp_dir("bad-seal");
    {
        let facade = connected();
        create_vault(&facade, &root);
        assert!(close(&facade));
    }
    {
        let vault =
            medscale_storage::EncryptedVault::open_with_passphrase(&root, PASS, "cli-holder")
                .expect("open");
        vault
            .meta
            .replace_authority_snapshot(
                &[medscale_storage::AuthorityObjectRow {
                    object_id: "x".to_owned(),
                    object_class: "not-a-class".to_owned(),
                    realm_id: "realm".to_owned(),
                    authority_scope_id: "scope".to_owned(),
                    body_json: "{}".to_owned(),
                    content_digest_hex: None,
                    updated_seq: 1,
                }],
                1,
            )
            .expect("plant");
        vault.close().expect("seal");
    }
    let seal = fs::read(root.join("meta.sealed")).unwrap();
    let facade = connected();
    let opened = facade.dispatch(req(
        Capability::OpenEncryptedVault,
        RequestBody::OpenEncryptedVault {
            vault_root: root.display().to_string(),
            passphrase: Some(PASS.to_owned()),
            recovery_code: None,
            discard_leftover: false,
        },
    ));
    assert!(opened.result.is_err(), "{opened:?}");
    assert_eq!(fs::read(root.join("meta.sealed")).unwrap(), seal);
    assert!(
        !root.join("meta.work.sqlite3").exists(),
        "a pristine copy is not a session, and must not read as one next time"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_discard_notice_survives_until_it_is_delivered() {
    let root = tmp_dir("notice");
    {
        let facade = connected();
        create_vault(&facade, &root);
        assert!(close(&facade));
    }
    fs::write(root.join("meta.work.sqlite3"), b"not a database").unwrap();
    let outcome_of_open = || {
        let facade = connected();
        let opened = facade.dispatch(req(
            Capability::OpenEncryptedVault,
            RequestBody::OpenEncryptedVault {
                vault_root: root.display().to_string(),
                passphrase: Some(PASS.to_owned()),
                recovery_code: None,
                discard_leftover: false,
            },
        ));
        let ResponseBody::EncryptedVaultReady { open_outcome, .. } = opened.result.expect("opened")
        else {
            panic!("expected ready");
        };
        assert!(close(&facade));
        open_outcome
    };
    assert_eq!(
        outcome_of_open(),
        Some(VaultOpenOutcome::DiscardedInterruptedSession)
    );
    assert_eq!(outcome_of_open(), None, "delivered once");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_wrong_key_for_a_move_aside_changes_nothing() {
    let root = tmp_dir("move-aside-key");
    {
        let facade = connected();
        create_vault(&facade, &root);
        add_intent(&facade, b"unsealed");
    }
    let work = fs::read(root.join("meta.work.sqlite3")).unwrap();
    let facade = connected();
    let opened = facade.dispatch(req(
        Capability::OpenEncryptedVault,
        RequestBody::OpenEncryptedVault {
            vault_root: root.display().to_string(),
            passphrase: Some("not-the-passphrase".to_owned()),
            recovery_code: None,
            discard_leftover: true,
        },
    ));
    assert!(matches!(
        opened.result,
        Err(AuthorityError::MissingKeyMaterial)
    ));
    assert_eq!(fs::read(root.join("meta.work.sqlite3")).unwrap(), work);
    assert!(!root.join("discarded-session").exists());
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
    assert_eq!(session.last_open_outcome(), None);
    drop(session); // A crash: nothing sealed this session.

    let mut reopened = CliSession::connect("v-109").unwrap();
    reopened.open_encrypted_vault(&root_text, PASS).unwrap();
    assert_eq!(
        reopened.last_open_outcome(),
        Some(VaultOpenOutcome::RecoveredInterruptedSession)
    );
    reopened.close_encrypted_vault().unwrap();
    fs::remove_dir_all(&root).unwrap();
}
