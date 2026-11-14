//! Spec 058 portable release package qualification honesty.

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
fn doctor_closes_only_proven_package_residuals() {
    let report = build_doctor_report(None, false, false);
    let rq = &report.release_qualification;
    assert!(rq.portable_release_package_qualified);
    assert!(rq.package_lifecycle_qualified);
    assert!(rq.package_upgrade_rollback_scaffold_present);
    assert!(!rq.release_ready);
    assert!(rq.is_honest_prep());
    assert!(
        !rq.missing_evidence_classes
            .contains(&"reproducible_release_package_contents".to_owned())
    );
    assert!(
        !rq.missing_evidence_classes
            .contains(&"release_package_upgrade_rollback_proof".to_owned())
    );
    for residual in [
        "release_sbom_signing_provenance",
        "checksums_provenance_signing_verification",
        "perf_budgets_attained_on_qualified_hardware",
        "macos_platform_product_qualification",
        "wcag_final_v0_ui_accessibility_qualification",
    ] {
        assert!(rq.missing_evidence_classes.contains(&residual.to_owned()));
    }
    // Spec 059 terminal audit closes this repository-owned residual after Spec 058.
    assert!(rq.material_findings_clearance);
    assert!(
        !rq.missing_evidence_classes
            .contains(&"unresolved_material_findings_clearance".to_owned())
    );
}

#[test]
fn package_scripts_encode_fail_closed_honesty() {
    let root = repo_root();
    let builder = std::fs::read_to_string(root.join("scripts/build-portable-release-package.ps1"))
        .expect("read builder");
    let verifier =
        std::fs::read_to_string(root.join("scripts/verify-portable-release-package.ps1"))
            .expect("read verifier");
    let installer =
        std::fs::read_to_string(root.join("scripts/install-portable-release-package.ps1"))
            .expect("read installer");
    let qualifier =
        std::fs::read_to_string(root.join("scripts/qualify-portable-release-package.ps1"))
            .expect("read qualifier");

    assert!(builder.contains("IsPathRooted"));
    assert!(qualifier.contains("IsPathRooted"));
    assert!(builder.contains("CompressionLevel]::NoCompression"));
    assert!(builder.contains("1980-01-01T00:00:00Z"));
    assert!(builder.contains("reproducible_binary_build = $false"));
    assert!(verifier.contains("package file inventory mismatch"));
    assert!(verifier.contains("payload hash mismatch"));
    assert!(verifier.contains("SBOM source SHA mismatch"));
    assert!(verifier.contains("[string[]]$CommandArgs"));
    assert!(verifier.contains("& $path @CommandArgs | Out-Null"));
    assert!(!verifier.contains("[string[]]$Args"));
    assert!(installer.contains("ValidateSet('install','upgrade','rollback')"));
    assert!(installer.contains("rollback unavailable: previous_slot absent"));
    assert!(qualifier.contains("deterministic package assembly failed"));
    assert!(qualifier.contains("install_upgrade_rollback_qualified = $true"));
    assert!(qualifier.contains("release_ready = $false"));
    let sbom_generator = std::fs::read_to_string(root.join("scripts/generate-release-sbom.ps1"))
        .expect("read SBOM generator");
    assert!(sbom_generator.contains("public_project_license'; value = 'Apache-2.0"));
    assert!(sbom_generator.contains("value = $buildOs"));
    assert!(sbom_generator.contains("value = $targetTriple"));
}
