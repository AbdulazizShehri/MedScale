//! Hugging Face snapshot verification and Pack building (Spec 103 VERIFY/ADMIT).
//!
//! Ported from OpenMed `openmed/core/model_integrity.py` at commit
//! `ea920f36fadd7b45935247d639f0ffa1ef493b23` (Apache-2.0):
//! - `_remote_reproducibility_hash`: SHA-256 over canonical JSON of repo id,
//!   commit, release date and sorted file names. This binds a catalog row to an
//!   immutable Hub commit and file list;
//! - `_verified_remote_artifact`: per-file SHA-256 for LFS files, Git blob
//!   SHA-1 for small files.
//!
//! This module performs no network access. Hub metadata and files are supplied
//! by the caller (an explicit, governed acquisition step). It only verifies and
//! builds a Pack, which then goes through the normal signed admission.

use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path};

use medscale_contracts::objects::DigestSha256;
use medscale_contracts::packs::ModelSourceProvenance;
use medscale_keys::{SYNTHETIC_PACK_TRUST_ROOT_ID, pack_signing_payload, sign_pack_payload};
use serde::Deserialize;
use serde_json::{Value, json};
use sha1::Sha1;
use sha2::{Digest, Sha256};

use crate::ONNX_TOKEN_CLASSIFIER_RUNTIME_ID;
use crate::catalog::CatalogRow;

#[derive(Debug, Clone, Deserialize)]
pub struct HfLfs {
    pub sha256: String,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HfSibling {
    pub rfilename: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default, rename = "blobId")]
    pub blob_id: Option<String>,
    #[serde(default)]
    pub lfs: Option<HfLfs>,
}

/// Subset of the Hub `GET /api/models/{repo}?blobs=true` response.
#[derive(Debug, Clone, Deserialize)]
pub struct HfRepoMetadata {
    pub id: String,
    pub sha: String,
    #[serde(default, rename = "lastModified")]
    pub last_modified: Option<String>,
    #[serde(default, rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(default)]
    pub gated: Value,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub siblings: Vec<HfSibling>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SnapshotError {
    #[error("invalid Hub metadata: {0}")]
    Metadata(String),
    #[error("repository {metadata} does not match catalog row {catalog}")]
    RepositoryMismatch { metadata: String, catalog: String },
    #[error("reproducibility hash mismatch: catalog {catalog}, recomputed {recomputed}")]
    ReproducibilityMismatch { catalog: String, recomputed: String },
    #[error("catalog row has no reproducibility hash")]
    MissingReproducibilityHash,
    #[error("model is gated or private; not admitted")]
    Gated,
    #[error("{0} is not listed in the pinned snapshot")]
    NotInSnapshot(String),
    #[error("{path}: digest mismatch")]
    DigestMismatch { path: String },
    #[error("{path}: size mismatch")]
    SizeMismatch { path: String },
    #[error("{0}: no verifiable digest in Hub metadata")]
    NoDigest(String),
    #[error("unsafe artifact path {0}")]
    UnsafePath(String),
    #[error("refused artifact {0}: only ONNX and JSON/tokenizer files are admitted")]
    RefusedArtifact(String),
    #[error("license claim {0:?} is not admitted without review")]
    RightsPending(Option<String>),
    #[error("config.json has no usable id2label")]
    Labels,
    #[error("tokenizer source refused: {0}")]
    TokenizerSource(String),
    #[error("io: {0}")]
    Io(String),
}

fn io(e: &std::io::Error) -> SnapshotError {
    SnapshotError::Io(e.to_string())
}

/// Python `json.dumps(..., ensure_ascii=True)` string encoding.
fn py_json_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf) {
                    let _ = write!(out, "\\u{unit:04x}");
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `released` as OpenMed computes it: date of `lastModified`, else `createdAt`.
fn released(meta: &HfRepoMetadata) -> Option<String> {
    meta.last_modified
        .as_deref()
        .or(meta.created_at.as_deref())
        .and_then(|ts| ts.get(..10))
        .map(ToString::to_string)
}

/// Port of OpenMed `_remote_reproducibility_hash` (canonical JSON, sorted keys,
/// compact separators, ASCII-escaped).
pub fn reproducibility_hash(meta: &HfRepoMetadata) -> String {
    let mut names: Vec<&str> = meta.siblings.iter().map(|s| s.rfilename.as_str()).collect();
    names.sort_unstable();
    let mut payload = String::from("{\"released\":");
    match released(meta) {
        Some(r) => py_json_string(&mut payload, &r),
        None => payload.push_str("null"),
    }
    payload.push_str(",\"repo_id\":");
    py_json_string(&mut payload, &meta.id);
    payload.push_str(",\"sha\":");
    py_json_string(&mut payload, &meta.sha);
    payload.push_str(",\"siblings\":[");
    for (i, name) in names.iter().enumerate() {
        if i > 0 {
            payload.push(',');
        }
        py_json_string(&mut payload, name);
    }
    payload.push_str("]}");
    format!("sha256:{}", hex(&Sha256::digest(payload.as_bytes())))
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Binds Hub metadata to a catalog row: same repository, same reproducibility
/// hash (hence the same immutable commit and file list), not gated or private.
pub fn verify_against_catalog(
    meta: &HfRepoMetadata,
    row: &CatalogRow,
) -> Result<(), SnapshotError> {
    if !is_hex(&meta.sha, 40) {
        return Err(SnapshotError::Metadata(
            "sha must be a 40-hex commit".into(),
        ));
    }
    if meta.id != row.repo_id {
        return Err(SnapshotError::RepositoryMismatch {
            metadata: meta.id.clone(),
            catalog: row.repo_id.clone(),
        });
    }
    if meta.private || !matches!(meta.gated, Value::Null | Value::Bool(false)) {
        return Err(SnapshotError::Gated);
    }
    let expected = row
        .reproducibility_hash
        .as_deref()
        .ok_or(SnapshotError::MissingReproducibilityHash)?;
    let recomputed = reproducibility_hash(meta);
    if recomputed != expected {
        return Err(SnapshotError::ReproducibilityMismatch {
            catalog: expected.to_string(),
            recomputed,
        });
    }
    Ok(())
}

fn safe_relative(path: &str) -> Result<&Path, SnapshotError> {
    let p = Path::new(path);
    if p.as_os_str().is_empty() || !p.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(SnapshotError::UnsafePath(path.to_string()));
    }
    Ok(p)
}

/// Only formats that carry no executable code are admitted.
fn admitted_artifact(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".onnx") || lower.ends_with(".json") || lower.ends_with(".safetensors")
}

