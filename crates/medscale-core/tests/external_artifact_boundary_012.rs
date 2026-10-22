//! Architecture: MESC remains ARTIFACT_FIRST — no Python package dependency.

use std::fs;
use std::path::PathBuf;

#[test]
fn workspace_crates_do_not_depend_on_mesc_python_runtime() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let forbidden = ["mesc", "pymesc", "thehalfmoon-mesc"];
    for entry in fs::read_dir(root.join("crates")).unwrap() {
        let entry = entry.unwrap();
        let cargo = entry.path().join("Cargo.toml");
        if !cargo.is_file() {
            continue;
        }
        let text = fs::read_to_string(&cargo).unwrap().to_lowercase();
        for needle in forbidden {
            // Allow comments mentioning MESC boundary; forbid dependency names.
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') {
                    continue;
                }
                if trimmed.contains(needle)
                    && (trimmed.contains("dependencies") || trimmed.contains('='))
                {
                    // Only fail if it looks like a crate dependency key.
                    if trimmed.starts_with(needle) || trimmed.contains(&format!("{needle} =")) {
                        panic!("{} must not depend on {needle}", cargo.display());
                    }
                }
            }
        }
    }
}
