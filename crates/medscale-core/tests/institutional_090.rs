//! Spec 090 institutional adapter integration tests.
//!
//! Every call goes through `CliSession` -> `CoreFacade`. The destination is
//! an institutional object store inside this process with fault injection;
//! no network is used. The product default transport is unavailable.

use std::sync::Arc;

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::institutional::{
    AdapterActRequest, AdapterCapability, AdapterConfig, AdapterKind, AdapterState,
    TransportOutcome, WriteRefusal,
};
use medscale_contracts::objects::{EffectState, OpaqueId};
use medscale_contracts::privacy_gate::DataClass;
use medscale_core::CliSession;
use medscale_core::institutional_transport::{Fault, InProcessStore};

const DEST: &str = "s3://lab-bucket/exports";

struct Lab {
    s: CliSession,
    project: OpaqueId,
    store: Arc<InProcessStore>,
    dir: std::path::PathBuf,
}

fn setup(name: &str, with_store: bool) -> Lab {
    let dir = std::env::temp_dir().join(format!("medscale-090c-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = CliSession::connect(&format!("vault-090-{name}")).unwrap();
    s.open_synthetic_vault(&dir.display().to_string()).unwrap();
    let store = Arc::new(InProcessStore::default());
    if with_store {
        s.set_institutional_transport(store.clone());
    }
    let project = s.project_create("lab".to_owned(), None).unwrap().header.id;
    Lab {
        s,
        project,
        store,
        dir,
    }
}

fn config(ceiling: DataClass) -> AdapterConfig {
    AdapterConfig {
        kind: AdapterKind::ObjectStorage,
        destination: DEST.to_owned(),
        data_class_ceiling: ceiling,
        credential_handle: Some("cred:lab-s3".to_owned()),
        capabilities: vec![AdapterCapability::PutObject, AdapterCapability::HeadObject],
    }
}

fn register(lab: &mut Lab, ceiling: DataClass) -> OpaqueId {
    lab.s
        .adapter_act(AdapterActRequest::Register {
            project_id: lab.project.clone(),
            name: "lab-s3".to_owned(),
            config: config(ceiling),
        })
        .unwrap()
        .adapter
        .unwrap()
        .header
        .id
}

fn artifact(lab: &mut Lab, bytes: &[u8], class: Option<DataClass>) -> OpaqueId {
    let id = lab
        .s
        .create_source_record("application/json".to_owned(), bytes.to_vec())
        .unwrap();
    if let Some(c) = class {
        lab.s
            .privacy_classify(lab.project.clone(), id.clone(), c, None)
            .unwrap();
    }
    id
}

fn intend(
    lab: &mut Lab,
    adapter: &OpaqueId,
    artifact: &OpaqueId,
    key: &str,
) -> medscale_contracts::institutional::AdapterActResult {
    lab.s
        .adapter_act(AdapterActRequest::Intend {
            adapter_id: adapter.clone(),
            artifact_id: artifact.clone(),
            object_key: key.to_owned(),
        })
        .unwrap()
}

fn act(lab: &mut Lab, a: AdapterActRequest) -> medscale_contracts::institutional::AdapterActResult {
    lab.s.adapter_act(a).unwrap()
}

#[test]
fn a_write_is_intended_sent_and_confirmed_with_receipts() {
    let mut lab = setup("happy", true);
    let adapter = register(&mut lab, DataClass::ExternalDeidentified);
    let art = artifact(
        &mut lab,
        br#"{"mean":52.5}"#,
        Some(DataClass::ExternalDeidentified),
    );
    let intent = intend(&mut lab, &adapter, &art, "exports/means.json")
        .intent
        .unwrap();
    assert_eq!(intent.state, EffectState::Pending);
    let sent = act(
        &mut lab,
        AdapterActRequest::Send {
            intent_id: intent.header.id.clone(),
        },
    );
    assert_eq!(sent.intent.as_ref().unwrap().state, EffectState::Confirmed);
    assert_eq!(
        sent.receipt.transport_outcome,
        Some(TransportOutcome::Stored)
    );
    assert_eq!(
        lab.store.object(DEST, "exports/means.json").unwrap(),
        br#"{"mean":52.5}"#.to_vec()
    );
    // A confirmed write is never sent again.
    let again = act(
        &mut lab,
        AdapterActRequest::Send {
            intent_id: intent.header.id.clone(),
        },
    );
    assert_eq!(again.receipt.refusal, Some(WriteRefusal::AlreadyConfirmed));
    assert_eq!(lab.store.puts(), 1);
    let view = lab.s.adapter_get(adapter.clone()).unwrap();
    assert!(view.receipts.len() >= 4);
    let listed = lab.s.adapter_list(lab.project.clone()).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].header.id, adapter);
}