/// Port of OpenMed `_verified_remote_artifact`: SHA-256 for LFS files, Git
/// blob SHA-1 (`blob <size>\0<bytes>`) for small files. Returns the SHA-256.
pub fn verify_file(
    meta: &HfRepoMetadata,
    rfilename: &str,
    bytes: &[u8],
) -> Result<String, SnapshotError> {
    safe_relative(rfilename)?;
    if !admitted_artifact(rfilename) {
        return Err(SnapshotError::RefusedArtifact(rfilename.to_string()));
    }
    let sibling = meta
        .siblings
        .iter()
        .find(|s| s.rfilename == rfilename)
        .ok_or_else(|| SnapshotError::NotInSnapshot(rfilename.to_string()))?;
    let size = bytes.len() as u64;
    let sha256 = hex(&Sha256::digest(bytes));
    let mismatch = || SnapshotError::DigestMismatch {
        path: rfilename.to_string(),
    };
    if let Some(lfs) = &sibling.lfs {
        if lfs.size.is_some_and(|s| s != size) {
            return Err(SnapshotError::SizeMismatch {
                path: rfilename.to_string(),
            });
        }
        if !lfs.sha256.eq_ignore_ascii_case(&sha256) {
            return Err(mismatch());
        }
    } else {
        let blob = sibling
            .blob_id
            .as_deref()
            .ok_or_else(|| SnapshotError::NoDigest(rfilename.to_string()))?;
        let mut h = Sha1::new();
        h.update(format!("blob {size}\0").as_bytes());
        h.update(bytes);
        if !blob.eq_ignore_ascii_case(&hex(&h.finalize())) {
            return Err(mismatch());
        }
    }
    if sibling.size.is_some_and(|s| s != size) {
        return Err(SnapshotError::SizeMismatch {
            path: rfilename.to_string(),
        });
    }
    Ok(sha256)
}

/// Labels ordered by index from a Transformers `config.json`.
pub fn labels_from_config(config: &[u8]) -> Result<Vec<String>, SnapshotError> {
    let value: Value = serde_json::from_slice(config).map_err(|_| SnapshotError::Labels)?;
    let map = value
        .get("id2label")
        .and_then(Value::as_object)
        .ok_or(SnapshotError::Labels)?;
    let mut labels = vec![None; map.len()];
    for (k, v) in map {
        let index: usize = k.parse().map_err(|_| SnapshotError::Labels)?;
        let slot = labels.get_mut(index).ok_or(SnapshotError::Labels)?;
        *slot = Some(v.as_str().ok_or(SnapshotError::Labels)?.to_string());
    }
    labels
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .filter(|l| !l.is_empty())
        .ok_or(SnapshotError::Labels)
}

/// Licenses that may be admitted from the row claim alone; anything else needs
/// a recorded card-level review first (Spec 103 R05).
fn spdx_for(claim: Option<&str>) -> Result<&'static str, SnapshotError> {
    match claim {
        Some("apache-2.0") => Ok("Apache-2.0"),
        Some("mit") => Ok("MIT"),
        other => Err(SnapshotError::RightsPending(other.map(ToString::to_string))),
    }
}

