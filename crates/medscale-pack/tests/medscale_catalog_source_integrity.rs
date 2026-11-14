//! MedScale native catalog bundle provenance and integrity contract.
//! This intentionally checks the real source identity without branding the
//! product or changing the actual external model IDs.

use medscale_pack::ModelCatalog;

#[test]
fn medscale_bundle_sha256_matches_native_catalog_import() {
    let descriptor: serde_json::Value = serde_json::from_str(include_str!(
        "../../../third_party/medscale-model-catalog-v3/source.json"
    ))
    .expect("valid pinned MedScale descriptor");
    let repository = descriptor["repository"]
        .as_str()
        .expect("catalog source repository is explicit");
    let commit = descriptor["commit"]
        .as_str()
        .expect("catalog source revision is pinned");
    let pinned_digest = descriptor["manifest_sha256"]
        .as_str()
        .expect("catalog digest is pinned");

    let original_bytes =
        include_bytes!("../../../third_party/medscale-model-catalog-v3/models.jsonl");
    let catalog = ModelCatalog::import(repository, commit, original_bytes)
        .expect("bundled MedScale catalog must import");
    assert_eq!(catalog.source.manifest_sha256, pinned_digest);
    assert_eq!(catalog.source.repository, repository);
    assert_eq!(catalog.source.commit, commit);
}