#[test]
fn local_phi_and_over_ceiling_data_never_leave() {
    let mut lab = setup("classes", true);
    let adapter = register(&mut lab, DataClass::ExternalDeidentified);
    let unclassified = artifact(&mut lab, b"patient list", None);
    let team = artifact(&mut lab, b"team notes", Some(DataClass::TeamProtected));
    for a in [&unclassified, &team] {
        let r = intend(&mut lab, &adapter, a, "exports/x.json");
        assert_eq!(r.receipt.refusal, Some(WriteRefusal::DataClassAboveCeiling));
        assert!(r.intent.is_none());
    }
    assert!(matches!(
        lab.s.adapter_act(AdapterActRequest::Register {
            project_id: lab.project.clone(),
            name: "phi".to_owned(),
            config: config(DataClass::LocalPhi),
        }),
        Err(AuthorityError::InvalidArgument { .. })
    ));
    let public = artifact(&mut lab, b"{}", Some(DataClass::Public));
    for bad in ["../x", "/abs", "a//b"] {
        assert_eq!(
            intend(&mut lab, &adapter, &public, bad).receipt.refusal,
            Some(WriteRefusal::BadObjectKey)
        );
    }
    assert_eq!(lab.store.puts(), 0);
}

#[test]
fn uncertain_writes_become_unknown_and_reconcile_never_blindly_retries() {
    let mut lab = setup("unknown", true);
    let adapter = register(&mut lab, DataClass::Public);
    let art = artifact(&mut lab, b"{\"n\":1}", Some(DataClass::Public));

    // Outage before sending: failed, nothing stored; retry is explicit.
    lab.store.set_fault(Fault::Outage);
    let i1 = intend(&mut lab, &adapter, &art, "a.json").intent.unwrap();
    let r = act(
        &mut lab,
        AdapterActRequest::Send {
            intent_id: i1.header.id.clone(),
        },
    );
    assert_eq!(r.intent.unwrap().state, EffectState::Failed);
    lab.store.set_fault(Fault::None);
    act(
        &mut lab,
        AdapterActRequest::Retry {
            intent_id: i1.header.id.clone(),
        },
    );
    let r = act(
        &mut lab,
        AdapterActRequest::Send {
            intent_id: i1.header.id.clone(),
        },
    );
    assert_eq!(r.intent.unwrap().state, EffectState::Confirmed);

    // Stored but the answer was lost: unknown; send and retry are refused.
    lab.store.set_fault(Fault::StoreThenTimeOut);
    let i2 = intend(&mut lab, &adapter, &art, "b.json").intent.unwrap();
    let r = act(
        &mut lab,
        AdapterActRequest::Send {
            intent_id: i2.header.id.clone(),
        },
    );
    assert_eq!(r.intent.unwrap().state, EffectState::Unknown);
    lab.store.set_fault(Fault::None);
    assert_eq!(
        act(
            &mut lab,
            AdapterActRequest::Send {
                intent_id: i2.header.id.clone()
            }
        )
        .receipt
        .refusal,
        Some(WriteRefusal::UnknownRequiresReconcile)
    );
    assert_eq!(
        act(
            &mut lab,
            AdapterActRequest::Retry {
                intent_id: i2.header.id.clone()
            }
        )
        .receipt
        .refusal,
        Some(WriteRefusal::UnknownRequiresReconcile)
    );
    let puts = lab.store.puts();
    // Reconciliation finds it stored: confirmed without sending again.
    let r = act(
        &mut lab,
        AdapterActRequest::Reconcile {
            intent_id: i2.header.id.clone(),
        },
    );
    assert_eq!(r.intent.unwrap().state, EffectState::Confirmed);
    assert_eq!(lab.store.puts(), puts, "reconcile does not write");

    // Answer lost and nothing stored: reconcile re-arms it as pending.
    lab.store.set_fault(Fault::TimeOutWithoutStore);
    let i3 = intend(&mut lab, &adapter, &art, "c.json").intent.unwrap();
    act(
        &mut lab,
        AdapterActRequest::Send {
            intent_id: i3.header.id.clone(),
        },
    );
    lab.store.set_fault(Fault::Outage);
    let blocked = act(
        &mut lab,
        AdapterActRequest::Reconcile {
            intent_id: i3.header.id.clone(),
        },
    );
    assert_eq!(
        blocked.receipt.refusal,
        Some(WriteRefusal::TransportUnavailable)
    );
    lab.store.set_fault(Fault::None);
    let r = act(
        &mut lab,
        AdapterActRequest::Reconcile {
            intent_id: i3.header.id.clone(),
        },
    );
    assert_eq!(r.intent.unwrap().state, EffectState::Pending);
    let r = act(
        &mut lab,
        AdapterActRequest::Send {
            intent_id: i3.header.id.clone(),
        },
    );
    assert_eq!(r.intent.unwrap().state, EffectState::Confirmed);
}