fn normalized_text(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            out.push(b'\n');
            if bytes.get(i + 1) == Some(&b'\n') {
                i += 1;
            }
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    out
}

/// Inputs of a text-generation Pack built from a verified snapshot (Spec 105).
pub struct TextGenerationSnapshot<'a> {
    pub meta: &'a HfRepoMetadata,
    pub row: &'a CatalogRow,
    pub snapshot_dir: &'a Path,
    /// Data-only weights, for example `model.safetensors`.
    pub weight_file: &'a str,
}

/// Verifies a text-generation snapshot against the catalog and Hub digests,
/// then writes a signed Pack (weights, config, tokenizer, provenance) for the
/// candle text-generation runtime. The catalog row must be a
/// `text-generation` row; only `.safetensors` weights are admitted, under the
/// same per-artifact bound as every Pack. Signed with the synthetic
/// qualification trust root.
pub fn build_text_generation_pack(
    input: &TextGenerationSnapshot<'_>,
    out_dir: &Path,
) -> Result<String, SnapshotError> {
    if input.row.task != "text-generation" {
        return Err(SnapshotError::Metadata(format!(
            "catalog task {:?} is not text-generation",
            input.row.task
        )));
    }
    if !input
        .weight_file
        .to_ascii_lowercase()
        .ends_with(".safetensors")
    {
        return Err(SnapshotError::RefusedArtifact(input.weight_file.to_owned()));
    }
    verify_against_catalog(input.meta, input.row)?;
    let license = spdx_for(input.row.license_claim.as_deref())?;
    let read = |name: &str| -> Result<Vec<u8>, SnapshotError> {
        fs::read(input.snapshot_dir.join(safe_relative(name)?)).map_err(|e| io(&e))
    };
    let runtime_id = crate::generation_runtime::CANDLE_TEXT_GENERATION_RUNTIME_ID;
    let model = read(input.weight_file)?;
    let model_sha = verify_file(input.meta, input.weight_file, &model)?;
    let tokenizer = read("tokenizer.json")?;
    verify_file(input.meta, "tokenizer.json", &tokenizer)?;
    let config = read("config.json")?;
    verify_file(input.meta, "config.json", &config)?;
    let context = serde_json::from_slice::<Value>(&config)
        .ok()
        .and_then(|c| c["max_position_embeddings"].as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| {
            SnapshotError::Metadata("config.json has no max_position_embeddings".into())
        })?;

    let revision = input.meta.sha.clone();
    let rights_uri = format!(
        "https://huggingface.co/{}/blob/{revision}/README.md",
        input.meta.id
    );
    let provenance = ModelSourceProvenance {
        source_kind: "hugging_face".into(),
        repository: input.meta.id.clone(),
        revision: revision.clone(),
        original_file: input.weight_file.to_string(),
        source_sha256: model_sha,
        transform: "none".into(),
        fixed_sequence_length: context,
        task: "text-generation".into(),
        export_format: "safetensors".into(),
        runtime_family: runtime_id.into(),
        license_id: license.into(),
        upstream_rights_uri: rights_uri.clone(),
    };
    fs::create_dir_all(out_dir).map_err(|e| io(&e))?;
    let write = |name: &str, bytes: &[u8]| fs::write(out_dir.join(name), bytes).map_err(|e| io(&e));
    write("model.safetensors", &model)?;
    write("config.json", &config)?;
    write("tokenizer.json", &tokenizer)?;
    write(
        "model.meta.json",
        (serde_json::to_string_pretty(&provenance)
            .map_err(|e| SnapshotError::Io(e.to_string()))?
            + "\n")
            .as_bytes(),
    )?;
    let mut rows = Vec::new();
    for (name, kind, text) in [
        ("config.json", "model_metadata", true),
        ("model.meta.json", "model_metadata", true),
        ("model.safetensors", "safetensors_model", false),
        ("tokenizer.json", "tokenizer_meta", true),
    ] {
        let bytes = fs::read(out_dir.join(name)).map_err(|e| io(&e))?;
        let bytes = if text { normalized_text(&bytes) } else { bytes };
        rows.push(json!({"relative_path": name, "kind": kind, "digest": DigestSha256::of(&bytes).to_hex()}));
    }
    let digest_lines: String = rows
        .iter()
        .map(|r| format!("{}\n", r["digest"].as_str().unwrap_or_default()))
        .collect();
    let content_digest = DigestSha256::of(digest_lines.as_bytes()).to_hex();
    let short = input
        .meta
        .id
        .rsplit('/')
        .next()
        .unwrap_or("model")
        .to_ascii_lowercase();
    let pack_id = format!("hf-{short}-generation");
    let sbom_ref = format!("hf:{}@{revision}#{}", input.meta.id, input.weight_file);
    let payload = pack_signing_payload(
        &pack_id,
        "1.0.0",
        1,
        &content_digest,
        &rights_uri,
        &sbom_ref,
    );
    let manifest = json!({
        "pack_id": pack_id,
        "version": "1.0.0",
        "pack_epoch": 1,
        "content_digest": content_digest,
        "artifacts": rows,
        "rights_uri": rights_uri,
        "sbom_ref": sbom_ref,
        "runtime_requirements": format!("{runtime_id};synthetic_only=true;context={context}"),
        "benchmark_links": ["specs/105-runtime-families/spec.md"],
        "promotion_state": "candidate",
        "trust_root_id": SYNTHETIC_PACK_TRUST_ROOT_ID,
        "signature_hex": sign_pack_payload(&payload),
    });
    write(
        "pack.manifest.json",
        (serde_json::to_string_pretty(&manifest).map_err(|e| SnapshotError::Io(e.to_string()))?
            + "\n")
            .as_bytes(),
    )?;
    Ok(pack_id)
}

