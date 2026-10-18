//! Core surface for the local model catalog and verified snapshot admission
//! (Spec 103). Surfaces (CLI, Desktop) reach `medscale-pack` only through
//! Core. Catalog queries are read-only metadata and need no vault; building a
//! Pack from a snapshot only writes the Pack directory, and admission then goes
//! through the normal `PacksInstallLocal` capability.

use std::collections::BTreeMap;
use std::path::Path;

use medscale_pack::{
    CatalogFilter, CatalogRow, CatalogSource, CatalogStatus, DeviceFit, HfRepoMetadata,
    RuntimeExpectation, TokenClassifierSnapshot, build_token_classifier_pack, runtime_expectation,
};
use serde::Serialize;

pub use medscale_pack::{CatalogError, ModelCatalog, SnapshotError};

/// Query over a catalog snapshot (all fields optional).
#[derive(Debug, Default, Clone)]
pub struct CatalogQuery {
    pub text: Option<String>,
    pub task: Option<String>,
    pub family: Option<String>,
    pub language: Option<String>,
    pub format: Option<String>,
    pub architecture: Option<String>,
    pub license_claim: Option<String>,
    /// OpenMed size tier, for example `Tiny` or `Base`.
    pub tier: Option<String>,
    /// `cpu_portable`, `apple_silicon_only` or `unknown`.
    pub device_fit: Option<String>,
    /// For example `DISCOVERABLE` or `RIGHTS_PENDING`.
    pub status: Option<String>,
    pub max_params: Option<u64>,
    pub max_disk_mb: Option<f64>,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Serialize)]
pub struct CatalogQueryView {
    pub source: CatalogSource,
    pub total_rows: usize,
    /// Every count is a listing count; none implies download or execution.
    pub status_counts: BTreeMap<CatalogStatus, usize>,
    pub total_matching: usize,
    pub next_offset: Option<usize>,
    pub rows: Vec<CatalogRowView>,
}

/// A catalog row plus the pre-download runtime expectation (an expectation
/// from recorded evidence, never a status promotion).
#[derive(Debug, Serialize)]
pub struct CatalogRowView {
    #[serde(flatten)]
    pub row: CatalogRow,
    pub runtime_expectation: RuntimeExpectation,
}

