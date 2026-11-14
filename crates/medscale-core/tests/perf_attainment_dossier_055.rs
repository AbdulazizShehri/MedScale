//! Spec 055 � performance attainment dossier honesty.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(name: &str) -> String {
    let root = repo_root();
    std::fs::read_to_string(root.join(name)).unwrap_or_else(|e| panic!("missing {name}: {e}"))
}

#[test]
fn dossier_and_binding_present_with_verdict() {
    let root = repo_root();
    assert!(
        root.join("evidence/055-perf-attainment/PERFORMANCE_DOSSIER.md")
            .is_file()
    );
    assert!(
        root.join("evidence/055-perf-attainment/BINDING.md")
            .is_file()
    );
    assert!(
        root.join("specs/055-perf-attainment-dossier/spec.md")
            .is_file()
    );
    assert!(
        root.join("docs/qualification/promotions/SPEC_055_PROMOTION.md")
            .is_file()
    );
    let dossier = read("evidence/055-perf-attainment/PERFORMANCE_DOSSIER.md");
    let binding = read("evidence/055-perf-attainment/BINDING.md");
    assert!(dossier.contains("PERFORMANCE_NON_ATTAINMENT"));
    assert!(dossier.contains("budgets_claimed_met: false"));
    assert!(dossier.contains("perf_budgets_attained_on_qualified_hardware"));
    for key in [
        "source_sha",
        "tree_sha",
        "cargo_lock_sha256",
        "toolchain_rustc",
        "target",
    ] {
        assert!(binding.contains(key), "binding missing {key}");
    }
}

#[test]
fn dossier_claims_no_attainment_or_release() {
    let dossier = read("evidence/055-perf-attainment/PERFORMANCE_DOSSIER.md");
    assert!(!dossier.contains("budgets_claimed_met=true"));
    assert!(!dossier.contains("budgets_claimed_met = true"));
    assert!(!dossier.contains("RELEASE_READY=true"));
    assert!(!dossier.contains("PERFORMANCE_ATTAINMENT\n"));
}
