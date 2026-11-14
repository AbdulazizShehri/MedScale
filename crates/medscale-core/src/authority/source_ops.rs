//! Source create/verify helpers (immutability).

use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId, SourceRecord,
};

use super::store::{InMemoryAuthorityStore, StoredObject};

/// Creates a new source record; never overwrites an existing id.
pub fn create_source_record(
    store: &mut InMemoryAuthorityStore,
    realm_id: RealmId,
    authority_scope_id: AuthorityScopeId,
    media_type: String,
    bytes: Vec<u8>,
) -> SourceRecord {
    let id = store.alloc_id("src");
    let digest = DigestSha256::of(&bytes);
    let record = SourceRecord {
        header: ObjectHeader {
            id,
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id,
            authority_scope_id,
        },
        bytes,
        content_digest: digest,
        media_type,
        acquired_at: None,
        provenance_note: None,
    };
    store.insert(StoredObject::Source(record.clone()));
    record
}

/// Verifies digest; fail closed on mismatch.
#[must_use]
pub fn verify_source_digest(record: &SourceRecord) -> bool {
    record.digest_valid()
}

/// There is intentionally no overwrite API for source bytes.
pub fn overwrite_source_bytes(_id: &OpaqueId, _bytes: &[u8]) -> Result<(), &'static str> {
    Err("source bytes are immutable; create a new SourceRecord")
}
