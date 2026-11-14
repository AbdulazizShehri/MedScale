//! Federation contracts (Spec 091, bounded bundle exchange).
//!
//! Federation here is controlled exchange between institutions without a
//! central MedScale service: each vault has an institution identity (an
//! ed25519 key whose secret never leaves the vault or its backups), trusts
//! peer institutions explicitly with a data-class ceiling, and exchanges
//! signed, sequenced bundles out of band (files the users move). A bundle
//! carries items with their provenance and tombstones for items the sender
//! withdrew. Import verifies everything and never overwrites local data.
//!
//! ```text
//! connectivity != trust (peers are added explicitly)
//! local artifact != exported item (signed, provenance-bound)
//!   != imported item (a new local row; the source stays the peer's)
//! ```
//!
//! Raw PHI never federates: `local_phi` data cannot be exported and a peer
//! ceiling cannot be `local_phi`. Partial or federated analysis is not part
//! of this slice.

use serde::{Deserialize, Serialize};

use crate::objects::{DigestSha256, ObjectHeader, OpaqueId};
use crate::privacy_gate::DataClass;

pub const FEDERATION_SCHEMA_VERSION: u32 = 1;
/// Bundle format this build writes and reads.
pub const BUNDLE_FORMAT_VERSION: u32 = 1;
pub const BUNDLE_SIGN_DOMAIN: &str = "MEDSCALE_FEDERATION_BUNDLE_V1";
pub const BUNDLE_ITEMS_MAX: usize = 64;
pub const BUNDLE_BYTES_MAX: u64 = 32 * 1024 * 1024;
pub const INSTITUTION_ID_MAX_CHARS: usize = 64;

macro_rules! closed_vocabulary {
    ($name:ident, $what:literal, { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            pub fn parse(value: &str) -> Result<Self, String> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    other => Err(format!(concat!("unknown ", $what, " {}"), other)),
                }
            }
        }
    };
}

/// A plain lowercase institution identifier.
pub fn validate_institution_id(id: &str) -> Result<(), String> {
    let ok = !id.is_empty()
        && id.chars().count() <= INSTITUTION_ID_MAX_CHARS
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-'))
        && id.starts_with(|c: char| c.is_ascii_lowercase());
    if ok {
        Ok(())
    } else {
        Err("institution id must be a plain lowercase identifier".to_owned())
    }
}

/// This vault's institution identity (public part; the secret is stored
/// separately and never exported).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FederationIdentity {
    pub header: ObjectHeader,
    pub institution_id: String,
    pub public_key_hex: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerState {
    Trusted,
    /// Terminal: nothing is exchanged again.
    Revoked,
}

closed_vocabulary!(PeerState, "peer state", {
    Trusted => "trusted",
    Revoked => "revoked",
});

/// A peer institution trusted explicitly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FederationPeer {
    pub header: ObjectHeader,
    pub institution_id: String,
    pub public_key_hex: String,
    /// The most restrictive class exchanged with this peer (never
    /// `local_phi`).
    pub ceiling: DataClass,
    pub state: PeerState,
    /// Highest bundle sequence accepted from this peer (replay guard).
    pub last_received_seq: u64,
    /// Last bundle sequence sent to this peer.
    pub last_sent_seq: u64,
    pub revision: u64,
}

impl FederationPeer {
    pub fn validate(&self) -> Result<(), String> {
        validate_institution_id(&self.institution_id)?;
        if self.public_key_hex.len() != 64
            || !self
                .public_key_hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("peer key must be 64 lowercase hex digits".to_owned());
        }
        if self.ceiling == DataClass::LocalPhi {
            return Err("local_phi never federates".to_owned());
        }
        if self.revision == 0 {
            return Err("peer revision starts at 1".to_owned());
        }
        Ok(())
    }

    #[must_use]
    pub const fn admits(&self, class: DataClass) -> bool {
        !matches!(class, DataClass::LocalPhi)
            && class.restrictiveness() <= self.ceiling.restrictiveness()
    }
}

/// One step of an item's history across institutions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceStep {
    pub institution_id: String,
    pub artifact_id: OpaqueId,
    pub digest: DigestSha256,
}

/// One exported artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleItem {
    pub media_type: String,
    pub bytes_hex: String,
    pub digest: DigestSha256,
    pub data_class: DataClass,
    /// Oldest first; the last step is the sender.
    pub provenance: Vec<ProvenanceStep>,
}

/// The signed content of a bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleBody {
    pub format_version: u32,
    pub from_institution: String,
    pub to_institution: String,
    /// Strictly increasing per (sender, receiver).
    pub seq: u64,
    pub items: Vec<BundleItem>,
    /// Digests of items the sender withdraws.
    pub tombstones: Vec<DigestSha256>,
}

impl BundleBody {
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// The exact bytes signed: domain, then the body digest.
    #[must_use]
    pub fn signing_payload(&self) -> Vec<u8> {
        format!(
            "{BUNDLE_SIGN_DOMAIN}\n{}\n",
            DigestSha256::of(&self.canonical_bytes()).to_hex()
        )
        .into_bytes()
    }
}

/// A bundle as moved between institutions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FederationBundle {
    pub body_json: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportedState {
    Active,
    /// Withdrawn by the sender; bytes erased, record kept.
    Tombstoned,
}

