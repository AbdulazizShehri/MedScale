//! Spec 065 CLI product-experience authority regression binding.

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
fn cli_reuses_facade_contracts_for_product_parity() {
    let root = repo_root();
    let session = std::fs::read_to_string(root.join("crates/medscale-core/src/cli_session.rs"))
        .expect("CliSession");
    let cli = std::fs::read_to_string(root.join("crates/medscale-cli/src/main.rs")).expect("CLI");
    for term in [
        "Capability::ListOutbox",
        "Capability::ListDisclosures",
        "Capability::GetFhirSupportMatrix",
    ] {
        assert!(session.contains(term), "missing facade contract: {term}");
    }
    for term in [
        "Commands::Status",
        "Commands::Capabilities",
        "Commands::Patient",
        "Commands::Actions",
        "Commands::Audit",
        "Commands::Fhir",
    ] {
        assert!(cli.contains(term), "missing CLI product path: {term}");
    }
}

#[test]
fn cli_still_has_no_direct_storage_key_or_network_privilege() {
    let root = repo_root();
    let manifest =
        std::fs::read_to_string(root.join("crates/medscale-cli/Cargo.toml")).expect("CLI manifest");
    for forbidden in [
        "medscale-storage",
        "medscale-keys",
        "medscale-network",
        "rusqlite",
        "ureq",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "CLI direct dependency admitted: {forbidden}"
        );
    }
}

#[test]
fn product_guide_preserves_privacy_and_transient_host_honesty() {
    let root = repo_root();
    let guide =
        std::fs::read_to_string(root.join("docs/product/CLI_PRODUCT_GUIDE.md")).expect("CLI guide");
    for required in [
        "REAL_PHI",
        "is not authorized",
        "transient in-process Core Host",
        "UNKNOWN",
        "is not a retry authorization",
        "not a full-conformance claim",
        "RELEASE_READY",
    ] {
        assert!(
            guide.contains(required),
            "missing CLI honesty text: {required}"
        );
    }
}
