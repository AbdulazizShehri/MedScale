//! Read-only local model catalog (Spec 103 phase 1).
//!
//! Imports an exact upstream manifest snapshot (JSON Lines, for example
//! OpenMed `models.jsonl`) that is supplied locally. Importing never contacts a
//! network. A catalog row is metadata only: listing a model says nothing about
//! whether it is downloaded, admitted, runnable, qualified or clinically
//! validated, which is why every row carries an explicit [`CatalogStatus`].

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Upper bounds so a hostile or corrupt manifest cannot exhaust memory.
pub const MAX_CATALOG_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_CATALOG_ROWS: usize = 100_000;
pub const MAX_LINE_BYTES: usize = 64 * 1024;
pub const MAX_PAGE_SIZE: usize = 500;

/// Evidence ladder. A row moves up only on evidence; states are never merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CatalogStatus {
    Discoverable,
    RightsPending,
    DownloadableOptIn,
    Cached,
    Verified,
    Admitted,
    RuntimeCompatible,
    ExecutedTested,
    TaskQualified,
    ClinicallyValidated,
}

/// Where a model's listed formats can run without accelerator-specific runtimes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceFit {
    /// At least one portable format (ONNX or PyTorch weights) is listed.
    CpuPortable,
    /// Only Apple MLX formats are listed.
    AppleSiliconOnly,
    /// No recognised format.
    Unknown,
}