/// Verifies a pinned Whisper snapshot against its Hub metadata and writes a
/// signed Audio Pack (weights, config, tokenizer, provenance) for the candle
/// Whisper ASR runtime (Spec 106). Only the repositories and revisions in
/// [`crate::ASR_SOURCES`] are admitted (their model cards declare
/// Apache-2.0); weights must be `model.safetensors` under the same
/// per-artifact bound as every Pack. Signed with the synthetic qualification
/// trust root.
pub fn build_asr_pack(
    meta: &HfRepoMetadata,
    snapshot_dir: &Path,
    out_dir: &Path,
) -> Result<String, SnapshotError> {
    if !crate::asr_runtime::ASR_SOURCES
        .iter()
        .any(|(repo, sha)| *repo == meta.id && *sha == meta.sha)
    {
        return Err(SnapshotError::Metadata(format!(
            "{}@{} is not an admitted ASR source",
            meta.id, meta.sha
        )));
    }
    if meta.private || meta.gated.as_bool().unwrap_or(false) || meta.gated.is_string() {
        return Err(SnapshotError::Gated);
    }
    let read = |name: &str| -> Result<Vec<u8>, SnapshotError> {
        fs::read(snapshot_dir.join(safe_relative(name)?)).map_err(|e| io(&e))
    };
    let runtime_id = crate::asr_runtime::CANDLE_WHISPER_ASR_RUNTIME_ID;
    let weight_file = "model.safetensors";
    let model = read(weight_file)?;
    let model_sha = verify_file(meta, weight_file, &model)?;
    let tokenizer = read("tokenizer.json")?;
    verify_file(meta, "tokenizer.json", &tokenizer)?;
    let config = read("config.json")?;
    verify_file(meta, "config.json", &config)?;
    let target = serde_json::from_slice::<Value>(&config)
        .ok()
        .and_then(|c| c["max_target_positions"].as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| SnapshotError::Metadata("config.json has no max_target_positions".into()))?;

    let revision = meta.sha.clone();
    let rights_uri = format!(
        "https://huggingface.co/{}/blob/{revision}/README.md",
        meta.id
    );
    let provenance = ModelSourceProvenance {
        source_kind: "hugging_face".into(),
        repository: meta.id.clone(),
        revision: revision.clone(),
        original_file: weight_file.to_string(),
        source_sha256: model_sha,
        transform: "none".into(),
        fixed_sequence_length: target,
        task: "automatic-speech-recognition".into(),
        export_format: "safetensors".into(),
        runtime_family: runtime_id.into(),
        license_id: "Apache-2.0".into(),
        upstream_rights_uri: rights_uri.clone(),
    };
    fs::create_dir_all(out_dir).map_err(|e| io(&e))?;
    let write = |name: &str, bytes: &[u8]| fs::write(out_dir.join(name), bytes).map_err(|e| io(&e));
    write("model.safetensors", &model)?;
    write("config.json", &config)?;
    write("tokenizer.json", &tokenizer)?;
    write(
        "model.meta.json",
        (serde_json::to_string_pretty(&provenance)
            .map_err(|e| SnapshotError::Io(e.to_string()))?
            + "\n")
            .as_bytes(),
    )?;
    let mut rows = Vec::new();
    for (name, kind, text) in [
        ("config.json", "model_metadata", true),
        ("model.meta.json", "model_metadata", true),
        ("model.safetensors", "safetensors_model", false),
        ("tokenizer.json", "tokenizer_meta", true),
    ] {
        let bytes = fs::read(out_dir.join(name)).map_err(|e| io(&e))?;
        let bytes = if text { normalized_text(&bytes) } else { bytes };
        rows.push(json!({"relative_path": name, "kind": kind, "digest": DigestSha256::of(&bytes).to_hex()}));
    }
    let digest_lines: String = rows
        .iter()
        .map(|r| format!("{}\n", r["digest"].as_str().unwrap_or_default()))
        .collect();
    let content_digest = DigestSha256::of(digest_lines.as_bytes()).to_hex();
    let short = meta
        .id
        .rsplit('/')
        .next()
        .unwrap_or("model")
        .to_ascii_lowercase();
    let pack_id = format!("hf-{short}-asr");
    let sbom_ref = format!("hf:{}@{revision}#{weight_file}", meta.id);
    let payload = pack_signing_payload(
        &pack_id,
        "1.0.0",
        1,
        &content_digest,
        &rights_uri,
        &sbom_ref,
    );
    let manifest = json!({
        "pack_id": pack_id,
        "version": "1.0.0",
        "pack_epoch": 1,
        "content_digest": content_digest,
        "artifacts": rows,
        "rights_uri": rights_uri,
        "sbom_ref": sbom_ref,
        "runtime_requirements": format!("{runtime_id};synthetic_only=true"),
        "benchmark_links": ["specs/106-local-asr/spec.md"],
        "promotion_state": "candidate",
        "trust_root_id": SYNTHETIC_PACK_TRUST_ROOT_ID,
        "signature_hex": sign_pack_payload(&payload),
    });
    write(
        "pack.manifest.json",
        (serde_json::to_string_pretty(&manifest).map_err(|e| SnapshotError::Io(e.to_string()))?
            + "\n")
            .as_bytes(),
    )?;
    Ok(pack_id)
}

