//! Local model catalog contracts (Spec 103).
//!
//! A read-only query over a locally supplied catalog snapshot, dispatched
//! through Core under `Capability::ModelCatalogRead`. Catalog rows are
//! metadata only: no count or status in a page implies that a model was
//! downloaded, verified or executed. Rows are carried as JSON objects because
//! the row schema is owned by `medscale-pack`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One catalog query. Every filter is optional.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelCatalogQueryRequest {
    /// Local path of the catalog snapshot (JSONL). Never fetched over the network.
    pub snapshot_path: String,
    /// Upstream repository the snapshot was taken from.
    pub repository: String,
    /// Exact 40-hex commit of the snapshot.
    pub commit: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub architecture: Option<String>,
    #[serde(default)]
    pub license_claim: Option<String>,
    /// OpenMed size tier, for example `Tiny` or `Base`.
    #[serde(default)]
    pub tier: Option<String>,
    /// `cpu_portable`, `apple_silicon_only` or `unknown`.
    #[serde(default)]
    pub device_fit: Option<String>,
    /// A catalog status such as `DISCOVERABLE` or `RIGHTS_PENDING`.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub max_params: Option<u64>,
    #[serde(default)]
    pub max_disk_mb: Option<f64>,
    #[serde(default)]
    pub offset: u32,
    /// Page size; must be at least 1 and at most the catalog page bound.
    pub limit: u32,
}

/// One page of catalog rows plus whole-catalog listing counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCatalogPage {
    pub repository: String,
    pub commit: String,
    pub manifest_sha256: String,
    pub total_rows: u64,
    /// Listing counts per catalog status; none implies download or execution.
    pub status_counts: BTreeMap<String, u64>,
    pub total_matching: u64,
    pub next_offset: Option<u64>,
    /// Catalog rows, each with its pre-download `runtime_expectation`.
    pub rows: Vec<serde_json::Value>,
}
