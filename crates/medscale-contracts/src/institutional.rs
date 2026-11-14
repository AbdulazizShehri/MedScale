//! Institutional adapter contracts (Spec 090, first path: object storage).
//!
//! An institutional adapter is an explicit, optional, revocable path from a
//! Project to an organizational system. Personal and Lab operation never
//! need one. Every adapter declares its destination, data-class ceiling,
//! capabilities and a credential *handle* (a name; never a secret). Every
//! external write is an intent with a durable effect state
//! (`pending -> sent -> confirmed | failed | unknown`), an idempotency key
//! bound to the exact payload, and a receipt per step. A write whose
//! outcome is uncertain becomes `unknown` and is never retried blindly:
//! only a reconciliation that asks the destination about the idempotency
//! key can move it on.
//!
//! No network transport is admitted in this build (product egress is
//! default-deny); the only product transport reports `unavailable`.
//! Qualification uses an in-process institutional store with fault
//! injection.

use serde::{Deserialize, Serialize};

use crate::objects::{DigestSha256, EffectState, ObjectHeader, OpaqueId};
use crate::privacy_gate::DataClass;

pub const INSTITUTIONAL_SCHEMA_VERSION: u32 = 1;
pub const ADAPTER_NAME_MAX_CHARS: usize = 64;
pub const DESTINATION_MAX_CHARS: usize = 256;
pub const OBJECT_KEY_MAX_CHARS: usize = 256;
pub const PAYLOAD_BYTES_MAX: u64 = 64 * 1024 * 1024;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind {
    /// S3-compatible or similar object storage.
    ObjectStorage,
}

closed_vocabulary!(AdapterKind, "adapter kind", {
    ObjectStorage => "object_storage",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterCapability {
    PutObject,
    HeadObject,
}

closed_vocabulary!(AdapterCapability, "adapter capability", {
    PutObject => "put_object",
    HeadObject => "head_object",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterState {
    Active,
    Suspended,
    /// Terminal: nothing is sent again; history is kept.
    Revoked,
}

closed_vocabulary!(AdapterState, "adapter state", {
    Active => "active",
    Suspended => "suspended",
    Revoked => "revoked",
});

/// The declared configuration of one adapter revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterConfig {
    pub kind: AdapterKind,
    /// Declared destination (for example `s3://bucket/prefix`); recorded,
    /// never interpreted as a local path.
    pub destination: String,
    /// The most restrictive class this adapter may carry. `local_phi` is
    /// never allowed to leave.
    pub data_class_ceiling: DataClass,
    /// A credential handle name (`cred:<name>`), never a secret value.
    pub credential_handle: Option<String>,
    /// Sorted, unique.
    pub capabilities: Vec<AdapterCapability>,
}

impl AdapterConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.destination.trim().is_empty()
            || self.destination.chars().count() > DESTINATION_MAX_CHARS
            || self.destination.chars().any(char::is_control)
        {
            return Err("destination must be 1-256 printable characters".to_owned());
        }
        if self.data_class_ceiling == DataClass::LocalPhi {
            return Err("local_phi never leaves the vault through an adapter".to_owned());
        }
        if let Some(handle) = &self.credential_handle {
            let name = handle.strip_prefix("cred:").unwrap_or("");
            if name.is_empty()
                || name.len() > 64
                || !name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            {
                return Err("credential handle must look like cred:<name>".to_owned());
            }
        }
        if !self.capabilities.windows(2).all(|w| w[0] < w[1]) || self.capabilities.is_empty() {
            return Err("capabilities must be sorted, unique and non-empty".to_owned());
        }
        Ok(())
    }

    /// Whether data of `class` may use this adapter.
    #[must_use]
    pub const fn admits(&self, class: DataClass) -> bool {
        class.restrictiveness() <= self.data_class_ceiling.restrictiveness()
            && !matches!(class, DataClass::LocalPhi)
    }
}

/// One adapter, with its current configuration revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstitutionalAdapter {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub name: String,
    pub config: AdapterConfig,
    /// 1-based; every configuration change or rollback adds one.
    pub config_revision: u64,
    /// The configuration before the current one (for rollback).
    pub previous_config: Option<AdapterConfig>,
    pub state: AdapterState,
    pub revision: u64,
}

