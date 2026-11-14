//! Deterministic CycloneDX release SBOM generation + verification (Spec 054).
//!
//! READY_BASE scope: binds `source_sha` / `tree_sha` / `cargo_lock_sha256`,
//! workspace + Rust + native inventories, artifacts, and Pack/model assets in
//! CycloneDX 1.5 JSON. Never claims `RELEASE_READY`, reproducible builds, or a
//! public SPDX license decision. Unknown licenses are recorded as
//! `NOASSERTION`, never fabricated.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// SBOM document format identifier (distinct from the public project license).
pub const SBOM_FORMAT: &str = "CycloneDX";
/// CycloneDX spec version emitted by the generator.
pub const SBOM_SPEC_VERSION: &str = "1.5";
/// Value of `medscale:sbom_kind` for qualified release documents.
pub const SBOM_KIND: &str = "release_qualified_cyclonedx15";
/// Fixed timestamp keeps the document deterministic; display-only overrides
/// are never part of the hashed document.
pub const ZERO_TIME: &str = "1970-01-01T00:00:00Z";
/// Recorded until bit-for-bit reproducibility is proven (it is not).
pub const REPRODUCIBLE_BUILD_UNPROVEN: &str = "unproven";

/// Build-environment binding recorded in the SBOM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildEnvironment {
    pub os: String,
    pub os_version: String,
    pub target_triple: String,
    pub rustc: String,
    pub cargo: String,
}

/// Workspace crate inventory entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspacePackage {
    pub name: String,
    pub version: String,
    pub license: Option<String>,
}

/// Rust (crates.io / workspace) dependency entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustDependency {
    pub name: String,
    pub version: String,
    pub license: Option<String>,
}

/// Native/system dependency entry. Anything that cannot be enumerated on the
/// host must be explicitly classified (`host-tool`, `os-provided`,
/// `not-applicable`), never silently omitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeDependency {
    pub name: String,
    pub classification: String,
    pub detail: String,
}

/// Release artifact inventory entry (name + expected SHA-256).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseArtifact {
    pub name: String,
    pub sha256: String,
}

/// Pack / model asset entry with honest rights metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackAsset {
    pub name: String,
    pub kind: String,
    pub rights_note: String,
}

/// Full generator input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSbomInput {
    pub source_sha: String,
    pub tree_sha: String,
    pub cargo_lock_sha256: String,
    pub build_env: BuildEnvironment,
    pub workspace_packages: Vec<WorkspacePackage>,
    pub rust_deps: Vec<RustDependency>,
    pub native_deps: Vec<NativeDependency>,
    pub artifacts: Vec<ReleaseArtifact>,
    pub pack_assets: Vec<PackAsset>,
}

/// Expected live binding the verifier checks a document against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedBinding {
    pub source_sha: String,
    pub tree_sha: String,
    pub cargo_lock_sha256: String,
    pub workspace_package_names: Vec<String>,
}

/// Verification outcome. `ok` is true only when `failures` is empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    pub ok: bool,
    pub failures: Vec<String>,
    pub component_count: usize,
    pub document_sha256: String,
}

/// SHA-256 hex digest helper (also used for lockfiles and artifacts).
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_of(&hasher.finalize())
}

