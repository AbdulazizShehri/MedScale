//! MedScale Audio Model Manager: source allowlist and local disk preflight.
//! Preflight is read-only. No model is fetched, installed, or activated here.
//! The pinned revisions must remain identical to medscale_pack::ASR_SOURCES.
use std::path::Path;

use serde_json::{Value, json};

use crate::host::{CmdError, CmdResult};

const MIB: u64 = 1024 * 1024;
const FREE_SPACE_RESERVE: u64 = 512 * MIB;

#[derive(Clone, Copy)]
struct Model {
    id: &'static str,
    name: &'static str,
    repository: &'static str,
    revision: &'static str,
    languages: &'static str,
    max_snapshot_bytes: u64,
}

const MODELS: [Model; 3] = [
    Model {
        id: "whisper-tiny-en",
        name: "Whisper Tiny · English",
        repository: "openai/whisper-tiny.en",
        revision: "87c7102498dcde7456f24cfd30239ca606ed9063",
        languages: "English",
        max_snapshot_bytes: 200 * MIB,
    },
    Model {
        id: "whisper-tiny-multilingual",
        name: "Whisper Tiny · Multilingual",
        repository: "openai/whisper-tiny",
        revision: "169d4a4341b33bc18d8881c4b69c2e104e1cc0af",
        languages: "Multilingual · English and Arabic",
        max_snapshot_bytes: 200 * MIB,
    },
    Model {
        id: "whisper-base-en",
        name: "Whisper Base · English",
        repository: "openai/whisper-base.en",
        revision: "911407f4214e0e1d82085af863093ec0b66f9cd6",
        languages: "English",
        max_snapshot_bytes: 384 * MIB,
    },
];

fn model(id: &str) -> CmdResult<Model> {
    MODELS
        .iter()
        .find(|entry| entry.id == id)
        .copied()
        .ok_or_else(|| CmdError::invalid("Invalid: unknown audio model ID"))
}

impl Model {
    fn required_bytes(self) -> u64 {
        // Snapshot and verified, signed Pack temporarily coexist during assembly.
        // Leave 512 MiB unused; these are conservative upper-bound budgets.
        self.max_snapshot_bytes * 2 + FREE_SPACE_RESERVE
    }
    fn directory(self, data_dir: &Path) -> std::path::PathBuf {
        data_dir.join("audio-models").join(self.id)
    }
    fn describe(self, data_dir: &Path) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "repository": self.repository,
            "revision": self.revision,
            "languages": self.languages,
            "download_budget_bytes": self.max_snapshot_bytes,
            "required_space_bytes": self.required_bytes(),
            "pack_files_present": self.directory(data_dir).join("pack.manifest.json").is_file(),
            "installed": false,
        })
    }
}

pub fn catalog(data_dir: &Path) -> Vec<Value> {
    MODELS.iter().map(|m| m.describe(data_dir)).collect()
}

