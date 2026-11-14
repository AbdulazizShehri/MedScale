//! Spec 008 pack admission + worker confinement tests.

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::packs::{PackAdmitReason, PackArtifactKind, PackPromotionState};
use medscale_contracts::worker_policy::WorkerSupervisionPolicy;
use medscale_core::CoreFacade;
use medscale_pack::{FixtureRuntime, PackRuntimeAdapter, admit_pack_dir, forbidden_reason};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

fn uid() -> u64 {
    static C: AtomicU64 = AtomicU64::new(1);
    C.fetch_add(1, Ordering::SeqCst)
}

fn req(cap: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("r-{}", uid())),
        VaultId::new("v-008"),
        RealmId::new("realm-008"),
        AuthorityScopeId::new("scope-008"),
        cap,
        body,
    )
}

fn lease(facade: &CoreFacade) {
    facade
        .dispatch(req(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("cli"),
                holder_id_hint: None,
            },
        ))
        .result
        .unwrap();
}

fn fixture_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../evidence/008-local-ai-capability-fabric/fixtures/pack-fixture-ner-v0")
}

#[test]
fn offline_pack_install_and_list() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    lease(&facade);
    let path = fixture_pack().display().to_string();
    let out = facade
        .dispatch(req(
            Capability::PacksInstallLocal,
            RequestBody::PacksInstallLocal { local_path: path },
        ))
        .result
        .unwrap();
    let ResponseBody::PackAdmit { result } = out else {
        panic!("expected admit");
    };
    assert!(result.admitted);
    assert_eq!(result.reason, PackAdmitReason::Ok);

    let list = facade
        .dispatch(req(Capability::PacksList, RequestBody::PacksList))
        .result
        .unwrap();
    let ResponseBody::PackList { packs } = list else {
        panic!("expected list");
    };
    assert_eq!(packs.len(), 1);
    assert_eq!(packs[0].promotion_state, PackPromotionState::Candidate);

    let promoted = facade
        .dispatch(req(
            Capability::PacksPromote,
            RequestBody::PacksPromote {
                pack_id: packs[0].pack_id.clone(),
                to: PackPromotionState::Current,
            },
        ))
        .result
        .unwrap();
    let ResponseBody::PackPromoted { state, .. } = promoted else {
        panic!("expected promote");
    };
    assert_eq!(state, PackPromotionState::Current);
}

#[test]
fn format_deny_pickle_and_custom_op() {
    assert_eq!(
        forbidden_reason(PackArtifactKind::Pickle),
        Some(PackAdmitReason::ForbiddenArtifactKind)
    );
    assert_eq!(
        forbidden_reason(PackArtifactKind::CodeBin),
        Some(PackAdmitReason::ForbiddenArtifactKind)
    );
    assert_eq!(
        forbidden_reason(PackArtifactKind::OnnxCustomOp),
        Some(PackAdmitReason::ForbiddenArtifactKind)
    );
    assert!(forbidden_reason(PackArtifactKind::FixtureBytes).is_none());
}

#[test]
fn admit_fixture_dir_direct() {
    let m = admit_pack_dir(&fixture_pack()).expect("admit");
    assert_eq!(m.pack_id.as_str(), "pack-fixture-ner-v0");
}

#[test]
fn worker_ambient_deny_and_fixture_runtime_not_assertion() {
    let policy = WorkerSupervisionPolicy::deny_by_default();
    assert!(policy.ambient_denied());
    assert_eq!(policy.confinement_profile, "policy_ambient_deny_v0");
    assert!(!policy.allow_network);
    assert!(!policy.allow_authority);

    let m = admit_pack_dir(&fixture_pack()).unwrap();
    let out = FixtureRuntime.run_fixture(&m, "synthetic input");
    assert!(out.evidence_only);
    assert_eq!(
        out.proposal_payload.get("note").and_then(|v| v.as_str()),
        Some("Proposal-only; never ClinicalAssertion")
    );
}

#[test]
fn cli_desktop_do_not_depend_on_medscale_pack_directly() {
    let cli = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../medscale-cli/Cargo.toml"
    ));
    let desktop = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../medscale-desktop/Cargo.toml"
    ));
    assert!(!cli.contains("medscale-pack"));
    assert!(!desktop.contains("medscale-pack"));
}
