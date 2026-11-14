//! Mobile surface contracts (Spec 009 READY_BASE).

use serde::{Deserialize, Serialize};

use crate::ffi_policy::FfiAdmissionRecord;
use crate::objects::PlacementClass;

/// Mobile host platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MobilePlatform {
    Ios,
    Android,
}

/// Apple Keychain sync policy (authoritative platform constraint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeychainSyncPolicy {
    /// Required: kSecAttrSynchronizable must be false.
    SynchronizableForbidden,
    Deferred,
}

/// Android keystore backing preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AndroidKeystoreBacking {
    HardwareBackedPreferred,
    SoftwareAllowed,
    Deferred,
}

/// Doctor axis for mobile readiness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobileDoctorStatus {
    pub present: bool,
    pub apps_shipped: bool,
    pub keychain_sync_policy: KeychainSyncPolicy,
    pub android_keystore_backing: AndroidKeystoreBacking,
    pub page_size_16kb_claim: String,
    pub sideload_posture: String,
    pub ffi_boundary: String,
    pub placement_class_native_runtime: PlacementClass,
}

impl MobileDoctorStatus {
    /// Spec 009 READY_BASE default (no apps; policy stubs present).
    #[must_use]
    pub fn ready_base() -> Self {
        Self {
            present: true,
            apps_shipped: false,
            keychain_sync_policy: KeychainSyncPolicy::SynchronizableForbidden,
            android_keystore_backing: AndroidKeystoreBacking::HardwareBackedPreferred,
            page_size_16kb_claim: "documented_not_platform_qualified".to_owned(),
            sideload_posture: "files_saf_design_only".to_owned(),
            ffi_boundary: "contracts_stub_no_uniffi".to_owned(),
            placement_class_native_runtime: PlacementClass::P2,
        }
    }
}

/// Incomplete FFI admission templates for future mobile ABIs.
#[must_use]
pub fn ios_arm64_ffi_stub() -> FfiAdmissionRecord {
    FfiAdmissionRecord {
        component_name: "medscale-ffi-ios-stub".to_owned(),
        upstream_url: "https://github.com/AbdulazizShehri/MedScale".to_owned(),
        revision: "spec-009-ready-base".to_owned(),
        build_flags: "TBD_when_uniffi_admitted".to_owned(),
        abi_notes: "arm64-apple-ios; no panic across extern C".to_owned(),
        ownership_freeing: String::new(),
        thread_affinity: "main_or_documented_worker".to_owned(),
        no_panic_across_extern_c: true,
        typed_errors: true,
        allocator_assumptions: "rust_owned".to_owned(),
        arches: vec!["aarch64-apple-ios".to_owned()],
        placement_class: PlacementClass::P2,
        fuzz_sanitizer_evidence: "pending_platform_qualified".to_owned(),
        sbom_path: String::new(),
        update_strategy: "pin_in_owning_spec".to_owned(),
        exit_strategy: "remove_feature_flag".to_owned(),
    }
}

#[must_use]
pub fn android_arm64_16kb_ffi_stub() -> FfiAdmissionRecord {
    FfiAdmissionRecord {
        component_name: "medscale-ffi-android-stub".to_owned(),
        upstream_url: "https://github.com/AbdulazizShehri/MedScale".to_owned(),
        revision: "spec-009-ready-base".to_owned(),
        build_flags: "TBD_16kb_page_size_link".to_owned(),
        abi_notes: "arm64-v8a; Android 16KB page-size compatibility required before ship"
            .to_owned(),
        ownership_freeing: String::new(),
        thread_affinity: "jni_documented".to_owned(),
        no_panic_across_extern_c: true,
        typed_errors: true,
        allocator_assumptions: "rust_owned".to_owned(),
        arches: vec!["aarch64-linux-android".to_owned()],
        placement_class: PlacementClass::P2,
        fuzz_sanitizer_evidence: "pending_platform_qualified".to_owned(),
        sbom_path: String::new(),
        update_strategy: "pin_in_owning_spec".to_owned(),
        exit_strategy: "remove_feature_flag".to_owned(),
    }
}
