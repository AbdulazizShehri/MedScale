//! Hostile-input unit checks for document MIME quarantine (FUZZ_POLICY synthetic).

use medscale_contracts::documents::{
    DocumentDenyReason, DocumentIntakeRequest, DocumentMimeClass, QuarantineDecision,
};
use medscale_core::authority::mime_decision;

#[test]
fn denied_mime_classes_quarantine() {
    for mime in [
        DocumentMimeClass::ApplicationOctetStream,
        DocumentMimeClass::TextHtml,
        DocumentMimeClass::OfficeOpenXml,
    ] {
        let req = DocumentIntakeRequest {
            mime,
            bytes: b"hostile".to_vec(),
            fixture_id: Some("hostile".to_owned()),
        };
        let (decision, reason) = mime_decision(&req);
        assert_eq!(decision, QuarantineDecision::Quarantine);
        assert_eq!(reason, DocumentDenyReason::MimeDenied);
    }
}

#[test]
fn empty_payload_quarantines() {
    let req = DocumentIntakeRequest {
        mime: DocumentMimeClass::TextPlain,
        bytes: vec![],
        fixture_id: None,
    };
    let (decision, reason) = mime_decision(&req);
    assert_eq!(decision, QuarantineDecision::Quarantine);
    assert_eq!(reason, DocumentDenyReason::EmptyPayload);
}
