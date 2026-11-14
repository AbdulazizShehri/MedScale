use medscale_contracts::objects::{AuthorityScopeId, DigestSha256, RealmId};
use medscale_core::authority::{
    InMemoryAuthorityStore, create_source_record, overwrite_source_bytes, verify_source_digest,
};

#[test]
fn digest_verify_and_no_overwrite() {
    let mut store = InMemoryAuthorityStore::new();
    let record = create_source_record(
        &mut store,
        RealmId::new("r"),
        AuthorityScopeId::new("s"),
        "text/plain".to_owned(),
        b"data".to_vec(),
    );
    assert!(verify_source_digest(&record));
    assert_eq!(record.content_digest, DigestSha256::of(b"data"));
    assert!(overwrite_source_bytes(&record.header.id, b"other").is_err());
}
