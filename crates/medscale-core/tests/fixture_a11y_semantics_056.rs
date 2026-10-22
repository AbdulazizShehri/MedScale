//! Spec 056 - fixture accessibility semantics deepening (honesty, not WCAG).

use medscale_contracts::doctor::AccessibilityDoctorStatus;
use medscale_contracts::fixture_ui::{
    FixtureA11ySemantics, FixtureUiRole, FixtureUiViewModel, FixtureViewState,
};
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
fn doctor_semantics_flag_honest_without_wcag_claim() {
    let report = build_doctor_report(None, false, false);
    let a11y = &report.accessibility;
    assert!(a11y.is_honest_ready_base());
    assert!(a11y.fixture_state_semantics_checked);
    assert!(!a11y.wcag_conformance_claimed);
    assert!(a11y.final_v0_ui_present); // Spec 060 native UI now exists; WCAG remains unclaimed.
    assert!(!a11y.release_ready);
    let status = AccessibilityDoctorStatus::ready_base();
    assert!(status.is_honest_ready_base());
}

#[test]
fn every_lifecycle_state_has_canonical_announcement() {
    let states = FixtureViewState::all();
    assert_eq!(states.len(), 6);
    for state in states {
        let sem = FixtureA11ySemantics::for_surface(
            FixtureUiRole::Status,
            0,
            "surface, 1 of 1".to_owned(),
            *state,
        );
        assert!(sem.is_honest());
        assert_eq!(sem.announcement, state.canonical_announcement());
    }
    // Error/conflict states must disclose that no action was taken.
    assert!(
        FixtureViewState::Error
            .canonical_announcement()
            .contains("No action was taken")
    );
    assert!(
        FixtureViewState::Conflict
            .canonical_announcement()
            .contains("No action was taken")
    );
}

#[test]
fn dishonest_semantics_rejected() {
    let mut sem = FixtureA11ySemantics::for_surface(
        FixtureUiRole::Document,
        1,
        "timeline, 2 of 4".to_owned(),
        FixtureViewState::Ready,
    );
    sem.announcement = "Something else".to_owned();
    assert!(!sem.is_honest());
    let legacy = FixtureA11ySemantics::default();
    assert!(!legacy.is_honest());
}

#[test]
fn doctor_view_model_carries_honest_semantics() {
    let report = build_doctor_report(None, false, false);
    let vm = FixtureUiViewModel::from_doctor(&report);
    assert!(vm.has_required_a11y_semantics());
    assert_eq!(vm.semantics.keyboard_focus_index, 0);
}

#[test]
fn legacy_view_model_json_without_semantics_still_parses() {
    let legacy = serde_json::json!({
        "surface": "doctor",
        "synthetic_only": true,
        "real_phi_authorized": false,
        "title": "doctor",
        "accessible_label": "MedScale doctor status",
        "body_json": null,
    });
    let vm: FixtureUiViewModel = serde_json::from_value(legacy).expect("legacy JSON parses");
    assert!(vm.has_required_a11y_labels());
    assert!(!vm.has_required_a11y_semantics());
}

#[test]
fn spec_and_evidence_present() {
    let root = repo_root();
    assert!(
        root.join("specs/056-fixture-a11y-semantics/spec.md")
            .is_file()
    );
    assert!(
        root.join("evidence/056-fixture-a11y-semantics/SUMMARY.md")
            .is_file()
    );
    assert!(
        root.join("docs/qualification/promotions/SPEC_056_PROMOTION.md")
            .is_file()
    );
}