#[test]
fn revocation_and_rollback_are_explicit_and_final() {
    let mut lab = setup("revoke", true);
    let adapter = register(&mut lab, DataClass::Public);
    let art = artifact(&mut lab, b"{}", Some(DataClass::Public));
    // Reconfigure to a new destination, then roll back.
    let mut moved = config(DataClass::Public);
    moved.destination = "s3://other-bucket".to_owned();
    let r = act(
        &mut lab,
        AdapterActRequest::Reconfigure {
            adapter_id: adapter.clone(),
            config: moved,
        },
    );
    assert_eq!(r.adapter.unwrap().config_revision, 2);
    let r = act(
        &mut lab,
        AdapterActRequest::Rollback {
            adapter_id: adapter.clone(),
        },
    );
    let a = r.adapter.unwrap();
    assert_eq!(
        (a.config.destination.as_str(), a.config_revision),
        (DEST, 3)
    );

    // Suspended adapters send nothing; revoked ones never again.
    let i = intend(&mut lab, &adapter, &art, "k.json").intent.unwrap();
    act(
        &mut lab,
        AdapterActRequest::SetState {
            adapter_id: adapter.clone(),
            state: AdapterState::Suspended,
        },
    );
    assert_eq!(
        act(
            &mut lab,
            AdapterActRequest::Send {
                intent_id: i.header.id.clone()
            }
        )
        .receipt
        .refusal,
        Some(WriteRefusal::AdapterNotActive)
    );
    act(
        &mut lab,
        AdapterActRequest::SetState {
            adapter_id: adapter.clone(),
            state: AdapterState::Revoked,
        },
    );
    assert!(matches!(
        lab.s.adapter_act(AdapterActRequest::SetState {
            adapter_id: adapter.clone(),
            state: AdapterState::Active,
        }),
        Err(AuthorityError::Conflict { .. })
    ));
    assert_eq!(
        intend(&mut lab, &adapter, &art, "k2.json").receipt.refusal,
        Some(WriteRefusal::AdapterNotActive)
    );
    assert_eq!(lab.store.puts(), 0);
}

#[test]
fn the_product_transport_sends_nothing() {
    let mut lab = setup("product", false);
    let adapter = register(&mut lab, DataClass::Public);
    let art = artifact(&mut lab, b"{}", Some(DataClass::Public));
    let i = intend(&mut lab, &adapter, &art, "k.json").intent.unwrap();
    let r = act(
        &mut lab,
        AdapterActRequest::Send {
            intent_id: i.header.id,
        },
    );
    assert_eq!(r.intent.unwrap().state, EffectState::Failed);
    assert_eq!(
        r.receipt.transport_outcome,
        Some(TransportOutcome::Unreachable)
    );
}

#[test]
fn a_crash_after_sent_recovers_as_unknown_never_as_a_resend() {
    let mut lab = setup("crash", true);
    let adapter = register(&mut lab, DataClass::Public);
    let art = artifact(&mut lab, b"{}", Some(DataClass::Public));
    let i = intend(&mut lab, &adapter, &art, "k.json").intent.unwrap();
    // Simulate a crash between the durable `sent` and the transport's
    // answer: the row says `sent`, nothing reached the store.
    let conn = rusqlite::Connection::open(lab.dir.join("meta.sqlite3")).unwrap();
    conn.execute(
        "UPDATE ia_intents SET state = 'sent', body_json = replace(body_json, '\"state\":\"pending\"', '\"state\":\"sent\"') WHERE intent_id = ?1",
        [i.header.id.as_str()],
    )
    .unwrap();
    drop(conn);
    let r = act(
        &mut lab,
        AdapterActRequest::Send {
            intent_id: i.header.id.clone(),
        },
    );
    assert_eq!(
        r.receipt.refusal,
        Some(WriteRefusal::UnknownRequiresReconcile)
    );
    assert_eq!(lab.store.puts(), 0, "no blind resend");
    let view = lab.s.adapter_get(adapter).unwrap();
    assert_eq!(view.intents[0].state, EffectState::Unknown);
    let r = act(
        &mut lab,
        AdapterActRequest::Reconcile {
            intent_id: i.header.id.clone(),
        },
    );
    assert_eq!(r.intent.unwrap().state, EffectState::Pending);
    let r = act(
        &mut lab,
        AdapterActRequest::Send {
            intent_id: i.header.id,
        },
    );
    assert_eq!(r.intent.unwrap().state, EffectState::Confirmed);
}
