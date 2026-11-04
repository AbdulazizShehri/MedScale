//! Persist / reload `InMemoryAuthorityStore` through vault metadata (Specs 016 / 035).

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::{
    ActionAuditRecord, AmendmentRecord, ClinicalAssertion, DerivedSourceArtifact, DigestSha256,
    EvaluationRecord, IdentityAssertion, IdentityMergeDecision, Projection, Proposal, SourceRecord,
};
use medscale_storage::{
    AuthorityObjectRow, EncryptedVault, FsBlobStore, SqliteMetaStore, SyntheticVault,
};
use serde_json::Value;

use super::store::{InMemoryAuthorityStore, StoredObject};

trait BlobBackend {
    fn put_bytes(&self, bytes: &[u8]) -> Result<DigestSha256, AuthorityError>;
    fn get_bytes(&self, digest: &DigestSha256) -> Result<Vec<u8>, AuthorityError>;
}

impl BlobBackend for FsBlobStore {
    fn put_bytes(&self, bytes: &[u8]) -> Result<DigestSha256, AuthorityError> {
        Ok(self
            .put_blob(bytes)
            .map_err(|e| AuthorityError::InvalidArgument {
                message: e.to_string(),
            })?
            .digest)
    }

    fn get_bytes(&self, digest: &DigestSha256) -> Result<Vec<u8>, AuthorityError> {
        self.get_blob(digest)
            .map_err(|e| AuthorityError::InvalidArgument {
                message: e.to_string(),
            })
    }
}

struct EncryptedBlobBackend<'a>(&'a EncryptedVault);

impl BlobBackend for EncryptedBlobBackend<'_> {
    fn put_bytes(&self, bytes: &[u8]) -> Result<DigestSha256, AuthorityError> {
        self.0
            .put_blob(bytes)
            .map_err(|e| AuthorityError::InvalidArgument {
                message: e.to_string(),
            })
    }

    fn get_bytes(&self, digest: &DigestSha256) -> Result<Vec<u8>, AuthorityError> {
        use medscale_storage::{EncryptedVaultError, SealedBlobError};
        // Spec 109 Part B: only a real absence or a failed check is definitive.
        // Anything else is transient, and must not read as a damaged vault.
        self.0.get_blob(digest).map_err(|e| match e {
            EncryptedVaultError::Sealed(SealedBlobError::DigestMismatch) => {
                AuthorityError::DigestMismatch
            }
            EncryptedVaultError::Sealed(SealedBlobError::NotFound | SealedBlobError::Crypto) => {
                AuthorityError::Corrupt {
                    message: e.to_string(),
                }
            }
            _ => AuthorityError::Unavailable {
                message: e.to_string(),
            },
        })
    }
}

pub fn sync_store_to_vault(
    vault: &SyntheticVault,
    store: &InMemoryAuthorityStore,
) -> Result<(), AuthorityError> {
    sync_store_to_meta(&vault.meta, &vault.blobs, store)
}

pub fn load_store_from_vault(
    vault: &SyntheticVault,
    store: &mut InMemoryAuthorityStore,
) -> Result<(), AuthorityError> {
    load_store_from_meta(&vault.meta, &vault.blobs, store)?;
    if store.objects_raw().is_empty() {
        // Empty authority rows: reconstruct Source envelopes from sources + blobs (v1).
        reconstruct_sources_from_meta(vault, store)?;
    }
    Ok(())
}

/// Spec 035: persist authority graph into open EncryptedVault (SQLCipher meta + sealed blobs).
pub fn sync_store_to_encrypted(
    vault: &EncryptedVault,
    store: &InMemoryAuthorityStore,
) -> Result<(), AuthorityError> {
    sync_store_to_meta(&vault.meta, &EncryptedBlobBackend(vault), store)
}

/// Spec 035: reload authority graph from EncryptedVault.
pub fn load_store_from_encrypted(
    vault: &EncryptedVault,
    store: &mut InMemoryAuthorityStore,
) -> Result<(), AuthorityError> {
    load_store_from_meta(&vault.meta, &EncryptedBlobBackend(vault), store)
}

fn sync_store_to_meta(
    meta: &SqliteMetaStore,
    blobs: &impl BlobBackend,
    store: &InMemoryAuthorityStore,
) -> Result<(), AuthorityError> {
    let mut rows = Vec::new();
    for obj in store.objects_raw().values() {
        match obj {
            StoredObject::Source(s) => {
                blobs.put_bytes(&s.bytes)?;
                let _ = meta.upsert_source(&medscale_storage::SourceMeta {
                    source_id: s.header.id.clone(),
                    realm_id: s.header.realm_id.clone(),
                    authority_scope_id: s.header.authority_scope_id.clone(),
                    digest: s.content_digest.clone(),
                    byte_length: s.bytes.len() as u64,
                    media_type: s.media_type.clone(),
                    visible: true,
                    resource_type: "authority_source".to_owned(),
                });
            }
            StoredObject::Derived(d) => {
                blobs.put_bytes(&d.bytes)?;
            }
            _ => {}
        }
        rows.push(to_row(obj, store.next_seq())?);
    }
    meta.replace_authority_snapshot(&rows, store.next_seq())
        .map_err(|e| AuthorityError::InvalidArgument {
            message: e.to_string(),
        })?;
    Ok(())
}

