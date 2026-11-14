//! Online pack acquisition contracts (Spec 015 READY_BASE — deny path).

use serde::{Deserialize, Serialize};

/// Doctor axis for online pack ecosystem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnlinePacksDoctorStatus {
    pub present: bool,
    pub online_download_authorized: bool,
    pub broker_required: bool,
    pub hf_runtime_required: bool,
    pub tuf_root_admitted: bool,
    pub distribution_channel: String,
}

impl OnlinePacksDoctorStatus {
    /// Spec 015 READY_BASE: contracts present; online denied.
    #[must_use]
    pub fn ready_base() -> Self {
        Self {
            present: true,
            online_download_authorized: false,
            broker_required: true,
            hf_runtime_required: false,
            tuf_root_admitted: false,
            distribution_channel: "offline_pack_v0_only".to_owned(),
        }
    }
}

/// Mobile / arch constraints carried on online acquire requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnlinePackFormatConstraints {
    pub chunked_resumable: bool,
    pub per_arch_artifacts: bool,
    pub android_16kb_page_size: bool,
    pub offline_verify_before_install: bool,
    pub no_icloud_key_sync: bool,
}

impl OnlinePackFormatConstraints {
    /// Defaults from Spec 009 pack-format constraints evidence.
    #[must_use]
    pub fn from_spec_009() -> Self {
        Self {
            chunked_resumable: true,
            per_arch_artifacts: true,
            android_16kb_page_size: true,
            offline_verify_before_install: true,
            no_icloud_key_sync: true,
        }
    }
}

/// Request to acquire a pack via online distribution (gated).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnlinePackAcquireRequest {
    pub pack_id: String,
    pub version: String,
    pub distribution_uri: String,
    pub broker_required: bool,
    pub format: OnlinePackFormatConstraints,
}

/// TUF / offline-root policy stub (no roots admitted in READY_BASE).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineRootPolicy {
    pub roots_admitted: bool,
    pub update_strategy: String,
    pub exit_strategy: String,
}

impl OfflineRootPolicy {
    #[must_use]
    pub fn ready_base() -> Self {
        Self {
            roots_admitted: false,
            update_strategy: "pin_in_owning_spec_when_gate_closes".to_owned(),
            exit_strategy: "remove_online_feature_flag".to_owned(),
        }
    }
}
