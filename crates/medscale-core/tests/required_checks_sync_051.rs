//! Spec 051 — REQUIRED_CHECKS packet sync with live CI job names.

use medscale_core::build_doctor_report;
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
fn required_checks_packet_doctor_honest() {
    let report = build_doctor_report(None, false, false);
    let rq = &report.release_qualification;
    assert!(rq.required_checks_packet_synced);
    assert!(rq.branch_protection_configured);
    assert!(!rq.release_ready);
    assert!(rq.is_honest_prep());
    assert!(
        !rq.missing_evidence_classes
            .contains(&"repo_branch_protection_required_checks".to_owned())
    );
}

#[test]
fn required_checks_packet_lists_perf_job() {
    let root = repo_root();
    let packet = std::fs::read_to_string(
        root.join("evidence/022-release-qualification-prep/REQUIRED_CHECKS.md"),
    )
    .expect("REQUIRED_CHECKS.md");
    assert!(
        packet.contains("perf delivery-plan scale (windows)"),
        "owner packet must list live Spec 042 CI job name"
    );
    assert!(packet.contains("rust (ubuntu-latest)"));
    assert!(packet.contains("rust (windows-latest)"));
    assert!(packet.contains("rust (macos-latest)"));
    assert!(packet.contains("cargo-deny"));
    assert!(packet.contains("supply-chain policy present"));
    assert!(packet.contains("CONFIGURED_RULESET_23259329"));
    assert!(packet.contains("Spec 051"));
}

#[test]
fn required_checks_owner_action_and_evidence_present() {
    let root = repo_root();
    assert!(
        root.join("evidence/051-required-checks-sync/SUMMARY.md")
            .is_file()
    );
    assert!(
        root.join("evidence/051-required-checks-sync/OWNER_BRANCH_PROTECTION_ACTION.md")
            .is_file()
    );
    assert!(
        root.join("docs/qualification/promotions/SPEC_051_PROMOTION.md")
            .is_file()
    );
    assert!(
        root.join("evidence/051-required-checks-sync/RULESET_QUALIFICATION_2026-09-14.md")
            .is_file()
    );
}
