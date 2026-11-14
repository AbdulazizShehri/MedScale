//! Federation authority (Spec 091, bounded bundle exchange).
//!
//! Core creates this vault's institution identity (ed25519; the secret stays
//! in the vault and is never exported), trusts or revokes peer institutions
//! explicitly, exports Core artifacts as signed and sequenced bundles
//! (policy-checked: never `local_phi`, never above the peer's ceiling) and
//! imports bundles only after checking format, recipient, trusted sender,
//! key, strict signature, sequence (replay guard), size and policy. Imports
//! create new local rows with provenance and never overwrite; tombstones
//! erase the bytes of items the sender withdrew. There is no central
//! service: bundles move out of band.

use std::collections::BTreeSet;

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::federation::{
    BUNDLE_BYTES_MAX, BUNDLE_FORMAT_VERSION, BUNDLE_ITEMS_MAX, BundleBody, BundleItem,
    FEDERATION_SCHEMA_VERSION, FederationActRequest, FederationActResult, FederationAction,
    FederationBundle, FederationIdentity, FederationPeer, FederationReceipt, FederationRefusal,
    FederationView, ImportedItem, ImportedState, PeerState, ProvenanceStep,
    validate_institution_id,
};
use medscale_contracts::objects::{DigestSha256, ObjectHeader, OpaqueId};
use medscale_contracts::privacy_gate::DataClass;
use medscale_keys::{generate_device_key, sign_device_payload, verify_device_signature};
use medscale_storage::{FederationChange, MetaError};

use super::data_sources::DataSources;
use super::extensions::valid_public_key;

fn meta_err(err: MetaError) -> AuthorityError {
    match err {
        MetaError::NotFound => AuthorityError::NotFound,
        MetaError::Conflict(message) => AuthorityError::Conflict { message },
        MetaError::UnsupportedSchema(message) => AuthorityError::UnsupportedSchema { message },
        MetaError::CorruptObjectBody(message) => AuthorityError::Corrupt { message },
        other => AuthorityError::Internal {
            message: other.to_string(),
        },
    }
}