/// A size that is either stated by the source, derived, or not known.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "megabytes", rename_all = "snake_case")]
pub enum SizeMb {
    Stated(f64),
    Estimated(f64),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CatalogRow {
    /// Stable first-party MedScale catalog lookup ID, never a model-ownership claim.
    pub medscale_id: String,
    /// Actual source repository identity used for verification and download.
    pub repo_id: String,
    pub family: String,
    pub task: String,
    pub languages: Vec<String>,
    pub tier: Option<String>,
    pub param_count: Option<u64>,
    pub architecture: String,
    pub base_model: Option<String>,
    pub formats: Vec<String>,
    /// License as claimed by the manifest row; not a reviewed rights decision.
    pub license_claim: Option<String>,
    pub released: Option<String>,
    pub reproducibility_hash: Option<String>,
    pub disk: SizeMb,
    pub device_fit: DeviceFit,
    pub status: CatalogStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogSource {
    /// Upstream repository, for example `maziyarpanahi/openmed`.
    pub repository: String,
    /// Exact 40-hex commit of the snapshot.
    pub commit: String,
    /// SHA-256 of the imported manifest bytes.
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CatalogError {
    #[error("manifest exceeds {MAX_CATALOG_BYTES} bytes")]
    TooLarge,
    #[error("manifest exceeds {MAX_CATALOG_ROWS} rows")]
    TooManyRows,
    #[error("line {line}: longer than {MAX_LINE_BYTES} bytes")]
    LineTooLong { line: usize },
    #[error("line {line}: invalid row: {reason}")]
    InvalidRow { line: usize, reason: String },
    #[error("line {line}: duplicate repo_id {repo_id}")]
    Duplicate { line: usize, repo_id: String },
    #[error("commit must be a 40-character hex revision")]
    InvalidCommit,
    #[error("manifest is not UTF-8")]
    NotUtf8,
    #[error("page size must be between 1 and {MAX_PAGE_SIZE}")]
    InvalidPageSize,
}

#[derive(Deserialize)]
struct RawRow {
    repo_id: String,
    family: String,
    task: String,
    languages: Option<Vec<String>>,
    tier: Option<String>,
    param_count: Option<u64>,
    architecture: Option<String>,
    base_model: Option<String>,
    formats: Option<Vec<String>>,
    license: Option<String>,
    released: Option<String>,
    reproducibility_hash: Option<String>,
    disk_mb: Option<f64>,
}

/// Licenses whose claim alone is enough to list a row as discoverable. Every
/// row still needs a card-level rights review before admission (Spec 103 R05).
const PERMISSIVE_CLAIMS: &[&str] = &["apache-2.0", "mit"];

fn bytes_per_param(format: &str) -> Option<f64> {
    match format {
        "pytorch" | "onnx" | "safetensors" => Some(4.0),
        "mlx-fp" => Some(2.0),
        "mlx-8bit" => Some(1.0),
        "mlx-4bit" => Some(0.5),
        _ => None,
    }
}

fn device_fit(formats: &[String]) -> DeviceFit {
    let portable = formats
        .iter()
        .any(|f| matches!(f.as_str(), "onnx" | "pytorch" | "safetensors"));
    let mlx = formats.iter().any(|f| f.starts_with("mlx"));
    match (portable, mlx) {
        (true, _) => DeviceFit::CpuPortable,
        (false, true) => DeviceFit::AppleSiliconOnly,
        (false, false) => DeviceFit::Unknown,
    }
}

fn disk_size(disk_mb: Option<f64>, param_count: Option<u64>, formats: &[String]) -> SizeMb {
    if let Some(mb) = disk_mb.filter(|v| v.is_finite() && *v > 0.0) {
        return SizeMb::Stated(mb);
    }
    let Some(params) = param_count else {
        return SizeMb::Unknown;
    };
    // Smallest listed artifact; parameters times bytes per parameter.
    formats
        .iter()
        .filter_map(|f| bytes_per_param(f))
        .reduce(f64::min)
        .map_or(SizeMb::Unknown, |b| {
            #[allow(clippy::cast_precision_loss)]
            let mb = params as f64 * b / 1_048_576.0;
            SizeMb::Estimated((mb * 10.0).round() / 10.0)
        })
}

fn non_empty(field: &str, value: &str, line: usize) -> Result<(), CatalogError> {
    if value.trim().is_empty() {
        return Err(CatalogError::InvalidRow {
            line,
            reason: format!("{field} is empty"),
        });
    }
    Ok(())
}

/// Deterministic catalog-local identifier. This names MedScale's index entry,
/// not the source model or its authored weights; the raw source ID is preserved.
fn medscale_catalog_id(repo_id: &str) -> String {
    let digest = Sha256::digest(repo_id.as_bytes());
    let mut id = String::from("medscale-");
    use std::fmt::Write as _;
    for byte in &digest[..12] {
        write!(&mut id, "{byte:02x}").expect("formatting into String cannot fail");
    }
    id
}

fn row_from_raw(raw: RawRow, line: usize) -> Result<CatalogRow, CatalogError> {
    non_empty("repo_id", &raw.repo_id, line)?;
    non_empty("family", &raw.family, line)?;
    non_empty("task", &raw.task, line)?;
    if raw.repo_id.split('/').count() != 2 || raw.repo_id.chars().any(char::is_whitespace) {
        return Err(CatalogError::InvalidRow {
            line,
            reason: "repo_id must be owner/name".into(),
        });
    }
    let license_claim = raw
        .license
        .as_deref()
        .map(str::to_ascii_lowercase)
        .filter(|l| !l.is_empty());
    let status = match license_claim.as_deref() {
        Some(l) if PERMISSIVE_CLAIMS.contains(&l) => CatalogStatus::Discoverable,
        _ => CatalogStatus::RightsPending,
    };
    // Upstream rows may carry null for these; normalise to explicit values.
    let formats = raw.formats.unwrap_or_default();
    Ok(CatalogRow {
        disk: disk_size(raw.disk_mb, raw.param_count, &formats),
        device_fit: device_fit(&formats),
        medscale_id: medscale_catalog_id(&raw.repo_id),
        repo_id: raw.repo_id,
        family: raw.family,
        task: raw.task,
        languages: raw.languages.unwrap_or_default(),
        tier: raw.tier,
        param_count: raw.param_count,
        architecture: raw.architecture.unwrap_or_else(|| "unknown".to_string()),
        base_model: raw.base_model,
        formats,
        license_claim,
        released: raw.released,
        reproducibility_hash: raw.reproducibility_hash,
        status,
    })
}

/// Filter over catalog rows. `None` means "no constraint".
#[derive(Debug, Default, Clone)]
pub struct CatalogFilter {
    /// Case-insensitive substring over repo id, family, task and architecture.
    pub text: Option<String>,
    pub task: Option<String>,
    pub family: Option<String>,
    pub language: Option<String>,
    pub format: Option<String>,
    pub architecture: Option<String>,
    pub license_claim: Option<String>,
    /// OpenMed size tier (`Tiny`, `Small`, `Base`, `Medium`, `Large`,
    /// `XLarge`); rows without a tier never match a tier filter.
    pub tier: Option<String>,
    pub device_fit: Option<DeviceFit>,
    pub status: Option<CatalogStatus>,
    pub max_params: Option<u64>,
    /// Rows whose disk size (stated or estimated) is at most this; unknown sizes are excluded.
    pub max_disk_mb: Option<f64>,
}

impl CatalogFilter {
    fn matches(&self, row: &CatalogRow) -> bool {
        let eq = |want: &Option<String>, have: &str| {
            want.as_deref().is_none_or(|w| w.eq_ignore_ascii_case(have))
        };
        let text_ok = self.text.as_deref().is_none_or(|t| {
            let t = t.to_ascii_lowercase();
            [&row.repo_id, &row.family, &row.task, &row.architecture]
                .iter()
                .any(|f| f.to_ascii_lowercase().contains(&t))
        });
        let disk_ok = self.max_disk_mb.is_none_or(|max| match row.disk {
            SizeMb::Stated(mb) | SizeMb::Estimated(mb) => mb <= max,
            SizeMb::Unknown => false,
        });
        text_ok
            && eq(&self.task, &row.task)
            && eq(&self.family, &row.family)
            && eq(&self.architecture, &row.architecture)
            && self
                .language
                .as_deref()
                .is_none_or(|l| row.languages.iter().any(|x| x.eq_ignore_ascii_case(l)))
            && self
                .format
                .as_deref()
                .is_none_or(|f| row.formats.iter().any(|x| x.eq_ignore_ascii_case(f)))
            && self.license_claim.as_deref().is_none_or(|l| {
                row.license_claim
                    .as_deref()
                    .is_some_and(|c| c.eq_ignore_ascii_case(l))
            })
            && self.tier.as_deref().is_none_or(|t| {
                row.tier
                    .as_deref()
                    .is_some_and(|r| r.eq_ignore_ascii_case(t))
            })
            && self.device_fit.is_none_or(|d| d == row.device_fit)
            && self.status.is_none_or(|s| s == row.status)
            && self
                .max_params
                .is_none_or(|max| row.param_count.is_some_and(|p| p <= max))
            && disk_ok
    }
}

#[derive(Debug, Serialize)]
pub struct CatalogPage<'a> {
    pub rows: Vec<&'a CatalogRow>,
    /// Number of rows matching the filter (all pages).
    pub total_matching: usize,
    /// Offset of the next page, if any.
    pub next_offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ModelCatalog {
    pub source: CatalogSource,
    rows: Vec<CatalogRow>,
}

impl ModelCatalog {
    /// Imports a JSON Lines manifest snapshot that is already on disk.
    pub fn import(repository: &str, commit: &str, manifest: &[u8]) -> Result<Self, CatalogError> {
        if manifest.len() > MAX_CATALOG_BYTES {
            return Err(CatalogError::TooLarge);
        }
        if commit.len() != 40 || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(CatalogError::InvalidCommit);
        }
        let text = std::str::from_utf8(manifest).map_err(|_| CatalogError::NotUtf8)?;
        let mut rows = Vec::new();
        let mut seen = HashSet::new();
        let mut seen_ids = HashSet::new();
        for (index, line) in text.lines().enumerate() {
            let line_no = index + 1;
            if line.trim().is_empty() {
                continue;
            }
            if line.len() > MAX_LINE_BYTES {
                return Err(CatalogError::LineTooLong { line: line_no });
            }
            if rows.len() == MAX_CATALOG_ROWS {
                return Err(CatalogError::TooManyRows);
            }
            let raw: RawRow = serde_json::from_str(line).map_err(|e| CatalogError::InvalidRow {
                line: line_no,
                reason: e.to_string(),
            })?;
            let row = row_from_raw(raw, line_no)?;
            if !seen.insert(row.repo_id.clone()) {
                return Err(CatalogError::Duplicate {
                    line: line_no,
                    repo_id: row.repo_id,
                });
            }
            if !seen_ids.insert(row.medscale_id.clone()) {
                return Err(CatalogError::InvalidRow {
                    line: line_no,
                    reason: "duplicate MedScale catalog ID".to_owned(),
                });
            }
            rows.push(row);
        }
        rows.sort_by(|a, b| a.repo_id.cmp(&b.repo_id));
        let digest = Sha256::digest(manifest);
        Ok(Self {
            source: CatalogSource {
                repository: repository.to_string(),
                commit: commit.to_ascii_lowercase(),
                manifest_sha256: digest.iter().map(|b| format!("{b:02x}")).collect(),
            },
            rows,
        })
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Every row, sorted by repository id.
    #[must_use]
    pub fn rows(&self) -> &[CatalogRow] {
        &self.rows
    }

    pub fn get(&self, repo_id: &str) -> Option<&CatalogRow> {
        self.rows
            .binary_search_by(|r| r.repo_id.as_str().cmp(repo_id))
            .ok()
            .map(|i| &self.rows[i])
    }

    /// Deterministic page (rows ordered by repo id).
    pub fn query(
        &self,
        filter: &CatalogFilter,
        offset: usize,
        limit: usize,
    ) -> Result<CatalogPage<'_>, CatalogError> {
        if limit == 0 || limit > MAX_PAGE_SIZE {
            return Err(CatalogError::InvalidPageSize);
        }
        let matching: Vec<&CatalogRow> = self.rows.iter().filter(|r| filter.matches(r)).collect();
        let total = matching.len();
        let end = offset.saturating_add(limit).min(total);
        let rows = if offset < total {
            matching[offset..end].to_vec()
        } else {
            Vec::new()
        };
        Ok(CatalogPage {
            rows,
            total_matching: total,
            next_offset: (end < total).then_some(end),
        })
    }

    /// Counts per status. Claims about the catalog must be scoped to these.
    pub fn status_counts(&self) -> BTreeMap<CatalogStatus, usize> {
        let mut counts = BTreeMap::new();
        for row in &self.rows {
            *counts.entry(row.status).or_insert(0) += 1;
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    const COMMIT: &str = "ea920f36fadd7b45935247d639f0ffa1ef493b23";

    fn synthetic(n: usize) -> String {
        let families = ["NER", "PII", "ZeroShot"];
        let archs = ["bert", "xlm-roberta", "deberta-v2", "modernbert"];
        let langs = ["en", "de", "ar", "fr"];
        let mut out = String::new();
        for i in 0..n {
            let formats = match i % 4 {
                0 => r#"["pytorch","onnx"]"#,
                1 => r#"["onnx"]"#,
                2 => r#"["mlx-fp"]"#,
                _ => r#"["pytorch"]"#,
            };
            let license = if i % 50 == 0 { "other" } else { "apache-2.0" };
            writeln!(
                out,
                r#"{{"repo_id":"Synthetic/model-{i:05}","family":"{}","task":"token-classification","languages":["{}"],"tier":null,"param_count":{},"architecture":"{}","base_model":null,"formats":{formats},"canonical_labels":[],"benchmark":{{}},"arxiv":null,"license":"{license}","reproducibility_hash":"sha256:00","released":"2026-01-01"}}"#,
                families[i % 3],
                langs[(i / 4) % 4],
                33_000_000 + (i as u64 % 10) * 50_000_000,
                archs[i % 4],
            )
            .unwrap();
        }
        out
    }

    #[test]
    fn medscale_catalog_ids_are_stable_and_dont_spoof_upstream_ownership() {
        let one = ModelCatalog::import("s/f", COMMIT, synthetic(10).as_bytes()).unwrap();
        let two = ModelCatalog::import("other/fixture", COMMIT, synthetic(10).as_bytes()).unwrap();
        let mut unique = std::collections::HashSet::new();
        for (a, b) in one.rows().iter().zip(two.rows()) {
            assert!(a.medscale_id.starts_with("medscale-"));
            assert_eq!(a.medscale_id.len(), 33);
            assert_eq!(a.medscale_id, b.medscale_id);
            assert!(!a.medscale_id.contains("Synthetic"));
            assert!(unique.insert(a.medscale_id.clone()));
            assert_eq!(a.repo_id, b.repo_id);
        }
        assert_eq!(unique.len(), 10);
    }

    #[test]
    fn imports_ten_thousand_rows_offline_with_source_binding() {
        let text = synthetic(10_000);
        let started = std::time::Instant::now();
        let catalog = ModelCatalog::import("synthetic/fixture", COMMIT, text.as_bytes()).unwrap();
        assert!(started.elapsed().as_secs() < 10, "import budget");
        assert_eq!(catalog.len(), 10_000);
        assert_eq!(catalog.source.commit, COMMIT);
        assert_eq!(catalog.source.manifest_sha256.len(), 64);
        let counts = catalog.status_counts();
        assert_eq!(counts.get(&CatalogStatus::RightsPending), Some(&200));
        assert_eq!(counts.get(&CatalogStatus::Discoverable), Some(&9_800));
        // Import never promotes beyond listing.
        assert!(counts.keys().all(|s| *s <= CatalogStatus::RightsPending));
    }

    #[test]
    fn pagination_is_complete_and_deterministic() {
        let catalog = ModelCatalog::import("s/f", COMMIT, synthetic(1_234).as_bytes()).unwrap();
        let filter = CatalogFilter::default();
        let mut seen = Vec::new();
        let mut offset = Some(0);
        while let Some(o) = offset {
            let page = catalog.query(&filter, o, 500).unwrap();
            assert_eq!(page.total_matching, 1_234);
            seen.extend(page.rows.iter().map(|r| r.repo_id.clone()));
            offset = page.next_offset;
        }
        assert_eq!(seen.len(), 1_234);
        let mut sorted = seen.clone();
        sorted.sort();
        assert_eq!(seen, sorted);
        assert!(catalog.query(&filter, 0, 0).is_err());
        assert!(catalog.query(&filter, 0, MAX_PAGE_SIZE + 1).is_err());
        assert!(catalog.query(&filter, 99_999, 10).unwrap().rows.is_empty());
    }

    #[test]
    fn tier_filter_is_case_insensitive_and_skips_rows_without_tier() {
        let rows = [
            r#"{"repo_id":"a/tiny","family":"NER","task":"token-classification","tier":"Tiny","architecture":"bert","formats":["onnx"],"license":"apache-2.0"}"#,
            r#"{"repo_id":"a/base","family":"NER","task":"token-classification","tier":"Base","architecture":"bert","formats":["onnx"],"license":"apache-2.0"}"#,
            r#"{"repo_id":"a/none","family":"NER","task":"token-classification","tier":null,"architecture":"bert","formats":["onnx"],"license":"apache-2.0"}"#,
        ]
        .join("
");
        let catalog = ModelCatalog::import("s/f", COMMIT, rows.as_bytes()).unwrap();
        let tiny = CatalogFilter {
            tier: Some("tiny".into()),
            ..Default::default()
        };
        let page = catalog.query(&tiny, 0, 10).unwrap();
        assert_eq!(page.total_matching, 1);
        assert_eq!(page.rows[0].repo_id, "a/tiny");
        let xlarge = CatalogFilter {
            tier: Some("XLarge".into()),
            ..Default::default()
        };
        assert_eq!(catalog.query(&xlarge, 0, 10).unwrap().total_matching, 0);
        assert_eq!(
            catalog
                .query(&CatalogFilter::default(), 0, 10)
                .unwrap()
                .total_matching,
            3
        );
    }

    #[test]
    fn filters_compose() {
        let catalog = ModelCatalog::import("s/f", COMMIT, synthetic(400).as_bytes()).unwrap();
        let f = CatalogFilter {
            language: Some("ar".into()),
            device_fit: Some(DeviceFit::CpuPortable),
            ..Default::default()
        };
        let page = catalog.query(&f, 0, 500).unwrap();
        assert!(page.total_matching > 0);
        assert!(
            page.rows
                .iter()
                .all(|r| r.languages == ["ar"] && r.device_fit == DeviceFit::CpuPortable)
        );
        let mlx = CatalogFilter {
            device_fit: Some(DeviceFit::AppleSiliconOnly),
            ..Default::default()
        };
        assert!(
            catalog
                .query(&mlx, 0, 500)
                .unwrap()
                .rows
                .iter()
                .all(|r| r.formats == ["mlx-fp"])
        );
        let text = CatalogFilter {
            text: Some("MODEL-0001".into()),
            ..Default::default()
        };
        assert_eq!(catalog.query(&text, 0, 500).unwrap().total_matching, 10);
        let small = CatalogFilter {
            max_disk_mb: Some(200.0),
            ..Default::default()
        };
        assert!(
            catalog
                .query(&small, 0, 500)
                .unwrap()
                .rows
                .iter()
                .all(|r| matches!(r.disk, SizeMb::Estimated(mb) if mb <= 200.0))
        );
    }

    #[test]
    fn sizes_are_labelled_stated_estimated_or_unknown() {
        let rows = concat!(
            r#"{"repo_id":"a/stated","family":"NER","task":"t","formats":["onnx"],"param_count":1000000,"license":"mit","disk_mb":12.5}"#,
            "\n",
            r#"{"repo_id":"a/estimated","family":"NER","task":"t","formats":["onnx","mlx-4bit"],"param_count":2097152,"license":"mit"}"#,
            "\n",
            r#"{"repo_id":"a/unknown","family":"NER","task":"t","formats":["gguf"],"param_count":10,"license":"mit"}"#,
            "\n",
        );
        let c = ModelCatalog::import("s/f", COMMIT, rows.as_bytes()).unwrap();
        assert_eq!(c.get("a/stated").unwrap().disk, SizeMb::Stated(12.5));
        // mlx-4bit: 2,097,152 params * 0.5 B = 1 MiB.
        assert_eq!(c.get("a/estimated").unwrap().disk, SizeMb::Estimated(1.0));
        assert_eq!(c.get("a/unknown").unwrap().disk, SizeMb::Unknown);
        assert_eq!(c.get("a/unknown").unwrap().device_fit, DeviceFit::Unknown);
    }

    #[test]
    fn rejects_malformed_duplicate_and_hostile_input() {
        let ok = r#"{"repo_id":"a/b","family":"NER","task":"t","license":"mit"}"#;
        assert!(matches!(
            ModelCatalog::import("s/f", COMMIT, format!("{ok}\n{ok}\n").as_bytes()),
            Err(CatalogError::Duplicate { line: 2, .. })
        ));
        assert!(matches!(
            ModelCatalog::import("s/f", COMMIT, b"{not json}\n"),
            Err(CatalogError::InvalidRow { line: 1, .. })
        ));
        assert!(matches!(
            ModelCatalog::import(
                "s/f",
                COMMIT,
                br#"{"repo_id":"no-owner","family":"NER","task":"t"}"#
            ),
            Err(CatalogError::InvalidRow { .. })
        ));
        assert!(matches!(
            ModelCatalog::import(
                "s/f",
                COMMIT,
                br#"{"repo_id":"a/b","family":"","task":"t"}"#
            ),
            Err(CatalogError::InvalidRow { .. })
        ));
        assert_eq!(
            ModelCatalog::import("s/f", "main", ok.as_bytes()).unwrap_err(),
            CatalogError::InvalidCommit
        );
        assert_eq!(
            ModelCatalog::import("s/f", COMMIT, &[0xff, 0xfe]).unwrap_err(),
            CatalogError::NotUtf8
        );
        let long = format!(
            r#"{{"repo_id":"a/b","family":"NER","task":"{}"}}"#,
            "x".repeat(MAX_LINE_BYTES)
        );
        assert_eq!(
            ModelCatalog::import("s/f", COMMIT, long.as_bytes()).unwrap_err(),
            CatalogError::LineTooLong { line: 1 }
        );
    }

    #[test]
    fn null_optional_fields_are_normalised() {
        // OpenMed v3.0.0 has rows with architecture: null (found by the
        // snapshot evidence run); languages/formats are tolerated too.
        let row = r#"{"repo_id":"a/b","family":"NER","task":"t","architecture":null,"languages":null,"formats":null,"param_count":null,"license":"mit"}"#;
        let c = ModelCatalog::import("s/f", COMMIT, row.as_bytes()).unwrap();
        let r = c.get("a/b").unwrap();
        assert_eq!(r.architecture, "unknown");
        assert!(r.languages.is_empty() && r.formats.is_empty());
        assert_eq!(
            (r.disk, r.device_fit),
            (SizeMb::Unknown, DeviceFit::Unknown)
        );
    }

    #[test]
    fn missing_or_other_license_is_rights_pending() {
        let rows = concat!(
            r#"{"repo_id":"a/none","family":"NER","task":"t"}"#,
            "\n",
            r#"{"repo_id":"a/other","family":"NER","task":"t","license":"other"}"#,
            "\n",
            r#"{"repo_id":"a/apache","family":"NER","task":"t","license":"Apache-2.0"}"#,
            "\n",
        );
        let c = ModelCatalog::import("s/f", COMMIT, rows.as_bytes()).unwrap();
        assert_eq!(
            c.get("a/none").unwrap().status,
            CatalogStatus::RightsPending
        );
        assert_eq!(
            c.get("a/other").unwrap().status,
            CatalogStatus::RightsPending
        );
        assert_eq!(
            c.get("a/apache").unwrap().status,
            CatalogStatus::Discoverable
        );
    }

    /// Evidence run against a real upstream snapshot supplied locally, e.g.
    /// MEDSCALE_CATALOG_SNAPSHOT=path/to/models.jsonl
    /// MEDSCALE_CATALOG_COMMIT=<40-hex> cargo test -p medscale-pack -- --ignored catalog_snapshot
    #[test]
    #[ignore = "needs a locally supplied upstream manifest snapshot"]
    fn catalog_snapshot_evidence() {
        let path = std::env::var("MEDSCALE_CATALOG_SNAPSHOT").unwrap();
        let commit = std::env::var("MEDSCALE_CATALOG_COMMIT").unwrap();
        let bytes = std::fs::read(path).unwrap();
        let catalog = ModelCatalog::import("snapshot", &commit, &bytes).unwrap();
        let portable = CatalogFilter {
            device_fit: Some(DeviceFit::CpuPortable),
            ..Default::default()
        };
        let onnx = CatalogFilter {
            format: Some("onnx".into()),
            ..Default::default()
        };
        let arabic = CatalogFilter {
            language: Some("ar".into()),
            ..Default::default()
        };
        println!(
            "CATALOG_EVIDENCE sha256={} rows={} status={:?} cpu_portable={} onnx={} arabic={}",
            catalog.source.manifest_sha256,
            catalog.len(),
            catalog.status_counts(),
            catalog.query(&portable, 0, 1).unwrap().total_matching,
            catalog.query(&onnx, 0, 1).unwrap().total_matching,
            catalog.query(&arabic, 0, 1).unwrap().total_matching
        );
    }
}