/// A row that won't decode is damage. The default is transient (Spec 109 I3).
fn load_error(e: medscale_storage::MetaError) -> AuthorityError {
    match e {
        medscale_storage::MetaError::CorruptObjectBody(message) => {
            AuthorityError::Corrupt { message }
        }
        other => AuthorityError::Unavailable {
            message: other.to_string(),
        },
    }
}

fn load_store_from_meta(
    meta: &SqliteMetaStore,
    blobs: &impl BlobBackend,
    store: &mut InMemoryAuthorityStore,
) -> Result<(), AuthorityError> {
    store.clear();
    let next_seq = meta.get_next_seq().map_err(load_error)?;
    store.set_next_seq(next_seq);
    let rows = meta.list_authority_objects().map_err(load_error)?;
    for row in rows {
        let obj = from_row(&row, blobs)?;
        store.insert(obj);
    }
    Ok(())
}

fn reconstruct_sources_from_meta(
    vault: &SyntheticVault,
    store: &mut InMemoryAuthorityStore,
) -> Result<(), AuthorityError> {
    let sources = vault
        .meta
        .list_sources()
        .map_err(|e| AuthorityError::InvalidArgument {
            message: e.to_string(),
        })?;
    let mut max_seq = store.next_seq();
    for meta in sources {
        let bytes =
            vault
                .blobs
                .get_blob(&meta.digest)
                .map_err(|e| AuthorityError::InvalidArgument {
                    message: format!("content missing for {}: {e}", meta.source_id.as_str()),
                })?;
        if DigestSha256::of(&bytes) != meta.digest {
            return Err(AuthorityError::DigestMismatch);
        }
        store.insert(StoredObject::Source(SourceRecord {
            header: medscale_contracts::objects::ObjectHeader {
                id: meta.source_id.clone(),
                schema_version: medscale_contracts::AUTHORITY_SCHEMA_VERSION,
                realm_id: meta.realm_id.clone(),
                authority_scope_id: meta.authority_scope_id.clone(),
            },
            bytes,
            content_digest: meta.digest,
            media_type: meta.media_type,
            acquired_at: None,
            provenance_note: Some(format!("reconstructed-from-sources:{}", meta.resource_type)),
        }));
        max_seq = max_seq.max(estimate_seq(meta.source_id.as_str()));
    }
    if max_seq > store.next_seq() {
        store.set_next_seq(max_seq);
    }
    // Persist reconstructed authority snapshot for v1 synthetic vaults.
    sync_store_to_vault(vault, store)?;
    Ok(())
}