#[derive(Debug, thiserror::Error)]
pub enum ModelCatalogError {
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    #[error(transparent)]
    Snapshot(#[from] SnapshotError),
    #[error("unknown {field}: {value}")]
    InvalidArgument { field: &'static str, value: String },
    #[error("repository {0} is not in the catalog")]
    NotInCatalog(String),
    #[error("io: {0}")]
    Io(String),
}

fn parse_device(value: &str) -> Result<DeviceFit, ModelCatalogError> {
    match value {
        "cpu_portable" => Ok(DeviceFit::CpuPortable),
        "apple_silicon_only" => Ok(DeviceFit::AppleSiliconOnly),
        "unknown" => Ok(DeviceFit::Unknown),
        other => Err(ModelCatalogError::InvalidArgument {
            field: "device_fit",
            value: other.into(),
        }),
    }
}

fn parse_status(value: &str) -> Result<CatalogStatus, ModelCatalogError> {
    serde_json::from_value(serde_json::Value::String(value.to_ascii_uppercase())).map_err(|_| {
        ModelCatalogError::InvalidArgument {
            field: "status",
            value: value.into(),
        }
    })
}

/// Imports a locally supplied catalog snapshot (no network access).
pub fn import_catalog(
    repository: &str,
    commit: &str,
    manifest: &[u8],
) -> Result<ModelCatalog, ModelCatalogError> {
    Ok(ModelCatalog::import(repository, commit, manifest)?)
}

fn load(
    manifest: &Path,
    repository: &str,
    commit: &str,
) -> Result<ModelCatalog, ModelCatalogError> {
    let bytes = std::fs::read(manifest).map_err(|e| ModelCatalogError::Io(e.to_string()))?;
    Ok(ModelCatalog::import(repository, commit, &bytes)?)
}

/// Read-only query of a locally supplied catalog snapshot. No network access.
/// Runnable format variants of `repo_id`'s model, best first for `host`
/// (`apple_silicon`, `other`, or `None` for this machine). Read-only; no
/// network. See `medscale_pack::rank_variants`.
pub fn catalog_variants(
    manifest: &Path,
    repository: &str,
    commit: &str,
    repo_id: &str,
    host: Option<&str>,
) -> Result<Vec<medscale_pack::VariantChoice>, ModelCatalogError> {
    let catalog = load(manifest, repository, commit)?;
    if catalog.get(repo_id).is_none() {
        return Err(ModelCatalogError::NotInCatalog(repo_id.to_owned()));
    }
    let host = match host {
        None => medscale_pack::HostPlatform::current(),
        Some("apple_silicon") => medscale_pack::HostPlatform::AppleSilicon,
        Some("other") => medscale_pack::HostPlatform::Other,
        Some(other) => {
            return Err(ModelCatalogError::InvalidArgument {
                field: "host",
                value: other.to_owned(),
            });
        }
    };
    Ok(medscale_pack::rank_variants(&catalog, repo_id, host))
}

pub fn query_catalog_snapshot(
    manifest: &Path,
    repository: &str,
    commit: &str,
    query: &CatalogQuery,
) -> Result<CatalogQueryView, ModelCatalogError> {
    let catalog = load(manifest, repository, commit)?;
    let filter = CatalogFilter {
        text: query.text.clone(),
        task: query.task.clone(),
        family: query.family.clone(),
        language: query.language.clone(),
        format: query.format.clone(),
        architecture: query.architecture.clone(),
        license_claim: query.license_claim.clone(),
        tier: query.tier.clone(),
        device_fit: query.device_fit.as_deref().map(parse_device).transpose()?,
        status: query.status.as_deref().map(parse_status).transpose()?,
        max_params: query.max_params,
        max_disk_mb: query.max_disk_mb,
    };
    let page = catalog.query(&filter, query.offset, query.limit)?;
    Ok(CatalogQueryView {
        total_matching: page.total_matching,
        next_offset: page.next_offset,
        rows: page
            .rows
            .into_iter()
            .map(|row| CatalogRowView {
                runtime_expectation: runtime_expectation(row),
                row: row.clone(),
            })
            .collect(),
        status_counts: catalog.status_counts(),
        total_rows: catalog.len(),
        source: catalog.source.clone(),
    })
}

/// The `ModelCatalogRead` capability body: one catalog page as a contract
/// type. Read-only; the snapshot is a local file.
pub fn catalog_page(
    request: &medscale_contracts::model_catalog::ModelCatalogQueryRequest,
) -> Result<medscale_contracts::model_catalog::ModelCatalogPage, ModelCatalogError> {
    let query = CatalogQuery {
        text: request.text.clone(),
        task: request.task.clone(),
        family: request.family.clone(),
        language: request.language.clone(),
        format: request.format.clone(),
        architecture: request.architecture.clone(),
        license_claim: request.license_claim.clone(),
        tier: request.tier.clone(),
        device_fit: request.device_fit.clone(),
        status: request.status.clone(),
        max_params: request.max_params,
        max_disk_mb: request.max_disk_mb,
        offset: request.offset as usize,
        limit: request.limit as usize,
    };
    let view = query_catalog_snapshot(
        Path::new(&request.snapshot_path),
        &request.repository,
        &request.commit,
        &query,
    )?;
    let to_json = |value: &CatalogRowView| {
        serde_json::to_value(value).map_err(|e| ModelCatalogError::Io(e.to_string()))
    };
    Ok(medscale_contracts::model_catalog::ModelCatalogPage {
        repository: view.source.repository.clone(),
        commit: view.source.commit.clone(),
        manifest_sha256: view.source.manifest_sha256.clone(),
        total_rows: view.total_rows as u64,
        status_counts: view
            .status_counts
            .iter()
            .map(|(status, count)| {
                let key = serde_json::to_value(status)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_else(|| format!("{status:?}"));
                (key, *count as u64)
            })
            .collect(),
        total_matching: view.total_matching as u64,
        next_offset: view.next_offset.map(|o| o as u64),
        rows: view.rows.iter().map(to_json).collect::<Result<_, _>>()?,
    })
}

/// Verifies a locally supplied Hub snapshot against its catalog row and writes
/// a signed token-classification Pack to `out_dir`. Returns the Pack id.
/// Admission is a separate `PacksInstallLocal` call.
pub fn build_pack_from_snapshot(
    manifest: &Path,
    repository: &str,
    commit: &str,
    snapshot_dir: &Path,
    onnx_file: &str,
    fixed_sequence_length: u32,
    out_dir: &Path,
) -> Result<String, ModelCatalogError> {
    let catalog = load(manifest, repository, commit)?;
    let meta_bytes = std::fs::read(snapshot_dir.join("metadata.json"))
        .map_err(|e| ModelCatalogError::Io(e.to_string()))?;
    let meta: HfRepoMetadata = serde_json::from_slice(&meta_bytes)
        .map_err(|e| ModelCatalogError::Io(format!("metadata.json: {e}")))?;
    let row = catalog
        .get(&meta.id)
        .ok_or_else(|| ModelCatalogError::NotInCatalog(meta.id.clone()))?;
    // An OpenMed MLX snapshot borrows its tokenizer from its catalog source.
    let source_meta = medscale_pack::snapshot_tokenizer_source(snapshot_dir)?;
    let source_row = match &source_meta {
        Some(m) => Some(
            catalog
                .get(&m.id)
                .ok_or_else(|| ModelCatalogError::NotInCatalog(m.id.clone()))?,
        ),
        None => None,
    };
    let tokenizer_source = source_meta
        .as_ref()
        .zip(source_row)
        .map(|(meta, row)| medscale_pack::TokenizerSource { meta, row });
    Ok(build_token_classifier_pack(
        &TokenClassifierSnapshot {
            meta: &meta,
            row,
            snapshot_dir,
            onnx_file,
            fixed_sequence_length,
            tokenizer_source,
        },
        out_dir,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMIT: &str = "ea920f36fadd7b45935247d639f0ffa1ef493b23";

    fn manifest(tag: &str) -> std::path::PathBuf {
        let rows = concat!(
            r#"{"repo_id":"Org/ner-en","family":"NER","task":"token-classification","languages":["en"],"formats":["onnx"],"param_count":33000000,"architecture":"bert","license":"apache-2.0"}"#,
            "\n",
            r#"{"repo_id":"Org/pii-ar","family":"PII","task":"token-classification","languages":["ar"],"formats":["mlx-fp"],"param_count":560000000,"architecture":"xlm-roberta","license":"apache-2.0"}"#,
            "\n",
            r#"{"repo_id":"Org/gen","family":"General","task":"text-generation","languages":["en"],"formats":["pytorch"],"architecture":"qwen","license":"other"}"#,
            "\n",
        );
        let path = std::env::temp_dir().join(format!(
            "medscale-core-catalog-{tag}-{}.jsonl",
            std::process::id()
        ));
        std::fs::write(&path, rows).unwrap();
        path
    }

    #[test]
    fn variants_are_read_only_and_validate_their_arguments() {
        let path = manifest("variants");
        let ranked =
            catalog_variants(&path, "org/fixture", COMMIT, "Org/ner-en", Some("other")).unwrap();
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].runtime, medscale_pack::VariantRuntime::TractOnnx);
        // Text generation has no runnable token-classification variant.
        assert!(
            catalog_variants(&path, "org/fixture", COMMIT, "Org/gen", None)
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            catalog_variants(&path, "org/fixture", COMMIT, "Org/missing", None),
            Err(ModelCatalogError::NotInCatalog(_))
        ));
        assert!(matches!(
            catalog_variants(&path, "org/fixture", COMMIT, "Org/ner-en", Some("gpu")),
            Err(ModelCatalogError::InvalidArgument { field: "host", .. })
        ));
    }

    #[test]
    fn query_filters_and_reports_listing_counts_only() {
        let path = manifest("query");
        let all = query_catalog_snapshot(
            &path,
            "org/fixture",
            COMMIT,
            &CatalogQuery {
                limit: 10,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(all.total_rows, 3);
        assert_eq!(
            all.status_counts.get(&CatalogStatus::Discoverable),
            Some(&2)
        );
        assert_eq!(
            all.status_counts.get(&CatalogStatus::RightsPending),
            Some(&1)
        );
        let cpu = query_catalog_snapshot(
            &path,
            "org/fixture",
            COMMIT,
            &CatalogQuery {
                device_fit: Some("cpu_portable".into()),
                status: Some("discoverable".into()),
                limit: 10,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            cpu.rows
                .iter()
                .map(|r| r.row.repo_id.as_str())
                .collect::<Vec<_>>(),
            ["Org/ner-en"]
        );
        assert!(
            query_catalog_snapshot(
                &path,
                "org/fixture",
                COMMIT,
                &CatalogQuery {
                    device_fit: Some("gpu".into()),
                    limit: 10,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            query_catalog_snapshot(
                &path,
                "org/fixture",
                COMMIT,
                &CatalogQuery {
                    status: Some("installed".into()),
                    limit: 10,
                    ..Default::default()
                }
            )
            .is_err()
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn snapshot_for_unknown_repository_is_refused_before_any_write() {
        let path = manifest("snapshot");
        let dir = std::env::temp_dir().join(format!("medscale-core-snap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("metadata.json"),
            format!(
                r#"{{"id":"Other/model","sha":"{}","siblings":[]}}"#,
                "a".repeat(40)
            ),
        )
        .unwrap();
        let out = dir.join("pack");
        let err =
            build_pack_from_snapshot(&path, "org/fixture", COMMIT, &dir, "model.onnx", 128, &out)
                .unwrap_err();
        assert!(matches!(err, ModelCatalogError::NotInCatalog(_)));
        assert!(!out.exists());
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_file(path);
    }
}
