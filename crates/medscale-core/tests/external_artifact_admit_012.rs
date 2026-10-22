//! Spec 012 MESC admit fail-closed while gate open.

use medscale_contracts::envelopes::{AuthorityError, AuthorityRequest, Capability, RequestBody};
use medscale_contracts::external_artifact::{MescArtifactAdmitRequest, MescArtifactDoctorStatus};
use medscale_contracts::objects::{AuthorityScopeId, DigestSha256, OpaqueId, RealmId, VaultId};
use medscale_core::{CoreFacade, build_doctor_report};

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
fn mesc_admit_refuses_without_released_artifact_gate() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let out = facade.dispatch(req(
        Capability::MescArtifactAdmit,
        RequestBody::MescArtifactAdmit {
            request: MescArtifactAdmitRequest {
                artifact_uri: "https://github.com/TheHalfMoon/MESC/releases/download/missing/x"
                    .to_owned(),
                content_digest: DigestSha256::of(b"missing"),
                rights_uri: "https://example.invalid/rights".to_owned(),
                sbom_digest: DigestSha256::of(b"sbom"),
                evaluation_digest: DigestSha256::of(b"eval"),
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
}

#[test]
fn mesc_admit_requires_pack_path_flag() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let out = facade.dispatch(req(
        Capability::MescArtifactAdmit,
        RequestBody::MescArtifactAdmit {
            request: MescArtifactAdmitRequest {
                artifact_uri: "x".to_owned(),
                content_digest: DigestSha256::of(b"a"),
                rights_uri: "y".to_owned(),
                sbom_digest: DigestSha256::of(b"b"),
                evaluation_digest: DigestSha256::of(b"c"),
                pack_path_required: false,
            },
        },
    ));
    assert!(matches!(
        out.result,
        Err(AuthorityError::InvalidArgument { .. })
    ));
}

#[test]
fn doctor_mesc_axis_gate_blocked() {
    let s = MescArtifactDoctorStatus::gate_blocked();
    assert!(!s.artifact_admitted);
    assert!(s.verifier_ready_base);
    assert!(!s.python_runtime_imported);
    assert_eq!(s.gate, "MESC_RELEASED_ARTIFACT");
    assert!(s.is_honest_ready_base());
    let report = build_doctor_report(None, false, false);
    assert!(!report.mesc_artifact.artifact_admitted);
    assert!(report.mesc_artifact.verifier_ready_base);
}
