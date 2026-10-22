//! Spec 063 Workflow Studio + Tasks/Messages authority regression binding.

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
fn workflow_tasks_messages_are_real_native_surfaces() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint"))
        .expect("desktop UI");
    for label in [
        "Review-first controlled-action studio · Synthetic demo",
        "Derived outbox review queue · No second task authority",
        "Local review previews · No external transport claim",
        "UNKNOWN policy",
        "Payload identity",
        "Authority boundary",
        "Derived review queue",
        "Message boundary",
    ] {
        assert!(app.contains(label), "missing 063 UI label: {label}");
    }
    assert!(app.contains("for item in root.workflow-steps"));
    assert!(app.contains("for item in root.workflow-tasks"));
    assert!(app.contains("for item in root.workflow-messages"));
}

#[test]
fn workflow_boundary_reuses_existing_outbox_and_effect_state() {
    let root = repo_root();
    let model =
        std::fs::read_to_string(root.join("crates/medscale-desktop/src/workflow_studio.rs"))
            .expect("workflow model");
    let manifest = std::fs::read_to_string(root.join("crates/medscale-desktop/Cargo.toml"))
        .expect("desktop manifest");
    for term in [
        "OutboxEntry",
        "EffectState::Pending",
        "EffectState::Sent",
        "EffectState::Confirmed",
        "EffectState::Failed",
        "EffectState::Unknown",
        "DigestSha256",
        "Blind retry is forbidden",
    ] {
        assert!(model.contains(term), "missing authority term: {term}");
    }
    assert!(!manifest.contains("medscale-storage"));
    assert!(!manifest.contains("medscale-network"));
}

#[test]
fn tasks_messages_do_not_claim_second_authority_or_transport() {
    let root = repo_root();
    let model =
        std::fs::read_to_string(root.join("crates/medscale-desktop/src/workflow_studio.rs"))
            .expect("workflow model");
    assert!(model.contains("does not invent a second canonical task store"));
    assert!(model.contains("local review previews only"));
    assert!(model.contains("No external messaging transport"));
    assert!(model.contains("no action was committed by Desktop"));
}

#[test]
fn spec_063_keeps_external_authority_and_mesc_out_of_scope() {
    let root = repo_root();
    let spec =
        std::fs::read_to_string(root.join("specs/063-workflow-studio-tasks-messages/spec.md"))
            .expect("Spec 063");
    let public_scope =
        std::fs::read_to_string(root.join("docs/qualification/INTEROPERABILITY_SCOPE.md"))
            .expect("public cross-project authority scope");
    assert!(spec.contains("Real PHI, MESC, NPHIES authorization, external messaging"));
    assert!(public_scope.contains("not MedScale runtime authority"));
}
