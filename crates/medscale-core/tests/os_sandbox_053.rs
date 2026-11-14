//! Spec 053 - doctor honesty for Linux seccomp composition.

use medscale_core::{build_doctor_report, privacy_proof_artifact_present};
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn doctor_linux_seccomp_composition_honest() {
    let report = build_doctor_report(None, false, privacy_proof_artifact_present());
    assert!(report.os_sandbox.linux_seccomp_composition_measured);
    assert!(report.os_sandbox.linux_landlock_composition_measured);
    assert!(!report.os_sandbox.platform_qualified);
    assert!(report.os_sandbox.is_honest_ready_base());
}

#[test]
fn evidence_and_promotion_present() {
    let root = repo_root();
    assert!(
        root.join("evidence/053-linux-seccomp-composition/SUMMARY.md")
            .is_file()
    );
    assert!(
        root.join("evidence/053-linux-seccomp-composition/LINUX_SECCOMP_COMPOSITION_MEASURED.md")
            .is_file()
    );
    assert!(
        root.join("evidence/053-linux-seccomp-composition/LIMITATIONS.md")
            .is_file()
    );
    assert!(
        root.join("docs/qualification/promotions/SPEC_053_PROMOTION.md")
            .is_file()
    );
}