/// The catalog repository a snapshot borrows `tokenizer.json` from.
///
/// OpenMed's MLX exports ship no tokenizer; OpenMed and OpenMedKit fall back
/// to the source checkpoint named by `_name_or_path` in `config.json`. MedScale
/// does the same, but only for an `OpenMed/` repository that is itself a
/// catalog row, with its own metadata verified against that row and the
/// tokenizer verified against its own pinned digest.
pub struct TokenizerSource<'a> {
    pub meta: &'a HfRepoMetadata,
    pub row: &'a CatalogRow,
}

/// The tokenizer source a snapshot's `config.json` names (`_name_or_path`),
/// when it is a plain `OpenMed/<name>` repository other than `own_repo`.
#[must_use]
pub fn tokenizer_source_repo(config: &[u8], own_repo: &str) -> Option<String> {
    let value: Value = serde_json::from_slice(config).ok()?;
    let name = value.get("_name_or_path")?.as_str()?;
    let model = name.strip_prefix("OpenMed/")?;
    let plain = !model.is_empty()
        && model
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        && !model.starts_with('.');
    (plain && name != own_repo).then(|| name.to_owned())
}

/// Metadata of the tokenizer source an acquisition stored beside the
/// snapshot (`tokenizer_source/metadata.json`), when there is one.
pub fn snapshot_tokenizer_source(
    snapshot_dir: &Path,
) -> Result<Option<HfRepoMetadata>, SnapshotError> {
    let path = snapshot_dir.join("tokenizer_source").join("metadata.json");
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| SnapshotError::Metadata(format!("tokenizer_source/metadata.json: {e}"))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io(&e)),
    }
}

/// Inputs of a token-classification Pack built from a verified snapshot.
pub struct TokenClassifierSnapshot<'a> {
    pub meta: &'a HfRepoMetadata,
    pub row: &'a CatalogRow,
    /// Directory holding the downloaded snapshot files (Hub relative paths).
    pub snapshot_dir: &'a Path,
    /// ONNX file to run, for example `model.onnx`.
    pub onnx_file: &'a str,
    pub fixed_sequence_length: u32,
    /// Set when the repository ships no `tokenizer.json` (OpenMed MLX
    /// exports); the snapshot's `tokenizer.json` is then the source's.
    pub tokenizer_source: Option<TokenizerSource<'a>>,
}