fn estimate_seq(id: &str) -> u64 {
    id.rsplit('-')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn to_row(obj: &StoredObject, updated_seq: u64) -> Result<AuthorityObjectRow, AuthorityError> {
    let (class, realm, scope, id, digest, body) = match obj {
        StoredObject::Source(s) => {
            let mut v = serde_json::to_value(s).map_err(|e| AuthorityError::InvalidArgument {
                message: e.to_string(),
            })?;
            if let Some(map) = v.as_object_mut() {
                map.insert("bytes".to_owned(), Value::Array(vec![]));
            }
            (
                "source".to_owned(),
                s.header.realm_id.as_opaque().as_str().to_owned(),
                s.header.authority_scope_id.as_opaque().as_str().to_owned(),
                s.header.id.as_str().to_owned(),
                Some(s.content_digest.to_hex()),
                v,
            )
        }
        StoredObject::Derived(d) => {
            let mut v = serde_json::to_value(d).map_err(|e| AuthorityError::InvalidArgument {
                message: e.to_string(),
            })?;
            if let Some(map) = v.as_object_mut() {
                map.insert("bytes".to_owned(), Value::Array(vec![]));
            }
            (
                "derived".to_owned(),
                d.header.realm_id.as_opaque().as_str().to_owned(),
                d.header.authority_scope_id.as_opaque().as_str().to_owned(),
                d.header.id.as_str().to_owned(),
                Some(d.content_digest.to_hex()),
                v,
            )
        }
        StoredObject::Proposal(p) => row_parts("proposal", p)?,
        StoredObject::Assertion(a) => row_parts("assertion", a)?,
        StoredObject::Audit(a) => row_parts("audit", a)?,
        StoredObject::Identity(a) => row_parts("identity", a)?,
        StoredObject::Merge(a) => row_parts("merge", a)?,
        StoredObject::Evaluation(e) => row_parts("evaluation", e)?,
        StoredObject::Projection(p) => row_parts("projection", p)?,
        StoredObject::Amendment(a) => row_parts("amendment", a)?,
    };
    Ok(AuthorityObjectRow {
        object_id: id,
        object_class: class,
        realm_id: realm,
        authority_scope_id: scope,
        content_digest_hex: digest,
        body_json: serde_json::to_string(&body).map_err(|e| AuthorityError::InvalidArgument {
            message: e.to_string(),
        })?,
        updated_seq,
    })
}

type RowParts = (String, String, String, String, Option<String>, Value);

fn row_parts<T: serde::Serialize + HasHeader>(
    class: &'static str,
    value: &T,
) -> Result<RowParts, AuthorityError> {
    let v = serde_json::to_value(value).map_err(|e| AuthorityError::InvalidArgument {
        message: e.to_string(),
    })?;
    let h = value.header_parts();
    Ok((class.to_owned(), h.0, h.1, h.2, None, v))
}

trait HasHeader {
    fn header_parts(&self) -> (String, String, String);
}

macro_rules! impl_header {
    ($t:ty) => {
        impl HasHeader for $t {
            fn header_parts(&self) -> (String, String, String) {
                (
                    self.header.realm_id.as_opaque().as_str().to_owned(),
                    self.header
                        .authority_scope_id
                        .as_opaque()
                        .as_str()
                        .to_owned(),
                    self.header.id.as_str().to_owned(),
                )
            }
        }
    };
}

impl_header!(Proposal);
impl_header!(ClinicalAssertion);
impl_header!(ActionAuditRecord);
impl_header!(IdentityAssertion);
impl_header!(IdentityMergeDecision);
impl_header!(EvaluationRecord);
impl_header!(Projection);
impl_header!(AmendmentRecord);

fn from_row(
    row: &AuthorityObjectRow,
    blobs: &impl BlobBackend,
) -> Result<StoredObject, AuthorityError> {
    let value: Value =
        serde_json::from_str(&row.body_json).map_err(|e| AuthorityError::Corrupt {
            message: format!("corrupt object body: {e}"),
        })?;
    match row.object_class.as_str() {
        "source" => {
            let mut source: SourceRecord =
                serde_json::from_value(value).map_err(|e| AuthorityError::Corrupt {
                    message: format!("corrupt source: {e}"),
                })?;
            let digest = source.content_digest.clone();
            let bytes = blobs.get_bytes(&digest)?;
            if DigestSha256::of(&bytes) != digest {
                return Err(AuthorityError::DigestMismatch);
            }
            source.bytes = bytes;
            Ok(StoredObject::Source(source))
        }
        "derived" => {
            let mut derived: DerivedSourceArtifact =
                serde_json::from_value(value).map_err(|e| AuthorityError::Corrupt {
                    message: format!("corrupt derived: {e}"),
                })?;
            let digest = derived.content_digest.clone();
            let bytes = blobs.get_bytes(&digest)?;
            if DigestSha256::of(&bytes) != digest {
                return Err(AuthorityError::DigestMismatch);
            }
            derived.bytes = bytes;
            Ok(StoredObject::Derived(derived))
        }
        "proposal" => Ok(StoredObject::Proposal(
            serde_json::from_value(value).map_err(|e| AuthorityError::Corrupt {
                message: e.to_string(),
            })?,
        )),
        "assertion" => Ok(StoredObject::Assertion(
            serde_json::from_value(value).map_err(|e| AuthorityError::Corrupt {
                message: e.to_string(),
            })?,
        )),
        "audit" => Ok(StoredObject::Audit(serde_json::from_value(value).map_err(
            |e| AuthorityError::Corrupt {
                message: e.to_string(),
            },
        )?)),
        "identity" => Ok(StoredObject::Identity(
            serde_json::from_value(value).map_err(|e| AuthorityError::Corrupt {
                message: e.to_string(),
            })?,
        )),
        "merge" => Ok(StoredObject::Merge(serde_json::from_value(value).map_err(
            |e| AuthorityError::Corrupt {
                message: e.to_string(),
            },
        )?)),
        "evaluation" => Ok(StoredObject::Evaluation(
            serde_json::from_value(value).map_err(|e| AuthorityError::Corrupt {
                message: e.to_string(),
            })?,
        )),
        "projection" => Ok(StoredObject::Projection(
            serde_json::from_value(value).map_err(|e| AuthorityError::Corrupt {
                message: e.to_string(),
            })?,
        )),
        "amendment" => Ok(StoredObject::Amendment(
            serde_json::from_value(value).map_err(|e| AuthorityError::Corrupt {
                message: e.to_string(),
            })?,
        )),
        other => Err(AuthorityError::Corrupt {
            message: format!("unknown object class {other}"),
        }),
    }
}