pub fn preflight(id: &str, data_dir: &Path) -> CmdResult<Value> {
    let entry = model(id)?;
    // Data dir is Tauri-owned and resolved by startup. No arbitrary user path.
    let free_bytes = fs2::available_space(data_dir)
        .map_err(|_| CmdError::unavailable("Unavailable: cannot measure local free storage"))?;
    Ok(json!({
        "model": entry.describe(data_dir),
        "free_bytes": free_bytes,
        "required_space_bytes": entry.required_bytes(),
        "enough_space": free_bytes >= entry.required_bytes(),
        "download_started": false,
        "download_requires_explicit_consent": true,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_sources_are_the_admitted_whisper_pins() {
        for entry in MODELS {
            assert!(medscale_pack::ASR_SOURCES.contains(&(entry.repository, entry.revision)));
            assert_eq!(entry.revision.len(), 40);
            assert!(entry.required_bytes() > entry.max_snapshot_bytes);
        }
        assert_eq!(MODELS.len(), medscale_pack::ASR_SOURCES.len());
    }

    #[test]
    fn preflight_denies_unknown_models_before_filesystem_access() {
        assert!(model("../whisper").is_err());
        assert!(model("whisper-tiny-en/../fake").is_err());
        assert!(model("https://example.com/model").is_err());
    }

    #[test]
    fn catalog_is_read_only_and_does_not_create_model_folders() {
        let path = std::env::temp_dir()
            .join(format!("medscale-model-preflight-{}", std::process::id()))
            .join("unused");
        let entries = catalog(&path);
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().all(|entry| entry["installed"] == false));
        assert!(!path.exists());
    }
}

// Acquisition is deliberately separate from Core session setup: a worker
// downloads verified *data-only* files into an app-owned temporary folder,
// builds the existing signed Pack, then the UI asks the existing Core
// audio_admit_asr_pack command to activate it.
use std::fs::{self, File};
use std::io::{Read, Write};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use medscale_pack::HfRepoMetadata;
use serde::Serialize;
use url::Url;

use crate::host::Host;

#[derive(Clone, Default, Serialize)]
pub struct DownloadStatus {
    pub state: String,
    pub model_id: Option<String>,
    pub bytes_downloaded: u64,
    pub bytes_total: u64,
    pub pack_dir: Option<String>,
    pub message: Option<String>,
}

#[derive(Default)]
pub struct DownloadCoordinator {
    pub active: bool,
    pub cancel: Option<Arc<AtomicBool>>,
    pub status: DownloadStatus,
}

fn safe_url(value: &Url) -> Result<(), String> {
    let host = value.host_str().ok_or("missing download host")?;
    if value.scheme() != "https"
        || value.username() != ""
        || value.password().is_some()
        || value.port().is_some_and(|port| port != 443)
        || !(host == "huggingface.co"
            || host.ends_with(".huggingface.co")
            || host.ends_with(".hf.co"))
    {
        return Err("Download redirected to an unapproved server".into());
    }
    Ok(())
}

fn http_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .https_only(true)
        .max_redirects(0)
        .timeout_per_call(Some(Duration::from_secs(90)))
        .timeout_recv_body(Some(Duration::from_secs(180)))
        .user_agent("MedScale/0.1 local-audio-model-installer")
        .build()
        .new_agent()
}

/// Every byte is checked against a hard limit, regardless of Content-Length.
/// Redirect targets are constrained by the Hub's published HTTPS domains.
/// Accepts only caller-supplied, pinned paths; never an arbitrary UI URL.
fn fetch_bounded(
    agent: &ureq::Agent,
    url: &Url,
    limit: u64,
    cancel: &AtomicBool,
    mut on_bytes: impl FnMut(&[u8]) -> Result<(), String>,
) -> Result<u64, String> {
    let mut next = url.clone();
    for _ in 0..6 {
        safe_url(&next)?;
        if cancel.load(Ordering::Relaxed) {
            return Err("Download cancelled".into());
        }
        let mut response = agent
            .get(next.as_str())
            .call()
            .map_err(|_| "Network request failed; check connection and retry".to_owned())?;
        let code = response.status().as_u16();
        if matches!(code, 301 | 302 | 303 | 307 | 308) {
            let location = response
                .headers()
                .get("location")
                .and_then(|value| value.to_str().ok())
                .ok_or("Download redirect has no Location")?;
            next = next
                .join(location)
                .map_err(|_| "Download redirect is invalid".to_owned())?;
            continue;
        }
        if code != 200 {
            return Err(format!("Download HTTP status {code}"));
        }
        let mut reader = response.body_mut().as_reader();
        let mut buf = [0u8; 64 * 1024];
        let mut received = 0u64;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("Download cancelled".into());
            }
            let n = reader
                .read(&mut buf)
                .map_err(|_| "Download stream interrupted".to_owned())?;
            if n == 0 {
                break;
            }
            received += n as u64;
            if received > limit {
                return Err("Download exceeded the selected model's byte limit".into());
            }
            on_bytes(&buf[..n])?;
        }
        if received == 0 {
            return Err("Download returned an empty file".into());
        }
        return Ok(received);
    }
    Err("Too many download redirects".into())
}

fn set_progress(
    shared: &Arc<Mutex<DownloadCoordinator>>,
    state: &str,
    bytes_downloaded: u64,
    bytes_total: u64,
) {
    if let Ok(mut current) = shared.lock() {
        current.status.state = state.to_owned();
        current.status.bytes_downloaded = bytes_downloaded;
        current.status.bytes_total = bytes_total;
    }
}

