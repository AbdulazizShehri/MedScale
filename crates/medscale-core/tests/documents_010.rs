//! Spec 010 document MIME quarantine + OCR/ASR stub tests.

use medscale_contracts::documents::{
    AsrStubRequest, DocumentIntakeRequest, DocumentMimeClass, OcrStubRequest, QuarantineDecision,
};
use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::CoreFacade;
use medscale_core::authority::{document_worker_policy, mime_decision, voice_worker_policy};
use std::sync::atomic::{AtomicU64, Ordering};

fn uid() -> u64 {
    static C: AtomicU64 = AtomicU64::new(1);
    C.fetch_add(1, Ordering::SeqCst)
}

fn req(cap: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("r-{}", uid())),
        VaultId::new("v-010"),
        RealmId::new("realm-010"),
        AuthorityScopeId::new("scope-010"),
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

#[test]
fn mime_deny_html_and_office() {
    let html = DocumentIntakeRequest {
        mime: DocumentMimeClass::TextHtml,
        bytes: b"<html/>".to_vec(),
        fixture_id: Some("x".into()),
    };
    let (d, _) = mime_decision(&html);
    assert_eq!(d, QuarantineDecision::Quarantine);

    let office = DocumentIntakeRequest {
        mime: DocumentMimeClass::OfficeOpenXml,
        bytes: b"PK".to_vec(),
        fixture_id: None,
    };
    assert_eq!(mime_decision(&office).0, QuarantineDecision::Quarantine);
}

#[test]
fn admit_text_plain_then_ocr_asr_stubs() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    lease(&facade);
    let out = facade
        .dispatch(req(
            Capability::DocumentIntake,
            RequestBody::DocumentIntake {
                request: DocumentIntakeRequest {
                    mime: DocumentMimeClass::TextPlain,
                    bytes: b"synthetic note".to_vec(),
                    fixture_id: Some("doc1".into()),
                },
            },
        ))
        .result
        .unwrap();
    let ResponseBody::DocumentIntake { result } = out else {
        panic!("expected intake");
    };
    assert_eq!(result.decision, QuarantineDecision::Admit);
    let source_id = result.source_id.expect("source");

    let ocr = facade
        .dispatch(req(
            Capability::OcrStub,
            RequestBody::OcrStub {
                request: OcrStubRequest {
                    source_id: source_id.clone(),
                    fixture_id: "ocr1".into(),
                },
            },
        ))
        .result
        .unwrap();
    let ResponseBody::MediaStub { result: ocr_r } = ocr else {
        panic!("ocr");
    };
    assert!(ocr_r.evidence_only);
    assert!(ocr_r.text.contains("OCR_FIXTURE"));

    let asr = facade
        .dispatch(req(
            Capability::AsrStub,
            RequestBody::AsrStub {
                request: AsrStubRequest {
                    source_id,
                    fixture_id: "asr1".into(),
                },
            },
        ))
        .result
        .unwrap();
    let ResponseBody::MediaStub { result: asr_r } = asr else {
        panic!("asr");
    };
    assert!(asr_r.evidence_only);
    assert!(asr_r.text.contains("ASR_FIXTURE"));
}

#[test]
fn quarantine_denied_mime_no_source() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    lease(&facade);
    let out = facade
        .dispatch(req(
            Capability::DocumentIntake,
            RequestBody::DocumentIntake {
                request: DocumentIntakeRequest {
                    mime: DocumentMimeClass::ApplicationOctetStream,
                    bytes: b"\0\0".to_vec(),
                    fixture_id: None,
                },
            },
        ))
        .result
        .unwrap();
    let ResponseBody::DocumentIntake { result } = out else {
        panic!("expected");
    };
    assert_eq!(result.decision, QuarantineDecision::Quarantine);
    assert!(result.source_id.is_none());
}

#[test]
fn worker_policies_ambient_deny() {
    assert!(document_worker_policy().ambient_denied());
    assert_eq!(
        document_worker_policy().confinement_profile,
        "p1_document_worker_v0"
    );
    assert!(voice_worker_policy().ambient_denied());
    assert_eq!(
        voice_worker_policy().confinement_profile,
        "p1_voice_worker_v0"
    );
}
