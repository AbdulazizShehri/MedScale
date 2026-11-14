//! Spec 015 online pack READY_BASE (deny path).

use medscale_contracts::envelopes::{AuthorityError, AuthorityRequest, Capability, RequestBody};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::online_packs::{
    OfflineRootPolicy, OnlinePackAcquireRequest, OnlinePackFormatConstraints,
    OnlinePacksDoctorStatus,
};
use medscale_core::{CoreFacade, build_doctor_report};
use std::fs;
use std::path::PathBuf;

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

#[test]
fn online_acquire_refuses_hf_gate() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let out = facade.dispatch(req(
        Capability::OnlinePackAcquire,
        RequestBody::OnlinePackAcquire {
            request: OnlinePackAcquireRequest {
                pack_id: "demo".to_owned(),
                version: "0.0.0".to_owned(),
                distribution_uri: "https://huggingface.co/example".to_owned(),
                broker_required: true,
                format: OnlinePackFormatConstraints::from_spec_009(),
            },
        },
    ));
    assert_eq!(
        out.result,
        Err(AuthorityError::ExternalGateRequired {
            gate: "HF_ONLINE_PACK_DISTRIBUTION".to_owned()
        })
    );
}

#[test]
fn online_acquire_requires_broker_flag() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let out = facade.dispatch(req(
        Capability::OnlinePackAcquire,
        RequestBody::OnlinePackAcquire {
            request: OnlinePackAcquireRequest {
                pack_id: "demo".to_owned(),
                version: "0.0.0".to_owned(),
                distribution_uri: "https://example.invalid".to_owned(),
                broker_required: false,
                format: OnlinePackFormatConstraints::from_spec_009(),
            },
        },
    ));
    assert!(matches!(
        out.result,
        Err(AuthorityError::InvalidArgument { .. })
    ));
}

#[test]
fn doctor_online_packs_denied() {
    let s = OnlinePacksDoctorStatus::ready_base();
    assert!(!s.online_download_authorized);
    assert!(s.broker_required);
    assert!(!s.hf_runtime_required);
    assert!(!OfflineRootPolicy::ready_base().roots_admitted);
    let report = build_doctor_report(None, false, false);
    assert!(!report.online_packs.online_download_authorized);
}

#[test]
fn workspace_cargo_tomls_have_no_hf_sdk() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let forbidden = [
        "huggingface",
        "hf-hub",
        "hf_hub",
        "candle-hf",
        "tokenizers-hf",
    ];
    for entry in fs::read_dir(root.join("crates")).unwrap() {
        let entry = entry.unwrap();
        let cargo = entry.path().join("Cargo.toml");
        if !cargo.is_file() {
            continue;
        }
        let text = fs::read_to_string(&cargo).unwrap().to_lowercase();
        for needle in forbidden {
            assert!(
                !text.contains(needle),
                "{} must not depend on {needle}",
                cargo.display()
            );
        }
    }
}