impl InstitutionalAdapter {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty()
            || self.name.chars().count() > ADAPTER_NAME_MAX_CHARS
            || !self
                .name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        {
            return Err("adapter name must be a plain identifier".to_owned());
        }
        self.config.validate()?;
        if let Some(p) = &self.previous_config {
            p.validate()?;
        }
        if self.config_revision == 0 || self.revision == 0 {
            return Err("revisions start at 1".to_owned());
        }
        Ok(())
    }
}

/// Why a write or reconciliation step did not proceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteRefusal {
    AdapterNotActive,
    CapabilityMissing,
    /// The payload's class is above the adapter's ceiling (or is local PHI).
    DataClassAboveCeiling,
    PayloadMissing,
    PayloadTooLarge,
    BadObjectKey,
    /// `unknown` needs reconciliation before anything else.
    UnknownRequiresReconcile,
    AlreadyConfirmed,
    /// The transport could not reach the destination; nothing was sent.
    TransportUnavailable,
}

closed_vocabulary!(WriteRefusal, "write refusal", {
    AdapterNotActive => "adapter_not_active",
    CapabilityMissing => "capability_missing",
    DataClassAboveCeiling => "data_class_above_ceiling",
    PayloadMissing => "payload_missing",
    PayloadTooLarge => "payload_too_large",
    BadObjectKey => "bad_object_key",
    UnknownRequiresReconcile => "unknown_requires_reconcile",
    AlreadyConfirmed => "already_confirmed",
    TransportUnavailable => "transport_unavailable",
});

/// Validates an object key: relative, no `..`, no control characters.
pub fn validate_object_key(key: &str) -> Result<(), String> {
    let ok = !key.is_empty()
        && key.chars().count() <= OBJECT_KEY_MAX_CHARS
        && !key.starts_with('/')
        && !key
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'));
    if ok {
        Ok(())
    } else {
        Err("object key must be a relative plain path".to_owned())
    }
}

/// The idempotency key of a write: adapter, destination, object key and
/// the exact payload digest. Replaying it can never store different bytes.
#[must_use]
pub fn idempotency_key(
    adapter_id: &OpaqueId,
    destination: &str,
    object_key: &str,
    payload: &DigestSha256,
) -> DigestSha256 {
    DigestSha256::of(
        format!(
            "MEDSCALE_IDEMPOTENCY_V1\n{}\n{destination}\n{object_key}\n{}\n",
            adapter_id.as_str(),
            payload.to_hex()
        )
        .as_bytes(),
    )
}

/// One external write, from intent to terminal state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalWriteIntent {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub adapter_id: OpaqueId,
    /// The adapter configuration revision the intent was bound to.
    pub config_revision: u64,
    pub destination: String,
    pub object_key: String,
    /// The Core artifact whose exact bytes are the payload.
    pub artifact_id: OpaqueId,
    pub payload_digest: DigestSha256,
    pub payload_bytes: u64,
    pub data_class: DataClass,
    pub idempotency_key: DigestSha256,
    pub state: EffectState,
    pub attempts: u32,
    pub revision: u64,
}

