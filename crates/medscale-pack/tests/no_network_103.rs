//! Spec 103 (T103-04): the catalog, snapshot verification and both model
//! runtimes live in `medscale-pack`, which must have no network capability.
//! Acquisition is the only egress and it lives in Core behind the Governed
//! Browse transport. This walks the locked dependency graph from
//! `medscale-pack` (transitively, every target) and fails if any networking
//! or HTTP crate is reachable.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Crates whose presence would give `medscale-pack` a network stack.
const FORBIDDEN: &[&str] = &[
    "ureq",
    "reqwest",
    "hyper",
    "h2",
    "http",
    "isahc",
    "curl",
    "surf",
    "attohttpc",
    "tokio",
    "mio",
    "socket2",
    "rustls",
    "native-tls",
    "openssl",
    "hf-hub",
    "medscale-network",
];

/// Parses `Cargo.lock` into package name -> dependency names (all versions merged).
fn lock_graph() -> BTreeMap<String, BTreeSet<String>> {
    let lock =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock"))
            .expect("Cargo.lock");
    let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for package in lock.split("[[package]]").skip(1) {
        let mut name = None;
        let mut deps = BTreeSet::new();
        let mut in_deps = false;
        for line in package.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("name = ") {
                name = Some(rest.trim_matches('"').to_owned());
            } else if line.starts_with("dependencies = [") {
                in_deps = true;
            } else if in_deps && line.starts_with(']') {
                in_deps = false;
            } else if in_deps {
                // `"name"` or `"name version"` or `"name version (source)"`.
                let entry = line.trim_matches(|c| c == '"' || c == ',');
                if let Some(dep) = entry.split_whitespace().next() {
                    deps.insert(dep.to_owned());
                }
            }
        }
        if let Some(name) = name {
            graph.entry(name).or_default().extend(deps);
        }
    }
    graph
}

#[test]
fn medscale_pack_has_no_network_crate_in_its_dependency_graph() {
    let graph = lock_graph();
    assert!(
        graph.contains_key("medscale-pack"),
        "lock lists medscale-pack"
    );
    let mut seen = BTreeSet::new();
    let mut stack = vec!["medscale-pack".to_owned()];
    while let Some(name) = stack.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        if let Some(deps) = graph.get(&name) {
            stack.extend(deps.iter().cloned());
        }
    }
    let reachable: Vec<&str> = FORBIDDEN
        .iter()
        .copied()
        .filter(|f| seen.contains(*f))
        .collect();
    assert!(
        reachable.is_empty(),
        "network-capable crates reachable from medscale-pack: {reachable:?}"
    );
    // The walk is meaningful: the runtimes' real dependencies are reached.
    for expected in ["tract-onnx", "candle-core", "tokenizers", "safetensors"] {
        assert!(seen.contains(expected), "{expected} reachable");
    }
}