fn hex_of(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Honest license JSON: known SPDX id passes through, unknown becomes
/// `NOASSERTION`. Never invents a license.
fn license_json(license: &Option<String>) -> serde_json::Value {
    match license.as_deref().map(str::trim) {
        // Workspace crates are UNLICENSED (private): not an SPDX id and not a
        // public license grant. Record honestly without inventing an id.
        Some("UNLICENSED") => serde_json::json!({
            "license": { "name": "UNLICENSED-workspace-no-public-license" }
        }),
        Some(id) if !id.is_empty() && id != "NOASSERTION" => {
            serde_json::json!({ "license": { "id": id } })
        }
        _ => serde_json::json!({ "license": { "name": "NOASSERTION" } }),
    }
}

/// Generates the deterministic CycloneDX 1.5 release SBOM document.
/// Components and properties are sorted; the timestamp is fixed.
#[must_use]
pub fn generate_release_sbom(input: &ReleaseSbomInput) -> String {
    let mut components: Vec<serde_json::Value> = Vec::new();

    let mut workspace = input.workspace_packages.clone();
    workspace.sort_by(|a, b| a.name.cmp(&b.name));
    for pkg in &workspace {
        components.push(serde_json::json!({
            "type": "application",
            "bom-ref": format!("pkg:cargo/{}@{}", pkg.name, pkg.version),
            "name": pkg.name,
            "version": pkg.version,
            "purl": format!("pkg:cargo/{}@{}", pkg.name, pkg.version),
            "licenses": [license_json(&pkg.license)],
            "scope": "required",
            "properties": [{ "name": "medscale:package_scope", "value": "workspace" }],
        }));
    }

    let mut rust_deps = input.rust_deps.clone();
    rust_deps.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    for dep in &rust_deps {
        components.push(serde_json::json!({
            "type": "library",
            "bom-ref": format!("pkg:cargo/{}@{}", dep.name, dep.version),
            "name": dep.name,
            "version": dep.version,
            "purl": format!("pkg:cargo/{}@{}", dep.name, dep.version),
            "licenses": [license_json(&dep.license)],
            "scope": "required",
        }));
    }

    let mut native = input.native_deps.clone();
    native.sort_by(|a, b| a.name.cmp(&b.name));
    for dep in &native {
        components.push(serde_json::json!({
            "type": "library",
            "bom-ref": format!("pkg:medscale/native/{}", dep.name),
            "name": dep.name,
            "licenses": [serde_json::json!({ "license": { "name": "NOASSERTION" } })],
            "scope": "required",
            "properties": [
                { "name": "medscale:native_classification", "value": dep.classification },
                { "name": "medscale:native_detail", "value": dep.detail },
            ],
        }));
    }

    let mut artifacts = input.artifacts.clone();
    artifacts.sort_by(|a, b| a.name.cmp(&b.name));
    for artifact in &artifacts {
        components.push(serde_json::json!({
            "type": "file",
            "bom-ref": format!("pkg:medscale/artifact/{}", artifact.name),
            "name": artifact.name,
            "licenses": [serde_json::json!({ "license": { "name": "NOASSERTION" } })],
            "hashes": [{ "alg": "SHA-256", "content": artifact.sha256 }],
        }));
    }

    let mut assets = input.pack_assets.clone();
    assets.sort_by(|a, b| a.name.cmp(&b.name));
    for asset in &assets {
        components.push(serde_json::json!({
            "type": "data",
            "bom-ref": format!("pkg:medscale/pack-asset/{}", asset.name),
            "name": asset.name,
            "licenses": [serde_json::json!({ "license": { "name": "NOASSERTION" } })],
            "properties": [
                { "name": "medscale:pack_asset_kind", "value": asset.kind },
                { "name": "medscale:rights_note", "value": asset.rights_note },
            ],
        }));
    }

    components.sort_by(|a, b| {
        a.get("bom-ref")
            .and_then(serde_json::Value::as_str)
            .cmp(&b.get("bom-ref").and_then(serde_json::Value::as_str))
    });

    let env = &input.build_env;
    let document = serde_json::json!({
        "bomFormat": SBOM_FORMAT,
        "specVersion": SBOM_SPEC_VERSION,
        "version": 1,
        "serialNumber": format!("urn:uuid:medscale-054-{}", &input.tree_sha[..input.tree_sha.len().min(12)]),
        "metadata": {
            "timestamp": ZERO_TIME,
            "tools": [{ "vendor": "MedScale", "name": "medscale-core::release_sbom", "version": "054" }],
            "component": { "type": "application", "name": "MedScale", "version": "trusted-v1" },
            "properties": [
                { "name": "medscale:sbom_kind", "value": SBOM_KIND },
                { "name": "medscale:sbom_document_format", "value": "CycloneDX-1.5 (SBOM format; not the public project license)" },
                { "name": "medscale:public_project_license", "value": "Apache-2.0" },
                { "name": "medscale:source_sha", "value": input.source_sha },
                { "name": "medscale:tree_sha", "value": input.tree_sha },
                { "name": "medscale:cargo_lock_sha256", "value": input.cargo_lock_sha256 },
                { "name": "medscale:build_os", "value": env.os },
                { "name": "medscale:build_os_version", "value": env.os_version },
                { "name": "medscale:target_triple", "value": env.target_triple },
                { "name": "medscale:rustc", "value": env.rustc },
                { "name": "medscale:cargo", "value": env.cargo },
                { "name": "medscale:reproducible_build", "value": REPRODUCIBLE_BUILD_UNPROVEN },
                { "name": "medscale:release_ready", "value": "false" },
            ],
        },
        "components": components,
    });
    serde_json::to_string_pretty(&document).unwrap_or_else(|_| "{}".to_owned())
}

fn metadata_properties(document: &serde_json::Value) -> Vec<(String, String)> {
    document
        .pointer("/metadata/properties")
        .and_then(serde_json::Value::as_array)
        .map(|props| {
            props
                .iter()
                .filter_map(|p| {
                    let name = p.get("name")?.as_str()?.to_owned();
                    let value = p.get("value")?.as_str()?.to_owned();
                    Some((name, value))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Verifies a release SBOM document against the expected live binding.
/// Fails closed: any parse error, missing property, digest mismatch, missing
/// workspace package, unclassified native dep, missing artifact hash, or
/// missing license block is a failure.
#[must_use]
pub fn verify_release_sbom(document: &str, expected: &ExpectedBinding) -> VerificationReport {
    let document_sha256 = sha256_hex(document.as_bytes());
    let mut failures: Vec<String> = Vec::new();

    let parsed: serde_json::Value = match serde_json::from_str(document) {
        Ok(value) => value,
        Err(err) => {
            failures.push(format!("parse_error: {err}"));
            return VerificationReport {
                ok: false,
                failures,
                component_count: 0,
                document_sha256,
            };
        }
    };

    if parsed.get("bomFormat").and_then(serde_json::Value::as_str) != Some(SBOM_FORMAT) {
        failures.push("missing_property: bomFormat must be CycloneDX".to_owned());
    }
    if parsed
        .get("specVersion")
        .and_then(serde_json::Value::as_str)
        != Some(SBOM_SPEC_VERSION)
    {
        failures.push("missing_property: specVersion must be 1.5".to_owned());
    }

    let props = metadata_properties(&parsed);
    let lookup = |name: &str| {
        props
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
    };
    let mut require_prop = |name: &str, want: &str, code: &str| match lookup(name) {
        Some(got) if got == want => {}
        Some(got) => failures.push(format!("{code}: {name} mismatch (want {want}, got {got})")),
        None => failures.push(format!("missing_property: {name}")),
    };
    require_prop("medscale:sbom_kind", SBOM_KIND, "digest_mismatch");
    require_prop(
        "medscale:source_sha",
        &expected.source_sha,
        "digest_mismatch",
    );
    require_prop("medscale:tree_sha", &expected.tree_sha, "digest_mismatch");
    require_prop(
        "medscale:cargo_lock_sha256",
        &expected.cargo_lock_sha256,
        "digest_mismatch",
    );
    for name in [
        "medscale:target_triple",
        "medscale:rustc",
        "medscale:reproducible_build",
    ] {
        if lookup(name).is_none() {
            failures.push(format!("missing_property: {name}"));
        }
    }

    let components = parsed
        .get("components")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();

    for name in &expected.workspace_package_names {
        let found = components.iter().any(|c| {
            c.get("name").and_then(serde_json::Value::as_str) == Some(name)
                && c.get("properties")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|p| {
                        p.iter().any(|e| {
                            e.get("name").and_then(serde_json::Value::as_str)
                                == Some("medscale:package_scope")
                        })
                    })
        });
        if !found {
            failures.push(format!("missing_workspace_package: {name}"));
        }
    }

    for component in &components {
        let label = component
            .get("bom-ref")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("?");
        let licenses = component
            .get("licenses")
            .and_then(serde_json::Value::as_array);
        if licenses.is_none_or(Vec::is_empty) {
            failures.push(format!("license_fabrication: {label} has no license block"));
        }
        if let Some(props) = component
            .get("properties")
            .and_then(serde_json::Value::as_array)
        {
            for prop in props {
                if prop.get("name").and_then(serde_json::Value::as_str)
                    == Some("medscale:native_classification")
                    && prop
                        .get("value")
                        .and_then(serde_json::Value::as_str)
                        .is_none_or(str::is_empty)
                {
                    failures.push(format!("native_unclassified: {label}"));
                }
            }
        }
        if component.get("type").and_then(serde_json::Value::as_str) == Some("file")
            && component.get("hashes").is_none()
        {
            failures.push(format!("missing_artifact: {label} has no hashes"));
        }
    }

    let component_count = components.len();
    let ok = failures.is_empty();
    VerificationReport {
        ok,
        failures,
        component_count,
        document_sha256,
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

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
                license: Some("MIT OR Apache-2.0".to_owned()),
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
                rights_note: "synthetic fixture; no third-party rights".to_owned(),
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
        assert_eq!(generate_release_sbom(&input), generate_release_sbom(&input));
    }

    #[test]
    fn fixture_verifies_clean() {
        let doc = generate_release_sbom(&fixture_input());
        let report = verify_release_sbom(&doc, &fixture_expected());
        assert!(report.ok, "unexpected failures: {:?}", report.failures);
    }
}