impl ExternalWriteIntent {
    pub fn validate(&self) -> Result<(), String> {
        validate_object_key(&self.object_key)?;
        if self.data_class == DataClass::LocalPhi {
            return Err("local_phi never leaves the vault".to_owned());
        }
        if self.idempotency_key
            != idempotency_key(
                &self.adapter_id,
                &self.destination,
                &self.object_key,
                &self.payload_digest,
            )
        {
            return Err("idempotency key is not bound to the payload".to_owned());
        }
        if self.revision == 0 {
            return Err("intent revision starts at 1".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterAction {
    Register,
    Reconfigure,
    Rollback,
    Suspend,
    Resume,
    Revoke,
    Intend,
    Send,
    Reconcile,
    Retry,
}

closed_vocabulary!(AdapterAction, "adapter action", {
    Register => "register",
    Reconfigure => "reconfigure",
    Rollback => "rollback",
    Suspend => "suspend",
    Resume => "resume",
    Revoke => "revoke",
    Intend => "intend",
    Send => "send",
    Reconcile => "reconcile",
    Retry => "retry",
});

/// Every adapter step, applied or refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterReceipt {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub adapter_id: OpaqueId,
    pub intent_id: Option<OpaqueId>,
    pub action: AdapterAction,
    pub from_state: Option<EffectState>,
    pub to_state: Option<EffectState>,
    pub refusal: Option<WriteRefusal>,
    /// What the destination reported, as a closed code (never raw text).
    pub transport_outcome: Option<TransportOutcome>,
}

/// What a transport reported for one call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportOutcome {
    Stored,
    AlreadyStored,
    /// The destination refused; nothing was stored.
    Rejected,
    /// Could not connect; nothing was sent.
    Unreachable,
    /// Sent, but no answer: the write may or may not have happened.
    TimedOut,
    PresentMatching,
    PresentDifferent,
    Absent,
}

closed_vocabulary!(TransportOutcome, "transport outcome", {
    Stored => "stored",
    AlreadyStored => "already_stored",
    Rejected => "rejected",
    Unreachable => "unreachable",
    TimedOut => "timed_out",
    PresentMatching => "present_matching",
    PresentDifferent => "present_different",
    Absent => "absent",
});

/// One adapter act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "act", deny_unknown_fields)]
pub enum AdapterActRequest {
    Register {
        project_id: OpaqueId,
        name: String,
        config: AdapterConfig,
    },
    Reconfigure {
        adapter_id: OpaqueId,
        config: AdapterConfig,
    },
    Rollback {
        adapter_id: OpaqueId,
    },
    SetState {
        adapter_id: OpaqueId,
        state: AdapterState,
    },
    Intend {
        adapter_id: OpaqueId,
        artifact_id: OpaqueId,
        object_key: String,
    },
    Send {
        intent_id: OpaqueId,
    },
    Reconcile {
        intent_id: OpaqueId,
    },
    Retry {
        intent_id: OpaqueId,
    },
}

/// What an act produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterActResult {
    pub adapter: Option<InstitutionalAdapter>,
    pub intent: Option<ExternalWriteIntent>,
    pub receipt: AdapterReceipt,
}

/// Everything recorded for one adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterView {
    pub adapter: InstitutionalAdapter,
    pub intents: Vec<ExternalWriteIntent>,
    pub receipts: Vec<AdapterReceipt>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> AdapterConfig {
        AdapterConfig {
            kind: AdapterKind::ObjectStorage,
            destination: "s3://lab-bucket/exports".to_owned(),
            data_class_ceiling: DataClass::ExternalDeidentified,
            credential_handle: Some("cred:lab-s3".to_owned()),
            capabilities: vec![AdapterCapability::PutObject, AdapterCapability::HeadObject],
        }
    }

    #[test]
    fn local_phi_never_leaves() {
        let mut c = config();
        c.validate().unwrap();
        assert!(c.admits(DataClass::Public) && c.admits(DataClass::ExternalDeidentified));
        assert!(!c.admits(DataClass::TeamProtected) && !c.admits(DataClass::LocalPhi));
        c.data_class_ceiling = DataClass::LocalPhi;
        assert!(c.validate().is_err());
    }

    #[test]
    fn credentials_are_handles_not_secrets() {
        let mut c = config();
        c.credential_handle = Some("AKIAIOSFODNN7EXAMPLE:secret".to_owned());
        assert!(c.validate().is_err());
        c.credential_handle = Some("cred:".to_owned());
        assert!(c.validate().is_err());
        c.credential_handle = None;
        c.validate().unwrap();
    }

    #[test]
    fn object_keys_are_relative_plain_paths() {
        for bad in ["", "/abs", "a/../b", "a//b", "a b", "../x", "x/."] {
            assert!(validate_object_key(bad).is_err(), "{bad:?}");
        }
        validate_object_key("exports/2026/table-1.json").unwrap();
    }

    #[test]
    fn idempotency_keys_bind_the_payload() {
        let a = OpaqueId::new("adapter-1");
        let k1 = idempotency_key(&a, "s3://b", "k", &DigestSha256::of(b"x"));
        let k2 = idempotency_key(&a, "s3://b", "k", &DigestSha256::of(b"y"));
        assert_ne!(k1, k2);
        assert_eq!(
            k1,
            idempotency_key(&a, "s3://b", "k", &DigestSha256::of(b"x"))
        );
    }
}
