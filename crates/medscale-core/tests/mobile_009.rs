//! Spec 009 mobile READY_BASE contracts.

use medscale_contracts::mobile::{
    KeychainSyncPolicy, MobileDoctorStatus, MobilePlatform, android_arm64_16kb_ffi_stub,
    ios_arm64_ffi_stub,
};
use medscale_contracts::objects::PlacementClass;
use medscale_core::build_doctor_report;
use medscale_keys::MobileKeyStorePolicy;

#[test]
fn mobile_doctor_ready_base_axes() {
    let m = MobileDoctorStatus::ready_base();
    assert!(m.present);
    assert!(!m.apps_shipped);
    assert_eq!(
        m.keychain_sync_policy,
        KeychainSyncPolicy::SynchronizableForbidden
    );
    assert_eq!(m.placement_class_native_runtime, PlacementClass::P2);
    let _ = MobilePlatform::Ios;
}

#[test]
fn ffi_stubs_incomplete_until_platform_evidence() {
    let ios = ios_arm64_ffi_stub();
    let and = android_arm64_16kb_ffi_stub();
    // READY_BASE stubs affirm panic/typed flags but remain incomplete (empty TBD fields).
    assert!(ios.no_panic_across_extern_c);
    assert!(and.abi_notes.contains("16KB"));
    assert!(!ios.is_complete());
    assert!(!and.is_complete());
}

#[test]
fn keystore_policy_forbids_icloud_sync() {
    let p = MobileKeyStorePolicy::ready_base();
    assert!(p.forbids_apple_icloud_key_sync());
    assert!(p.android_hardware_backed_preferred);
}

#[test]
fn doctor_json_includes_mobile_axis() {
    let report = build_doctor_report(None, false, false);
    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains("mobile"));
    assert!(json.contains("synchronizable_forbidden") || json.contains("keychain_sync_policy"));
    assert!(!report.mobile.apps_shipped);
}
