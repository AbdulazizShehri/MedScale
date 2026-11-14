//! Spec 023/028 doctor honesty for vault privacy axis.

use medscale_core::build_doctor_report;

#[test]
fn doctor_vault_privacy_023() {
    let report = build_doctor_report(None, false, false);
    let v = &report.vault_privacy;
    assert!(v.present);
    assert!(!v.private_data_ready);
    assert!(v.sealed_at_close);
    assert!(v.work_wipe_on_close);
    assert!(v.sqlcipher_enabled);
    assert!(v.open_work_page_encrypted);
    // Residual OS risk remains honest.
    assert!(v.open_work_plaintext_risk);
    let note = report
        .notes
        .iter()
        .any(|n| n.contains("PRIVATE_DATA_READY=false"));
    assert!(note, "doctor notes must mention PRIVATE_DATA_READY=false");
}

#[test]
fn doctor_os_keyring_028_honesty() {
    let report = build_doctor_report(None, false, false);
    let v = &report.vault_privacy;
    assert!(!v.private_data_ready);
    // Fields always present; values depend on host probe / MEDSCALE_FORCE_MEMORY_KEYSTORE.
    let _ = v.os_keyring_available;
    let _ = v.os_keyring_used;
    if v.os_keyring_used {
        assert!(v.os_keyring_available);
        assert_eq!(
            report.key_store,
            medscale_contracts::doctor::KeyStoreAvailability::OsStoreAvailable
        );
    }
    let note = report
        .notes
        .iter()
        .any(|n| n.contains("PRIVATE_DATA_READY=false") && n.contains("Spec 043"));
    assert!(
        note,
        "doctor notes must mention Spec 043 privacy honesty + PRIVATE_DATA_READY=false"
    );
}