fn acquire(
    entry: Model,
    data_dir: &Path,
    cancel: &AtomicBool,
    shared: &Arc<Mutex<DownloadCoordinator>>,
) -> Result<String, String> {
    let root = data_dir.join("audio-models");
    fs::create_dir_all(&root).map_err(|_| "Cannot create application model cache")?;
    let final_pack = entry.directory(data_dir);
    if final_pack.exists() {
        return Err(
            "This model directory already exists; use the existing Pack or inspect it first".into(),
        );
    }
    // Fixed, app-owned name with exclusive create; never overwrite a prior
    // attempt or remove anything that existed before this function started.
    let work = root.join(format!(".download-{}-{}", entry.id, std::process::id()));
    fs::create_dir(&work)
        .map_err(|_| "Existing temporary download; refusing to overwrite".to_owned())?;
    let run = || -> Result<String, String> {
        let snapshot = work.join("snapshot");
        let signed = work.join("pack");
        fs::create_dir(&snapshot).map_err(|_| "Cannot create temporary snapshot")?;
        fs::create_dir(&signed).map_err(|_| "Cannot create temporary Pack")?;

        let agent = http_agent();
        // Use a fixed Hub API endpoint rather than deriving paths from redirects.
        let metadata_url = Url::parse(&format!(
            "https://huggingface.co/api/models/{}/revision/{}?blobs=true",
            entry.repository, entry.revision,
        ))
        .map_err(|_| "Invalid pinned metadata endpoint")?;
        let mut metadata_bytes = Vec::new();
        fetch_bounded(&agent, &metadata_url, 3 * MIB, cancel, |chunk| {
            metadata_bytes.extend_from_slice(chunk);
            Ok(())
        })?;
        let meta: HfRepoMetadata = serde_json::from_slice(&metadata_bytes)
            .map_err(|_| "Model metadata is not valid JSON")?;
        if meta.id != entry.repository
            || meta.sha != entry.revision
            || meta.private
            || !matches!(
                meta.gated,
                serde_json::Value::Null | serde_json::Value::Bool(false)
            )
        {
            return Err(
                "Hugging Face model metadata did not match the approved pinned revision".into(),
            );
        }
        let names = ["config.json", "tokenizer.json", "model.safetensors"];
        let mut total = 0u64;
        for name in names {
            let sibling = meta
                .siblings
                .iter()
                .find(|row| row.rfilename == name)
                .ok_or("Pinned Hub snapshot is missing a required model file")?;
            let size = sibling
                .size
                .or_else(|| sibling.lfs.as_ref().and_then(|l| l.size));
            let limit = if name == "model.safetensors" {
                entry.max_snapshot_bytes
            } else {
                16 * MIB
            };
            if size.is_some_and(|s| s == 0 || s > limit) {
                return Err("Pinned model file exceeds the allowed download size".into());
            }
            total = total.saturating_add(size.unwrap_or(limit));
        }
        if total > entry.max_snapshot_bytes {
            return Err("Pinned snapshot exceeds the declared download budget".into());
        }
        let remaining = fs2::available_space(data_dir)
            .map_err(|_| "Cannot check storage before downloading")?;
        if remaining < entry.required_bytes() {
            return Err(
                "Not enough free disk space for the verified model and temporary Pack".into(),
            );
        }

        let mut downloaded = 0u64;
        set_progress(shared, "downloading", downloaded, total);
        for name in names {
            let limit = if name == "model.safetensors" {
                entry.max_snapshot_bytes
            } else {
                16 * MIB
            };
            let source = Url::parse(&format!(
                "https://huggingface.co/{}/resolve/{}/{}",
                entry.repository, entry.revision, name
            ))
            .map_err(|_| "Invalid approved download URL")?;
            let mut output =
                File::create(snapshot.join(name)).map_err(|_| "Cannot write the model snapshot")?;
            fetch_bounded(&agent, &source, limit, cancel, |chunk| {
                output
                    .write_all(chunk)
                    .map_err(|_| "Cannot write model download")?;
                downloaded += chunk.len() as u64;
                set_progress(shared, "downloading", downloaded, total);
                Ok(())
            })?;
            output
                .sync_all()
                .map_err(|_| "Cannot flush verified snapshot to disk")?;
        }
        if cancel.load(Ordering::Relaxed) {
            return Err("Download cancelled".into());
        }
        set_progress(shared, "verifying", downloaded, total);
        // This verifies the three files against the immutable Hub metadata
        // (LFS SHA-256 or Git blob hash) before signing the Pack. No weights
        // are loaded as executable code.
        medscale_pack::build_asr_pack(&meta, &snapshot, &signed)
            .map_err(|_| "Downloaded files failed pinned hash or Pack verification")?;
        if cancel.load(Ordering::Relaxed) {
            return Err("Download cancelled".into());
        }
        if final_pack.exists() {
            return Err("Destination appeared during download; refusing to overwrite".into());
        }
        fs::rename(&signed, &final_pack)
            .map_err(|_| "Could not publish verified Pack atomically")?;
        Ok(final_pack.to_string_lossy().to_string())
    };
    let result = run();
    // Only remove our own directory, created exclusively above. Do not
    // remove final Pack on a cancellation occurring after atomic publication.
    let _ = fs::remove_dir_all(&work);
    result
}

