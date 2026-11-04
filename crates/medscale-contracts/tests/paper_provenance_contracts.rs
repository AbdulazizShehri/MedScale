//! Paper Experiment A0: contract-level provenance and evidence invariants.
//!
//! This is a research harness layered on top of the frozen scientific snapshot
//! `1e2b7d94e970256b38bda15fa91f62bc397e825a`. It establishes only contract-
//! level controls. System-level refusal/detection experiments belong in later
//! A1+ harnesses and must not be inferred from these tests.

use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId, SourceRecord,
};
use medscale_contracts::research_packs::{
    EvidenceAssessment, EvidenceQuality, SupportRelation, Tri,
};

fn header(id: &str) -> ObjectHeader {
    ObjectHeader {
        id: OpaqueId::new(id),
        schema_version: 1,
        realm_id: RealmId::new("paper-realm"),
        authority_scope_id: AuthorityScopeId::new("paper-scope"),
    }
}

fn source(id: &str, bytes: &[u8]) -> SourceRecord {
    SourceRecord {
        header: header(id),
        bytes: bytes.to_vec(),
        content_digest: DigestSha256::of(bytes),
        media_type: "application/json".to_owned(),
        acquired_at: None,
        provenance_note: Some("synthetic paper fixture".to_owned()),
    }
}

fn assessment() -> EvidenceAssessment {
    EvidenceAssessment {
        claim: "Synthetic claim".to_owned(),
        citation: "doi:10.0000/synthetic".to_owned(),
        citation_exists: Tri::Yes,
        relation: SupportRelation::Supports,
        applicable_to_population: Tri::Yes,
        year: Some(2026),
        jurisdiction: None,
        guideline_version: None,
        retracted: Tri::No,
        quality: EvidenceQuality::Moderate,
        trial_criteria_met: Tri::Unknown,
    }
}

#[test]
fn a0_source_digest_detects_content_mutation() {
    let mut record = source("src-paper-1", br#"{"value":1}"#);
    assert!(record.digest_valid(), "known-valid control must verify");

    record.bytes = br#"{"value":2}"#.to_vec();
    assert!(
        !record.digest_valid(),
        "holding the recorded digest constant while mutating bytes must be detectable"
    );
}

#[test]
fn a0_source_digest_detects_digest_mutation() {
    let mut record = source("src-paper-2", br#"{"value":1}"#);
    assert!(record.digest_valid(), "known-valid control must verify");

    record.content_digest = DigestSha256::of(br#"{"other":true}"#);
    assert!(
        !record.digest_valid(),
        "holding bytes constant while replacing the digest must be detectable"
    );
}

#[test]
fn a0_source_identity_is_not_content_identity() {
    let a = source("src-paper-a", b"identical synthetic evidence");
    let b = source("src-paper-b", b"identical synthetic evidence");

    assert_eq!(a.content_digest, b.content_digest);
    assert_ne!(a.header.id, b.header.id);
    assert!(a.digest_valid() && b.digest_valid());
}

#[test]
fn a0_nonexistent_citation_cannot_be_recorded_as_support() {
    let mut item = assessment();
    item.citation_exists = Tri::No;

    assert!(item.validate().is_err());
}

#[test]
fn a0_retracted_citation_cannot_be_recorded_as_support() {
    let mut item = assessment();
    item.retracted = Tri::Yes;

    assert!(item.validate().is_err());
}

#[test]
fn a0_unknown_evidence_stays_unknown() {
    let mut item = assessment();
    item.applicable_to_population = Tri::Unknown;

    assert!(item.validate().is_ok());
    assert_eq!(item.verdict(), "unknown");
}

#[test]
fn a0_fully_recorded_support_has_supported_verdict() {
    let item = assessment();

    assert!(item.validate().is_ok());
    assert_eq!(item.verdict(), "supported");
}
