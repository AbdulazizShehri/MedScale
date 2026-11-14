//! Spec 103: the local model catalog is reachable through Core dispatch under
//! `Capability::ModelCatalogRead` (read-only, no vault, no network). Synthetic
//! catalog rows only.

use std::path::{Path, PathBuf};

use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::model_catalog::ModelCatalogQueryRequest;
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::CoreFacade;

const COMMIT: &str = "ea920f36fadd7b45935247d639f0ffa1ef493b23";

fn snapshot(tag: &str) -> PathBuf {
    let rows = [
        r#"{"repo_id":"OpenMed/A-bert","family":"NER","task":"token-classification","architecture":"bert","formats":["onnx"],"license":"apache-2.0"}"#,
        r#"{"repo_id":"OpenMed/OpenMed-NER-AnatomyDetect-BigMed-278M-v1-onnx-android","family":"NER","task":"token-classification","architecture":"xlm-roberta","formats":["onnx"],"license":"apache-2.0"}"#,
        r#"{"repo_id":"OpenMed/C-mlx","family":"NER","task":"token-classification","architecture":"bert","formats":["mlx-fp"],"license":"apache-2.0"}"#,
    ];
    let path = std::env::temp_dir().join(format!(
        "medscale-103-cap-{tag}-{}.jsonl",
        std::process::id()
    ));
    std::fs::write(&path, rows.join("\n")).unwrap();
    path
}

fn query(path: &Path) -> ModelCatalogQueryRequest {
    ModelCatalogQueryRequest {
        snapshot_path: path.display().to_string(),
        repository: "maziyarpanahi/openmed".into(),
        commit: COMMIT.into(),
        limit: 10,
        ..Default::default()
    }
}

fn dispatch(
    facade: &CoreFacade,
    capability: Capability,
    q: ModelCatalogQueryRequest,
) -> Result<ResponseBody, AuthorityError> {
    facade
        .dispatch(AuthorityRequest::new(
            OpaqueId::new("req-catalog"),
            VaultId::new("vault-unopened"),
            RealmId::new("realm-a"),
            AuthorityScopeId::new("scope-a"),
            capability,
            RequestBody::ModelCatalogQuery { query: Box::new(q) },
        ))
        .result
}

#[test]
fn catalog_query_is_dispatched_read_only_without_a_vault() {
    let path = snapshot("ok");
    let facade = CoreFacade::new();
    let body = dispatch(&facade, Capability::ModelCatalogRead, query(&path)).unwrap();
    let ResponseBody::ModelCatalogPage { page } = body else {
        panic!("{body:?}");
    };
    assert_eq!(page.commit, COMMIT);
    assert_eq!(page.total_rows, 3);
    assert_eq!(page.total_matching, 3);
    assert_eq!(page.next_offset, None);
    // Listing counts only: every row is metadata, nothing executed.
    assert_eq!(page.status_counts.values().sum::<u64>(), 3);
    assert!(!page.status_counts.contains_key("EXECUTED_TESTED"));
    let kinds: Vec<(&str, &str)> = page
        .rows
        .iter()
        .map(|r| {
            (
                r["repo_id"].as_str().unwrap(),
                r["runtime_expectation"]["kind"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        kinds,
        [
            ("OpenMed/A-bert", "expected_runnable"),
            ("OpenMed/C-mlx", "expected_runnable_safetensors"),
            (
                "OpenMed/OpenMed-NER-AnatomyDetect-BigMed-278M-v1-onnx-android",
                "known_unsupported"
            ),
        ]
    );

    let mut filtered = query(&path);
    filtered.architecture = Some("xlm-roberta".into());
    let ResponseBody::ModelCatalogPage { page } =
        dispatch(&facade, Capability::ModelCatalogRead, filtered).unwrap()
    else {
        panic!("page");
    };
    assert_eq!(page.total_matching, 1);
    let _ = std::fs::remove_file(path);
}

#[test]
fn catalog_query_needs_the_matching_capability_and_valid_input() {
    let path = snapshot("deny");
    let facade = CoreFacade::new();
    assert!(matches!(
        dispatch(&facade, Capability::PacksList, query(&path)),
        Err(AuthorityError::Unauthorized)
    ));
    let mut bad = query(&path);
    bad.status = Some("NOT_A_STATUS".into());
    assert!(matches!(
        dispatch(&facade, Capability::ModelCatalogRead, bad),
        Err(AuthorityError::InvalidArgument { .. })
    ));
    let mut zero = query(&path);
    zero.limit = 0;
    assert!(matches!(
        dispatch(&facade, Capability::ModelCatalogRead, zero),
        Err(AuthorityError::InvalidArgument { .. })
    ));
    let mut wrong_commit = query(&path);
    wrong_commit.commit = "not-a-commit".into();
    assert!(dispatch(&facade, Capability::ModelCatalogRead, wrong_commit).is_err());
    let _ = std::fs::remove_file(path);
}
