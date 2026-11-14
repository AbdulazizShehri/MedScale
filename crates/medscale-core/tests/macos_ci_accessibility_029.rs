//! Spec 029 macOS CI + accessibility honesty.

use medscale_contracts::fixture_ui::FixtureUiViewModel;
use medscale_core::build_doctor_report;

#[test]
fn release_qualification_reports_macos_ci_without_claiming_qualified_or_release() {
    let report = build_doctor_report(None, false, false);
    let rq = &report.release_qualification;
    assert!(rq.macos_ci_present);
    assert!(!rq.macos_qualified);
    assert!(!rq.release_ready);
    assert!(rq.is_honest_prep());
    assert!(
        rq.missing_evidence_classes
            .contains(&"macos_platform_product_qualification".to_owned())
    );
    assert!(
        !rq.missing_evidence_classes
            .contains(&"qualified_os_matrix_macos".to_owned()),
        "CI presence supersedes the old macOS-missing class name"
    );
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.contains("macos_qualified=false") && n.contains("Release qualification"))
    );
}

#[test]
fn accessibility_axis_is_honest_ready_base_not_wcag() {
    let report = build_doctor_report(None, false, false);
    let a11y = &report.accessibility;
    assert!(a11y.is_honest_ready_base());
    assert!(a11y.fixture_cli_labels_checked);
    assert!(a11y.cli_keyboard_path_documented);
    assert!(a11y.disclosure_clarity_checked);
    assert!(!a11y.wcag_conformance_claimed);
    assert!(a11y.final_v0_ui_present); // Spec 060 supersedes the historical no-final-v0 state.
    assert!(!a11y.release_ready);
    assert!(
        a11y.limitations
            .iter()
            .any(|l| l.to_lowercase().contains("wcag"))
    );
}

#[test]
fn fixture_ui_doctor_surfaces_required_labels() {
    let report = build_doctor_report(None, false, false);
    let vm = FixtureUiViewModel::from_doctor(&report);
    assert!(vm.has_required_a11y_labels());
    assert_eq!(vm.title, "doctor");
    assert!(vm.accessible_label.to_lowercase().contains("doctor"));
    assert!(vm.synthetic_only);
    assert!(!vm.real_phi_authorized);
}
