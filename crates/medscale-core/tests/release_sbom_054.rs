//! Spec 054 — native/full release SBOM qualification: generator determinism,
//! verifier positive + negative paths, committed-document self-consistency,
//! and doctor honesty (qualified=true, release_ready=false).

use medscale_core::build_doctor_report;
use medscale_core::release_sbom::{
    BuildEnvironment, ExpectedBinding, NativeDependency, PackAsset, ReleaseArtifact,
    ReleaseSbomInput, RustDependency, WorkspacePackage, generate_release_sbom, verify_release_sbom,
};
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn fixture_input() -> ReleaseSbomInput {
    ReleaseSbomInput {
        source_sha: "a".repeat(40),
        tree_sha: "b".repeat(40),
        cargo_lock_sha256: "c".repeat(64),
        build_env: BuildEnvironment {
            os: "windows".to_owned(),
            os_version: "11".to_owned(),
            target_triple: "x86_64-pc-windows-msvc".to_owned(),
            rustc: "1.89.0".to_owned(),
            cargo: "1.89.0".to_owned(),
        },
        workspace_packages: vec![WorkspacePackage {
            name: "medscale-core".to_owned(),
            version: "0.1.0".to_owned(),
            license: None,
        }],
        rust_deps: vec![RustDependency {
            name: "serde".to_owned(),
            version: "1.0.0".to_owned(),
            license: Some("MIT".to_owned()),
        }],
        native_deps: vec![NativeDependency {
            name: "sqlite".to_owned(),
            classification: "bundled-source".to_owned(),
            detail: "via rusqlite bundled".to_owned(),
        }],
        artifacts: vec![ReleaseArtifact {
            name: "medscale-cli".to_owned(),
            sha256: "d".repeat(64),
        }],
        pack_assets: vec![PackAsset {
            name: "pack-v0".to_owned(),
            kind: "offline-pack".to_owned(),
            rights_note: "synthetic fixture only".to_owned(),
        }],
    }
}

fn fixture_expected() -> ExpectedBinding {
    ExpectedBinding {
        source_sha: "a".repeat(40),
        tree_sha: "b".repeat(40),
        cargo_lock_sha256: "c".repeat(64),
        workspace_package_names: vec!["medscale-core".to_owned()],
    }
}

#[test]
fn generator_is_deterministic() {
    let input = fixture_input();
    let first = generate_release_sbom(&input);
    assert_eq!(first, generate_release_sbom(&input));
    assert!(first.contains("\"medscale:public_project_license\""));
    assert!(first.contains("\"Apache-2.0\""));
}

#[test]
fn verifier_accepts_fixture_document() {
    let doc = generate_release_sbom(&fixture_input());
    let report = verify_release_sbom(&doc, &fixture_expected());
    assert!(report.ok, "unexpected failures: {:?}", report.failures);
    assert!(report.component_count >= 5);
    assert_eq!(report.document_sha256.len(), 64);
}

#[test]
fn verifier_rejects_malformed_json() {
    let report = verify_release_sbom("{not json", &fixture_expected());
    assert!(!report.ok);
    assert!(report.failures.iter().any(|f| f.starts_with("parse_error")));
}

#[test]
fn verifier_rejects_tampered_component() {
    let mut value: serde_json::Value =
        serde_json::from_str(&generate_release_sbom(&fixture_input())).expect("valid");
    value["components"].as_array_mut().expect("array")[0]
        .as_object_mut()
        .expect("object")
        .remove("licenses");
    let report = verify_release_sbom(&value.to_string(), &fixture_expected());
    assert!(!report.ok);
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.starts_with("license_fabrication"))
    );
}

#[test]
fn verifier_rejects_stale_lock_digest() {
    let doc = generate_release_sbom(&fixture_input());
    let mut expected = fixture_expected();
    expected.cargo_lock_sha256 = "0".repeat(64);
    let report = verify_release_sbom(&doc, &expected);
    assert!(!report.ok);
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.starts_with("digest_mismatch"))
    );
}

#[test]
fn verifier_rejects_stale_source_sha() {
    let doc = generate_release_sbom(&fixture_input());
    let mut expected = fixture_expected();
    expected.source_sha = "0".repeat(40);
    let report = verify_release_sbom(&doc, &expected);
    assert!(!report.ok);
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.contains("medscale:source_sha"))
    );
}