pub fn start(host: &Host, model_id: String, consent: bool) -> CmdResult<Value> {
    if !consent {
        return Err(CmdError::new(
            "denied",
            "Denied: model download needs explicit consent",
        ));
    }
    let entry = model(&model_id)?;
    let data_dir = host
        .state
        .lock()
        .map_err(|_| CmdError::unavailable("Unavailable: desktop state"))?
        .data_dir()?
        .to_path_buf();
    let status = preflight(&model_id, &data_dir)?;
    if status["enough_space"] != true {
        return Err(CmdError::invalid(
            "Invalid: insufficient local storage for model staging",
        ));
    }
    let mut manager = host
        .model_download
        .lock()
        .map_err(|_| CmdError::unavailable("Unavailable: model download state"))?;
    if manager.active {
        return Err(CmdError::new(
            "conflict",
            "Conflict: another model download is active",
        ));
    }
    let cancel = Arc::new(AtomicBool::new(false));
    manager.active = true;
    manager.cancel = Some(Arc::clone(&cancel));
    manager.status = DownloadStatus {
        state: "downloading".into(),
        model_id: Some(model_id),
        bytes_total: entry.max_snapshot_bytes,
        ..Default::default()
    };
    let downloads = Arc::clone(&host.model_download);
    std::thread::spawn(move || {
        let result = acquire(entry, &data_dir, &cancel, &downloads);
        if let Ok(mut state) = downloads.lock() {
            state.active = false;
            state.cancel = None;
            match result {
                Ok(path) => {
                    state.status.state = "ready".into();
                    state.status.pack_dir = Some(path);
                    state.status.message = None;
                }
                Err(error) => {
                    state.status.state = if cancel.load(Ordering::Relaxed) {
                        "cancelled".into()
                    } else {
                        "error".into()
                    };
                    state.status.message = Some(error);
                }
            }
        }
    });
    Ok(json!({"started": true, "model_id": entry.id}))
}

pub fn status(host: &Host) -> CmdResult<Value> {
    let manager = host
        .model_download
        .lock()
        .map_err(|_| CmdError::unavailable("Unavailable: model download state"))?;
    Ok(json!(manager.status))
}

pub fn cancel(host: &Host) -> CmdResult<Value> {
    let manager = host
        .model_download
        .lock()
        .map_err(|_| CmdError::unavailable("Unavailable: model download state"))?;
    if !manager.active {
        return Err(CmdError::new(
            "conflict",
            "Conflict: no model download is active",
        ));
    }
    if let Some(cancel) = &manager.cancel {
        cancel.store(true, Ordering::Relaxed);
    }
    Ok(json!({"cancel_requested": true}))
}

#[cfg(test)]
mod acquisition_tests {
    use super::*;

    #[test]
    fn downloader_refuses_non_hub_or_non_https_redirects() {
        for raw in [
            "http://huggingface.co/model",
            "https://huggingface.co.evil.test/weights",
            "https://localhost/secret",
            "https://127.0.0.1/model",
            "https://user@huggingface.co/file",
            "https://example.org/data",
            "https://huggingface.co:8080/file",
        ] {
            assert!(safe_url(&Url::parse(raw).unwrap()).is_err(), "{raw}");
        }
        for raw in [
            "https://huggingface.co/openai/whisper-tiny.en",
            "https://cdn-lfs-us-1.hf.co/weight",
            "https://cas-bridge.xethub.hf.co/data",
        ] {
            assert!(safe_url(&Url::parse(raw).unwrap()).is_ok(), "{raw}");
        }
    }

    #[test]
    fn cancellation_never_makes_network_request() {
        let flag = AtomicBool::new(true);
        let agent = http_agent();
        let result = fetch_bounded(
            &agent,
            &Url::parse("https://huggingface.co/openai/whisper-tiny.en").unwrap(),
            1024,
            &flag,
            |_| panic!("unexpected downloaded bytes"),
        );
        assert_eq!(result.unwrap_err(), "Download cancelled");
    }
}