/// Verifies the snapshot against the catalog and Hub digests, then writes a
/// Pack (model, tokenizer, labels, provenance, signed manifest) to `out_dir`.
/// The manifest is signed with the repository's **synthetic** qualification
/// trust root; production signing is not granted.
pub fn build_token_classifier_pack(
    input: &TokenClassifierSnapshot<'_>,
    out_dir: &Path,
) -> Result<String, SnapshotError> {
    verify_against_catalog(input.meta, input.row)?;
    let license = spdx_for(input.row.license_claim.as_deref())?;
    let read = |name: &str| -> Result<Vec<u8>, SnapshotError> {
        fs::read(input.snapshot_dir.join(safe_relative(name)?)).map_err(|e| io(&e))
    };
    // Spec 104: a `.safetensors` weight file builds a candle-runtime Pack.
    let safetensors = input
        .onnx_file
        .to_ascii_lowercase()
        .ends_with(".safetensors");
    let (runtime_id, export_format, model_name, model_kind) = if safetensors {
        (
            crate::candle_runtime::CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID,
            "safetensors",
            "model.safetensors",
            "safetensors_model",
        )
    } else {
        (
            ONNX_TOKEN_CLASSIFIER_RUNTIME_ID,
            "onnx",
            "model.onnx",
            "onnx_model",
        )
    };
    let model = read(input.onnx_file)?;
    let model_sha = verify_file(input.meta, input.onnx_file, &model)?;
    let config = read("config.json")?;
    verify_file(input.meta, "config.json", &config)?;
    let tokenizer = read("tokenizer.json")?;
    let tokenizer_source = match &input.tokenizer_source {
        None => {
            verify_file(input.meta, "tokenizer.json", &tokenizer)?;
            None
        }
        Some(source) => {
            let own = input
                .meta
                .siblings
                .iter()
                .any(|s| s.rfilename == "tokenizer.json");
            if own {
                return Err(SnapshotError::TokenizerSource(
                    "the repository ships its own tokenizer.json".into(),
                ));
            }
            if tokenizer_source_repo(&config, &input.meta.id).as_deref()
                != Some(source.meta.id.as_str())
            {
                return Err(SnapshotError::TokenizerSource(format!(
                    "config.json does not name {} as its source",
                    source.meta.id
                )));
            }
            verify_against_catalog(source.meta, source.row)?;
            // The borrowed file carries its own repository's license claim,
            // which must be admitted like the export's.
            spdx_for(source.row.license_claim.as_deref())?;
            let sha256 = verify_file(source.meta, "tokenizer.json", &tokenizer)?;
            Some(json!({
                "repository": source.meta.id,
                "revision": source.meta.sha,
                "file": "tokenizer.json",
                "sha256": sha256,
                "reason": "the repository ships no tokenizer; config.json _name_or_path names this catalog repository",
            }))
        }
    };
    let labels = labels_from_config(&config)?;

    let revision = input.meta.sha.clone();
    let rights_uri = format!(
        "https://huggingface.co/{}/blob/{revision}/README.md",
        input.meta.id
    );
    let provenance = ModelSourceProvenance {
        source_kind: "hugging_face".into(),
        repository: input.meta.id.clone(),
        revision: revision.clone(),
        original_file: input.onnx_file.to_string(),
        source_sha256: model_sha,
        transform: "none".into(),
        fixed_sequence_length: input.fixed_sequence_length,
        task: "token-classification".into(),
        export_format: export_format.into(),
        runtime_family: runtime_id.into(),
        license_id: license.into(),
        upstream_rights_uri: rights_uri.clone(),
    };
    fs::create_dir_all(out_dir).map_err(|e| io(&e))?;
    let write = |name: &str, bytes: &[u8]| fs::write(out_dir.join(name), bytes).map_err(|e| io(&e));
    write(model_name, &model)?;
    if safetensors {
        // The candle runtime builds the encoder from the verified config.
        write("config.json", &config)?;
    }
    write("tokenizer.json", &tokenizer)?;
    write(
        "labels.json",
        (serde_json::to_string(&labels).map_err(|e| SnapshotError::Io(e.to_string()))? + "\n")
            .as_bytes(),
    )?;
    write(
        "model.meta.json",
        (serde_json::to_string_pretty(&provenance)
            .map_err(|e| SnapshotError::Io(e.to_string()))?
            + "\n")
            .as_bytes(),
    )?;
    if let Some(source) = &tokenizer_source {
        write(
            "tokenizer_source.json",
            (serde_json::to_string_pretty(source).map_err(|e| SnapshotError::Io(e.to_string()))?
                + "\n")
                .as_bytes(),
        )?;
    }

    let mut rows = Vec::new();
    let mut artifacts = vec![
        ("labels.json", "tokenizer_meta", true),
        ("model.meta.json", "model_metadata", true),
        (model_name, model_kind, false),
        ("tokenizer.json", "tokenizer_meta", true),
    ];
    if safetensors {
        artifacts.insert(0, ("config.json", "model_metadata", true));
    }
    if tokenizer_source.is_some() {
        artifacts.push(("tokenizer_source.json", "model_metadata", true));
    }
    for (name, kind, text) in artifacts {
        let bytes = fs::read(out_dir.join(name)).map_err(|e| io(&e))?;
        let bytes = if text { normalized_text(&bytes) } else { bytes };
        rows.push(json!({"relative_path": name, "kind": kind, "digest": DigestSha256::of(&bytes).to_hex()}));
    }
    let digest_lines: String = rows
        .iter()
        .map(|r| format!("{}\n", r["digest"].as_str().unwrap_or_default()))
        .collect();
    let content_digest = DigestSha256::of(digest_lines.as_bytes()).to_hex();
    let short = input
        .meta
        .id
        .rsplit('/')
        .next()
        .unwrap_or("model")
        .to_ascii_lowercase();
    let pack_id = if safetensors {
        format!("hf-{short}-safetensors")
    } else {
        format!("hf-{short}-static{}", input.fixed_sequence_length)
    };
    let sbom_ref = format!("hf:{}@{revision}#{}", input.meta.id, input.onnx_file);
    let payload = pack_signing_payload(
        &pack_id,
        "1.0.0",
        1,
        &content_digest,
        &rights_uri,
        &sbom_ref,
    );
    let manifest = json!({
        "pack_id": pack_id,
        "version": "1.0.0",
        "pack_epoch": 1,
        "content_digest": content_digest,
        "artifacts": rows,
        "rights_uri": rights_uri,
        "sbom_ref": sbom_ref,
        "runtime_requirements": format!("{runtime_id};synthetic_only=true;fixed_sequence_length={}", input.fixed_sequence_length),
        "benchmark_links": ["specs/103-local-model-catalog/research.md"],
        "promotion_state": "candidate",
        "trust_root_id": SYNTHETIC_PACK_TRUST_ROOT_ID,
        "signature_hex": sign_pack_payload(&payload),
    });
    write(
        "pack.manifest.json",
        (serde_json::to_string_pretty(&manifest).map_err(|e| SnapshotError::Io(e.to_string()))?
            + "\n")
            .as_bytes(),
    )?;
    Ok(pack_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::ModelCatalog;

    /// Real Hub metadata for an OpenMed v3.0.0 catalog row (captured
    /// 2026-10-08, metadata only; weights not downloaded for this test).
    const DISEASE: &str = r#"{"id":"OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-33M-v1-onnx-android","sha":"54c9cc119d325ddad23300a1e9108bafa9643a06","lastModified":"2026-07-13T07:43:16.000Z","createdAt":"2026-07-09T19:02:22.000Z","gated":false,"private":false,"siblings":[{"rfilename":".gitattributes"},{"rfilename":"README.md"},{"rfilename":"config.json"},{"rfilename":"id2label.json"},{"rfilename":"model.onnx"},{"rfilename":"model.ort"},{"rfilename":"model.required_operators_and_types.config"},{"rfilename":"model_fp16.onnx"},{"rfilename":"model_int8.onnx"},{"rfilename":"openmed-onnx.json"},{"rfilename":"tokenizer.json"},{"rfilename":"tokenizer_config.json"}]}"#;
    const DISEASE_HASH: &str =
        "sha256:5f22790e0683fee106df70d08690575e75d905f5417588f0e2b46ed1e35614b0";
    const COMMIT: &str = "ea920f36fadd7b45935247d639f0ffa1ef493b23";

    fn meta(json: &str) -> HfRepoMetadata {
        serde_json::from_str(json).unwrap()
    }

    fn row(hash: &str, license: &str) -> CatalogRow {
        let line = format!(
            r#"{{"repo_id":"OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-33M-v1-onnx-android","family":"NER","task":"token-classification","formats":["onnx"],"license":"{license}","reproducibility_hash":"{hash}"}}"#
        );
        ModelCatalog::import("s/f", COMMIT, line.as_bytes())
            .unwrap()
            .get("OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-33M-v1-onnx-android")
            .unwrap()
            .clone()
    }

    #[test]
    fn port_reproduces_openmed_reproducibility_hash_for_a_real_row() {
        assert_eq!(reproducibility_hash(&meta(DISEASE)), DISEASE_HASH);
        verify_against_catalog(&meta(DISEASE), &row(DISEASE_HASH, "apache-2.0")).unwrap();
    }

    #[test]
    fn any_change_to_commit_or_file_list_breaks_the_binding() {
        let changed_sha = DISEASE.replace("54c9cc11", "54c9cc12");
        assert!(matches!(
            verify_against_catalog(&meta(&changed_sha), &row(DISEASE_HASH, "apache-2.0")),
            Err(SnapshotError::ReproducibilityMismatch { .. })
        ));
        let extra_file = DISEASE.replace(
            r#"{"rfilename":"tokenizer.json"}"#,
            r#"{"rfilename":"tokenizer.json"},{"rfilename":"evil.py"}"#,
        );
        assert!(
            verify_against_catalog(&meta(&extra_file), &row(DISEASE_HASH, "apache-2.0")).is_err()
        );
    }

    #[test]
    fn gated_private_and_wrong_repository_are_refused() {
        let gated = DISEASE.replace(r#""gated":false"#, r#""gated":"auto""#);
        assert_eq!(
            verify_against_catalog(&meta(&gated), &row(DISEASE_HASH, "apache-2.0")),
            Err(SnapshotError::Gated)
        );
        let private = DISEASE.replace(r#""private":false"#, r#""private":true"#);
        assert_eq!(
            verify_against_catalog(&meta(&private), &row(DISEASE_HASH, "apache-2.0")),
            Err(SnapshotError::Gated)
        );
        let other = DISEASE.replace("DiseaseDetect", "ChemicalDetect");
        assert!(matches!(
            verify_against_catalog(&meta(&other), &row(DISEASE_HASH, "apache-2.0")),
            Err(SnapshotError::RepositoryMismatch { .. })
        ));
    }

    #[test]
    fn file_verification_uses_lfs_sha256_or_git_blob_sha1() {
        let small = b"{\"a\":1}\n";
        let mut h = Sha1::new();
        h.update(format!("blob {}\0", small.len()).as_bytes());
        h.update(small);
        let blob = hex(&h.finalize());
        let big = vec![7u8; 1000];
        let big_sha = hex(&Sha256::digest(&big));
        let m = meta(&format!(
            r#"{{"id":"a/b","sha":"{0}","siblings":[{{"rfilename":"config.json","size":{1},"blobId":"{blob}"}},{{"rfilename":"model.onnx","size":1000,"lfs":{{"sha256":"{big_sha}","size":1000}}}},{{"rfilename":"weights.bin","lfs":{{"sha256":"{big_sha}"}}}}]}}"#,
            "0".repeat(40),
            small.len()
        ));
        assert!(verify_file(&m, "config.json", small).is_ok());
        assert_eq!(verify_file(&m, "model.onnx", &big).unwrap(), big_sha);
        assert!(matches!(
            verify_file(&m, "config.json", b"{\"a\":2}\n"),
            Err(SnapshotError::DigestMismatch { .. })
        ));
        assert!(matches!(
            verify_file(&m, "model.onnx", &[7u8; 999]),
            Err(SnapshotError::SizeMismatch { .. })
        ));
        assert!(matches!(
            verify_file(&m, "weights.bin", &big),
            Err(SnapshotError::RefusedArtifact(_))
        ));
        assert!(matches!(
            verify_file(&m, "../config.json", small),
            Err(SnapshotError::UnsafePath(_))
        ));
        assert!(matches!(
            verify_file(&m, "other.json", small),
            Err(SnapshotError::NotInSnapshot(_))
        ));
    }

    #[test]
    fn labels_are_ordered_by_index_and_validated() {
        let cfg = br#"{"id2label":{"1":"B-DISEASE","0":"O","2":"I-DISEASE"}}"#;
        assert_eq!(
            labels_from_config(cfg).unwrap(),
            ["O", "B-DISEASE", "I-DISEASE"]
        );
        assert!(labels_from_config(br#"{"id2label":{"0":"O","5":"X"}}"#).is_err());
        assert!(labels_from_config(br#"{}"#).is_err());
    }

    #[test]
    fn rights_pending_rows_are_not_built_into_packs() {
        let m = meta(DISEASE);
        let r = row(DISEASE_HASH, "other");
        let input = TokenClassifierSnapshot {
            meta: &m,
            row: &r,
            snapshot_dir: Path::new("."),
            onnx_file: "model.onnx",
            fixed_sequence_length: 128,
            tokenizer_source: None,
        };
        assert!(matches!(
            build_token_classifier_pack(&input, Path::new("unused")),
            Err(SnapshotError::RightsPending(_))
        ));
    }

    #[test]
    fn tokenizer_sources_are_plain_openmed_repositories_named_by_the_config() {
        let own = "OpenMed/OpenMed-PII-X-mlx";
        let src = |name: &str| {
            tokenizer_source_repo(format!(r#"{{"_name_or_path":"{name}"}}"#).as_bytes(), own)
        };
        assert_eq!(
            src("OpenMed/OpenMed-PII-X").as_deref(),
            Some("OpenMed/OpenMed-PII-X")
        );
        assert_eq!(src(own), None);
        assert_eq!(src("other/OpenMed-PII-X"), None);
        assert_eq!(src("OpenMed/../evil"), None);
        assert_eq!(src("OpenMed/a/b"), None);
        assert_eq!(src("OpenMed/"), None);
        assert_eq!(src("./local-dir"), None);
        assert_eq!(tokenizer_source_repo(b"{}", own), None);
        assert_eq!(tokenizer_source_repo(b"not json", own), None);
    }

    #[test]
    fn python_json_escaping_matches_ensure_ascii() {
        // CPython: json.dumps(input) for input = a, quote, b, backslash, U+00E9, U+1F600.
        let bs = char::from(92u8);
        let q = char::from(34u8);
        let input: String = [
            'a',
            q,
            'b',
            bs,
            char::from_u32(0xe9).unwrap(),
            char::from_u32(0x1F600).unwrap(),
        ]
        .iter()
        .collect();
        let mut s = String::new();
        py_json_string(&mut s, &input);
        let expected = format!("{q}a{bs}{q}b{bs}{bs}{bs}u00e9{bs}ud83d{bs}ude00{q}");
        assert_eq!(s, expected);
    }
}