#[test]
fn verifier_rejects_missing_workspace_package() {
    let doc = generate_release_sbom(&fixture_input());
    let mut expected = fixture_expected();
    expected
        .workspace_package_names
        .push("medscale-ghost".to_owned());
    let report = verify_release_sbom(&doc, &expected);
    assert!(!report.ok);
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.starts_with("missing_workspace_package"))
    );
}

#[test]
fn verifier_rejects_unclassified_native_dep() {
    let mut input = fixture_input();
    input.native_deps = vec![NativeDependency {
        name: "mystery".to_owned(),
        classification: String::new(),
        detail: "unknown".to_owned(),
    }];
    let doc = generate_release_sbom(&input);
    let report = verify_release_sbom(&doc, &fixture_expected());
    assert!(!report.ok);
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.starts_with("native_unclassified"))
    );
}

#[test]
fn verifier_rejects_missing_required_property() {
    let mut value: serde_json::Value =
        serde_json::from_str(&generate_release_sbom(&fixture_input())).expect("valid");
    let props = value["metadata"]["properties"]
        .as_array_mut()
        .expect("props");
    props.retain(|p| p.get("name").and_then(|n| n.as_str()) != Some("medscale:rustc"));
    let report = verify_release_sbom(&value.to_string(), &fixture_expected());
    assert!(!report.ok);
    assert!(report.failures.iter().any(|f| f.contains("medscale:rustc")));
}

#[test]
fn committed_document_is_self_consistent() {
    let path = repo_root().join("evidence/054-release-sbom/SBOM.cdx.json");
    let doc = std::fs::read_to_string(&path).expect("committed SBOM.cdx.json");
    let parsed: serde_json::Value = serde_json::from_str(&doc).expect("parses");
    let props: Vec<(String, String)> = parsed["metadata"]["properties"]
        .as_array()
        .expect("properties")
        .iter()
        .map(|p| {
            (
                p["name"].as_str().unwrap_or_default().to_owned(),
                p["value"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    let lookup = |name: &str| {
        props
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    let names: Vec<String> = parsed["components"]
        .as_array()
        .expect("components")
        .iter()
        .filter(|c| {
            c["properties"].as_array().is_some_and(|p| {
                p.iter()
                    .any(|e| e["name"].as_str() == Some("medscale:package_scope"))
            })
        })
        .filter_map(|c| c["name"].as_str().map(str::to_owned))
        .collect();
    // Every workspace crate directory must be represented.
    let crates_dir = repo_root().join("crates");
    for entry in std::fs::read_dir(&crates_dir).expect("crates dir") {
        let entry = entry.expect("entry");
        if entry.path().join("Cargo.toml").is_file() {
            let name = entry.file_name().to_string_lossy().into_owned();
            assert!(names.contains(&name), "workspace package missing: {name}");
        }
    }
    let expected = ExpectedBinding {
        source_sha: lookup("medscale:source_sha"),
        tree_sha: lookup("medscale:tree_sha"),
        cargo_lock_sha256: lookup("medscale:cargo_lock_sha256"),
        workspace_package_names: names,
    };
    let report = verify_release_sbom(&doc, &expected);
    assert!(
        report.ok,
        "committed SBOM fails self-verify: {:?}",
        report.failures
    );
    // License honesty: every component carries a license block; unknown crates
    // use NOASSERTION rather than a fabricated id.
    for component in parsed["components"].as_array().expect("components") {
        assert!(
            component["licenses"]
                .as_array()
                .is_some_and(|l| !l.is_empty()),
            "component without license block"
        );
    }
}

#[test]
fn doctor_reports_sbom_qualified_without_release_ready() {
    let report = build_doctor_report(None, false, false);
    let rq = &report.release_qualification;
    assert!(rq.release_sbom_qualified);
    assert!(!rq.release_ready);
    assert!(rq.is_honest_prep());
    assert!(
        !rq.missing_evidence_classes
            .contains(&"release_sbom_native_model_assets".to_owned())
    );
    assert!(
        rq.missing_evidence_classes
            .contains(&"release_sbom_signing_provenance".to_owned())
    );
}