closed_vocabulary!(ImportedState, "imported state", {
    Active => "active",
    Tombstoned => "tombstoned",
});

/// An item received from a peer: a new local row, never an overwrite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportedItem {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub from_institution: String,
    pub bundle_seq: u64,
    pub media_type: String,
    pub digest: DigestSha256,
    pub data_class: DataClass,
    pub provenance: Vec<ProvenanceStep>,
    pub state: ImportedState,
}

/// Why an exchange step was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FederationRefusal {
    NoIdentity,
    PeerUnknown,
    PeerRevoked,
    /// Policy: an item's class is above the peer ceiling, or is local PHI.
    PolicyDenied,
    BundleInvalid,
    FormatUnsupported,
    WrongRecipient,
    SignatureInvalid,
    /// The sequence was already seen (or went backwards).
    Replay,
    ItemMissing,
    TooLarge,
}

closed_vocabulary!(FederationRefusal, "federation refusal", {
    NoIdentity => "no_identity",
    PeerUnknown => "peer_unknown",
    PeerRevoked => "peer_revoked",
    PolicyDenied => "policy_denied",
    BundleInvalid => "bundle_invalid",
    FormatUnsupported => "format_unsupported",
    WrongRecipient => "wrong_recipient",
    SignatureInvalid => "signature_invalid",
    Replay => "replay",
    ItemMissing => "item_missing",
    TooLarge => "too_large",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FederationAction {
    CreateIdentity,
    TrustPeer,
    RevokePeer,
    Export,
    Import,
}

closed_vocabulary!(FederationAction, "federation action", {
    CreateIdentity => "create_identity",
    TrustPeer => "trust_peer",
    RevokePeer => "revoke_peer",
    Export => "export",
    Import => "import",
});

/// Every federation step, applied or refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FederationReceipt {
    pub header: ObjectHeader,
    pub action: FederationAction,
    pub peer_institution: Option<String>,
    pub seq: Option<u64>,
    pub item_digests: Vec<DigestSha256>,
    pub tombstoned: Vec<DigestSha256>,
    pub refusal: Option<FederationRefusal>,
}

/// One federation act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "act", deny_unknown_fields)]
pub enum FederationActRequest {
    CreateIdentity {
        institution_id: String,
    },
    TrustPeer {
        institution_id: String,
        public_key_hex: String,
        ceiling: DataClass,
    },
    RevokePeer {
        institution_id: String,
    },
    /// Exports Core artifacts of `project_id` (and withdraws earlier ones).
    Export {
        project_id: OpaqueId,
        to_institution: String,
        artifact_ids: Vec<OpaqueId>,
        tombstones: Vec<DigestSha256>,
    },
    /// Imports a bundle into `project_id`.
    Import {
        project_id: OpaqueId,
        bundle_json: String,
    },
}

/// What an act produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FederationActResult {
    pub receipt: FederationReceipt,
    pub identity: Option<FederationIdentity>,
    pub peer: Option<FederationPeer>,
    /// The signed bundle, for the host to hand over out of band.
    pub bundle_json: Option<String>,
    pub imported: Vec<ImportedItem>,
}

/// Everything recorded for federation in this vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FederationView {
    pub identity: Option<FederationIdentity>,
    pub peers: Vec<FederationPeer>,
    pub imported: Vec<ImportedItem>,
    pub receipts: Vec<FederationReceipt>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(ceiling: DataClass) -> FederationPeer {
        FederationPeer {
            header: ObjectHeader {
                id: OpaqueId::new("federation-peer-1"),
                schema_version: FEDERATION_SCHEMA_VERSION,
                realm_id: crate::objects::RealmId::new("r"),
                authority_scope_id: crate::objects::AuthorityScopeId::new("s"),
            },
            institution_id: "hospital-b".to_owned(),
            public_key_hex: "ab".repeat(32),
            ceiling,
            state: PeerState::Trusted,
            last_received_seq: 0,
            last_sent_seq: 0,
            revision: 1,
        }
    }

    #[test]
    fn raw_phi_never_federates() {
        let p = peer(DataClass::ExternalDeidentified);
        p.validate().unwrap();
        assert!(p.admits(DataClass::Public) && p.admits(DataClass::ExternalDeidentified));
        assert!(!p.admits(DataClass::TeamProtected) && !p.admits(DataClass::LocalPhi));
        assert!(peer(DataClass::LocalPhi).validate().is_err());
    }

    #[test]
    fn bundle_signatures_cover_the_exact_body() {
        let body = BundleBody {
            format_version: BUNDLE_FORMAT_VERSION,
            from_institution: "hospital-a".to_owned(),
            to_institution: "hospital-b".to_owned(),
            seq: 1,
            items: Vec::new(),
            tombstones: Vec::new(),
        };
        let mut other = body.clone();
        other.seq = 2;
        assert_ne!(body.signing_payload(), other.signing_payload());
        assert!(
            String::from_utf8(body.signing_payload())
                .unwrap()
                .starts_with(BUNDLE_SIGN_DOMAIN)
        );
    }

    #[test]
    fn institution_ids_are_plain() {
        for bad in ["", "Hospital", "a b", "-x", "a/b"] {
            assert!(validate_institution_id(bad).is_err(), "{bad:?}");
        }
        validate_institution_id("hospital-a.example").unwrap();
    }
}
