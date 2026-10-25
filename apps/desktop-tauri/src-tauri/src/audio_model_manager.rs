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