fn invalid(message: impl Into<String>) -> AuthorityError {
    AuthorityError::InvalidArgument {
        message: message.into(),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

impl DataSources<'_> {
    fn fed_header(&self, id: OpaqueId) -> ObjectHeader {
        ObjectHeader {
            id,
            schema_version: FEDERATION_SCHEMA_VERSION,
            realm_id: self.realm.clone(),
            authority_scope_id: self.scope.clone(),
        }
    }

    fn fed_alloc(&self, prefix: &str) -> Result<OpaqueId, AuthorityError> {
        self.meta().alloc_federation_id(prefix).map_err(meta_err)
    }

    fn fed_receipt(
        &self,
        action: FederationAction,
        peer: Option<&str>,
    ) -> Result<FederationReceipt, AuthorityError> {
        Ok(FederationReceipt {
            header: self.fed_header(self.fed_alloc("federation-receipt")?),
            action,
            peer_institution: peer.map(str::to_owned),
            seq: None,
            item_digests: Vec::new(),
            tombstoned: Vec::new(),
            refusal: None,
        })
    }

    fn fed_result(receipt: FederationReceipt) -> FederationActResult {
        FederationActResult {
            receipt,
            identity: None,
            peer: None,
            bundle_json: None,
            imported: Vec::new(),
        }
    }

    fn fed_commit(
        &mut self,
        change: FederationChange<'_>,
        audit: &str,
    ) -> Result<(), AuthorityError> {
        let target = change.receipt.map(|r| r.header.id.clone());
        self.meta()
            .commit_federation_change(&change)
            .map_err(meta_err)?;
        self.audit(audit, target.into_iter().collect())
    }

    fn fed_refuse(
        &mut self,
        mut receipt: FederationReceipt,
        refusal: FederationRefusal,
    ) -> Result<FederationActResult, AuthorityError> {
        receipt.refusal = Some(refusal);
        self.fed_commit(
            FederationChange {
                receipt: Some(&receipt),
                ..FederationChange::default()
            },
            "federation.refused",
        )?;
        Ok(Self::fed_result(receipt))
    }

    fn fed_peer(&self, institution_id: &str) -> Result<Option<FederationPeer>, AuthorityError> {
        match self.meta().get_federation_peer(institution_id) {
            Ok(p) => Ok(Some(p)),
            Err(MetaError::NotFound) => Ok(None),
            Err(e) => Err(meta_err(e)),
        }
    }

    pub fn fed_create_identity(
        &mut self,
        institution_id: String,
    ) -> Result<FederationActResult, AuthorityError> {
        validate_institution_id(&institution_id).map_err(invalid)?;
        let (secret, public) = generate_device_key();
        let identity = FederationIdentity {
            header: self.fed_header(self.fed_alloc("federation-identity")?),
            institution_id,
            public_key_hex: public,
        };
        let receipt = self.fed_receipt(FederationAction::CreateIdentity, None)?;
        self.fed_commit(
            FederationChange {
                identity: Some((&identity, &secret)),
                receipt: Some(&receipt),
                ..FederationChange::default()
            },
            "federation.identity",
        )?;
        let mut result = Self::fed_result(receipt);
        result.identity = Some(identity);
        Ok(result)
    }

    pub fn fed_trust_peer(
        &mut self,
        institution_id: String,
        public_key_hex: String,
        ceiling: DataClass,
    ) -> Result<FederationActResult, AuthorityError> {
        if !valid_public_key(&public_key_hex) {
            return Err(invalid("peer key is not a valid ed25519 public key"));
        }
        let peer = FederationPeer {
            header: self.fed_header(self.fed_alloc("federation-peer")?),
            institution_id,
            public_key_hex,
            ceiling,
            state: PeerState::Trusted,
            last_received_seq: 0,
            last_sent_seq: 0,
            revision: 1,
        };
        peer.validate().map_err(invalid)?;
        let receipt = self.fed_receipt(FederationAction::TrustPeer, Some(&peer.institution_id))?;
        self.fed_commit(
            FederationChange {
                peer: Some((&peer, None)),
                receipt: Some(&receipt),
                ..FederationChange::default()
            },
            "federation.trust",
        )?;
        let mut result = Self::fed_result(receipt);
        result.peer = Some(peer);
        Ok(result)
    }

    pub fn fed_revoke_peer(
        &mut self,
        institution_id: &str,
    ) -> Result<FederationActResult, AuthorityError> {
        let old = self
            .fed_peer(institution_id)?
            .ok_or(AuthorityError::NotFound)?;
        if old.state == PeerState::Revoked {
            return Err(AuthorityError::Conflict {
                message: "peer is already revoked".to_owned(),
            });
        }
        let next = FederationPeer {
            state: PeerState::Revoked,
            revision: old.revision + 1,
            ..old.clone()
        };
        let receipt = self.fed_receipt(FederationAction::RevokePeer, Some(institution_id))?;
        self.fed_commit(
            FederationChange {
                peer: Some((&next, Some(old.revision))),
                receipt: Some(&receipt),
                ..FederationChange::default()
            },
            "federation.revoke",
        )?;
        let mut result = Self::fed_result(receipt);
        result.peer = Some(next);
        Ok(result)
    }

    /// Bytes, media type and effective class of a Core artifact in a Project.
    fn fed_artifact(
        &self,
        project_id: &OpaqueId,
        artifact_id: &OpaqueId,
    ) -> Option<(Vec<u8>, String, DataClass)> {
        let (bytes, media) = match self
            .store
            .get_scoped(artifact_id, &self.realm, &self.scope)
            .ok()?
        {
            super::store::StoredObject::Source(s) => (s.bytes.clone(), s.media_type.clone()),
            super::store::StoredObject::Derived(d) => {
                (d.bytes.clone(), "application/octet-stream".to_owned())
            }
            _ => return None,
        };
        let class = match self.meta().get_classification(project_id, artifact_id) {
            Ok(Some(row)) => row.data_class,
            _ => DataClass::LocalPhi,
        };
        Some((bytes, media, class))
    }

    pub fn fed_export(
        &mut self,
        project_id: &OpaqueId,
        to_institution: &str,
        artifact_ids: &[OpaqueId],
        tombstones: Vec<DigestSha256>,
    ) -> Result<FederationActResult, AuthorityError> {
        self.require_project(project_id)?;
        let mut receipt = self.fed_receipt(FederationAction::Export, Some(to_institution))?;
        let Some((identity, Some(secret))) =
            self.meta().get_federation_identity().map_err(meta_err)?
        else {
            return self.fed_refuse(receipt, FederationRefusal::NoIdentity);
        };
        let Some(peer) = self.fed_peer(to_institution)? else {
            return self.fed_refuse(receipt, FederationRefusal::PeerUnknown);
        };
        if peer.state == PeerState::Revoked {
            return self.fed_refuse(receipt, FederationRefusal::PeerRevoked);
        }
        if artifact_ids.len() > BUNDLE_ITEMS_MAX {
            return self.fed_refuse(receipt, FederationRefusal::TooLarge);
        }
        let mut items = Vec::new();
        let mut total = 0_u64;
        for artifact_id in artifact_ids {
            let Some((bytes, media_type, class)) = self.fed_artifact(project_id, artifact_id)
            else {
                return self.fed_refuse(receipt, FederationRefusal::ItemMissing);
            };
            if !peer.admits(class) {
                return self.fed_refuse(receipt, FederationRefusal::PolicyDenied);
            }
            total += bytes.len() as u64;
            if total > BUNDLE_BYTES_MAX / 2 {
                return self.fed_refuse(receipt, FederationRefusal::TooLarge);
            }
            let digest = DigestSha256::of(&bytes);
            items.push(BundleItem {
                media_type,
                bytes_hex: hex(&bytes),
                digest: digest.clone(),
                data_class: class,
                provenance: vec![ProvenanceStep {
                    institution_id: identity.institution_id.clone(),
                    artifact_id: artifact_id.clone(),
                    digest,
                }],
            });
        }
        let body = BundleBody {
            format_version: BUNDLE_FORMAT_VERSION,
            from_institution: identity.institution_id.clone(),
            to_institution: to_institution.to_owned(),
            seq: peer.last_sent_seq + 1,
            items,
            tombstones,
        };
        let signature_hex = sign_device_payload(&secret, &body.signing_payload()).map_err(|e| {
            AuthorityError::Internal {
                message: e.to_string(),
            }
        })?;
        let bundle = FederationBundle {
            body_json: String::from_utf8(body.canonical_bytes()).map_err(|e| {
                AuthorityError::Internal {
                    message: e.to_string(),
                }
            })?,
            signature_hex,
        };
        let next = FederationPeer {
            last_sent_seq: body.seq,
            revision: peer.revision + 1,
            ..peer.clone()
        };
        receipt.seq = Some(body.seq);
        receipt.item_digests = body.items.iter().map(|i| i.digest.clone()).collect();
        receipt.tombstoned = body.tombstones.clone();
        self.fed_commit(
            FederationChange {
                peer: Some((&next, Some(peer.revision))),
                receipt: Some(&receipt),
                ..FederationChange::default()
            },
            "federation.export",
        )?;
        let mut result = Self::fed_result(receipt);
        result.bundle_json =
            Some(
                serde_json::to_string(&bundle).map_err(|e| AuthorityError::Internal {
                    message: e.to_string(),
                })?,
            );
        Ok(result)
    }

    pub fn fed_import(
        &mut self,
        project_id: &OpaqueId,
        bundle_json: &str,
    ) -> Result<FederationActResult, AuthorityError> {
        self.require_project(project_id)?;
        let mut receipt = self.fed_receipt(FederationAction::Import, None)?;
        if bundle_json.len() as u64 > BUNDLE_BYTES_MAX {
            return self.fed_refuse(receipt, FederationRefusal::TooLarge);
        }
        let Ok(bundle) = serde_json::from_str::<FederationBundle>(bundle_json) else {
            return self.fed_refuse(receipt, FederationRefusal::BundleInvalid);
        };
        let Ok(body) = serde_json::from_str::<BundleBody>(&bundle.body_json) else {
            return self.fed_refuse(receipt, FederationRefusal::BundleInvalid);
        };
        if body.canonical_bytes() != bundle.body_json.as_bytes() {
            return self.fed_refuse(receipt, FederationRefusal::BundleInvalid);
        }
        receipt.peer_institution = Some(body.from_institution.clone());
        receipt.seq = Some(body.seq);
        if body.format_version != BUNDLE_FORMAT_VERSION {
            return self.fed_refuse(receipt, FederationRefusal::FormatUnsupported);
        }
        let Some((identity, _)) = self.meta().get_federation_identity().map_err(meta_err)? else {
            return self.fed_refuse(receipt, FederationRefusal::NoIdentity);
        };
        if body.to_institution != identity.institution_id {
            return self.fed_refuse(receipt, FederationRefusal::WrongRecipient);
        }
        let Some(peer) = self.fed_peer(&body.from_institution)? else {
            return self.fed_refuse(receipt, FederationRefusal::PeerUnknown);
        };
        if peer.state == PeerState::Revoked {
            return self.fed_refuse(receipt, FederationRefusal::PeerRevoked);
        }
        if verify_device_signature(
            &peer.public_key_hex,
            &body.signing_payload(),
            &bundle.signature_hex,
        )
        .is_err()
        {
            return self.fed_refuse(receipt, FederationRefusal::SignatureInvalid);
        }
        if body.seq <= peer.last_received_seq {
            return self.fed_refuse(receipt, FederationRefusal::Replay);
        }
        if body.items.len() > BUNDLE_ITEMS_MAX {
            return self.fed_refuse(receipt, FederationRefusal::TooLarge);
        }
        // Verify every item before writing anything.
        let mut decoded = Vec::new();
        for item in &body.items {
            let Some(bytes) = unhex(&item.bytes_hex) else {
                return self.fed_refuse(receipt, FederationRefusal::BundleInvalid);
            };
            if DigestSha256::of(&bytes) != item.digest
                || item.provenance.last().map(|p| &p.institution_id) != Some(&body.from_institution)
            {
                return self.fed_refuse(receipt, FederationRefusal::BundleInvalid);
            }
            if !peer.admits(item.data_class) {
                return self.fed_refuse(receipt, FederationRefusal::PolicyDenied);
            }
            decoded.push((item, bytes));
        }
        let existing = self.meta().list_imported_items().map_err(meta_err)?;
        let already: BTreeSet<String> = existing
            .iter()
            .filter(|r| {
                &r.item.project_id == project_id && r.item.from_institution == peer.institution_id
            })
            .map(|r| r.item.digest.to_hex())
            .collect();
        let mut new_items = Vec::new();
        for (item, bytes) in decoded {
            if already.contains(&item.digest.to_hex())
                || new_items
                    .iter()
                    .any(|(i, _): &(ImportedItem, Vec<u8>)| i.digest == item.digest)
            {
                continue;
            }
            new_items.push((
                ImportedItem {
                    header: self.fed_header(self.fed_alloc("imported-item")?),
                    project_id: project_id.clone(),
                    from_institution: peer.institution_id.clone(),
                    bundle_seq: body.seq,
                    media_type: item.media_type.clone(),
                    digest: item.digest.clone(),
                    data_class: item.data_class,
                    provenance: item.provenance.clone(),
                    state: ImportedState::Active,
                },
                bytes,
            ));
        }
        let tombstone_items: Vec<ImportedItem> = existing
            .into_iter()
            .map(|r| r.item)
            .filter(|i| {
                &i.project_id == project_id
                    && i.from_institution == peer.institution_id
                    && i.state == ImportedState::Active
                    && body.tombstones.contains(&i.digest)
            })
            .collect();
        let next = FederationPeer {
            last_received_seq: body.seq,
            revision: peer.revision + 1,
            ..peer.clone()
        };
        receipt.item_digests = new_items.iter().map(|(i, _)| i.digest.clone()).collect();
        receipt.tombstoned = tombstone_items.iter().map(|i| i.digest.clone()).collect();
        self.fed_commit(
            FederationChange {
                identity: None,
                peer: Some((&next, Some(peer.revision))),
                new_items: new_items.iter().map(|(i, b)| (i, b.as_slice())).collect(),
                tombstone_items: tombstone_items.iter().collect(),
                receipt: Some(&receipt),
            },
            "federation.import",
        )?;
        let mut result = Self::fed_result(receipt);
        result.imported = new_items.into_iter().map(|(i, _)| i).collect();
        Ok(result)
    }

    pub fn fed_act(
        &mut self,
        act: FederationActRequest,
    ) -> Result<FederationActResult, AuthorityError> {
        match act {
            FederationActRequest::CreateIdentity { institution_id } => {
                self.fed_create_identity(institution_id)
            }
            FederationActRequest::TrustPeer {
                institution_id,
                public_key_hex,
                ceiling,
            } => self.fed_trust_peer(institution_id, public_key_hex, ceiling),
            FederationActRequest::RevokePeer { institution_id } => {
                self.fed_revoke_peer(&institution_id)
            }
            FederationActRequest::Export {
                project_id,
                to_institution,
                artifact_ids,
                tombstones,
            } => self.fed_export(&project_id, &to_institution, &artifact_ids, tombstones),
            FederationActRequest::Import {
                project_id,
                bundle_json,
            } => self.fed_import(&project_id, &bundle_json),
        }
    }

    pub fn fed_view(&self) -> Result<FederationView, AuthorityError> {
        Ok(FederationView {
            identity: self
                .meta()
                .get_federation_identity()
                .map_err(meta_err)?
                .map(|(i, _)| i),
            peers: self.meta().list_federation_peers().map_err(meta_err)?,
            imported: self
                .meta()
                .list_imported_items()
                .map_err(meta_err)?
                .into_iter()
                .map(|r| r.item)
                .collect(),
            receipts: self.meta().list_federation_receipts().map_err(meta_err)?,
        })
    }
}
