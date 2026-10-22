//! Synthetic MESC release-dir verifier (Spec 036 READY_BASE).
//!
//! Verifies byte identity of a local synthetic release against `manifest.json`.
//! Never executes model bytes. Never clears `MESC_RELEASED_ARTIFACT`.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use medscale_contracts::external_artifact::{
    MescAdmissionState, MescReleaseManifestV0, MescVerifyReason, MescVerifyReport,
};
use medscale_contracts::objects::DigestSha256;
use thiserror::Error;

/// Verifier errors mapped to stable reason codes.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("mesc verify rejected: {}", .0.reason.as_str())]
pub struct MescVerifyError(MescVerifyReport);

impl MescVerifyError {
    #[must_use]
    pub fn report(&self) -> &MescVerifyReport {
        &self.0
    }
}

fn reject(reason: MescVerifyReason, detail: impl Into<String>) -> MescVerifyError {
    MescVerifyError(MescVerifyReport::rejected(reason, detail))
}

fn parse_hex_digest(hex: &str) -> Result<DigestSha256, MescVerifyError> {
    let hex = hex.trim();
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(reject(
            MescVerifyReason::MalformedManifest,
            "digest must be 64 hex chars",
        ));
    }
    let mut bytes = [0_u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk)
            .map_err(|_| reject(MescVerifyReason::MalformedManifest, "digest utf8"))?;
        bytes[i] = u8::from_str_radix(s, 16)
            .map_err(|_| reject(MescVerifyReason::MalformedManifest, "digest hex"))?;
    }
    Ok(DigestSha256::from_bytes(bytes))
}

fn require_nonempty(field: &str, value: &str) -> Result<(), MescVerifyError> {
    if value.trim().is_empty() {
        return Err(reject(
            MescVerifyReason::MissingField,
            format!("missing or empty field: {field}"),
        ));
    }
    Ok(())
}

/// Verify a local directory containing `manifest.json` and referenced files.
///
/// On success returns `Verified` with `product_admit_authorized=false`.
pub fn verify_mesc_release_dir(release_dir: &Path) -> Result<MescVerifyReport, MescVerifyError> {
    let manifest_path = release_dir.join("manifest.json");
    if !manifest_path.is_file() {
        return Err(reject(
            MescVerifyReason::MissingManifest,
            "manifest.json not found",
        ));
    }
    let bytes = fs::read(&manifest_path).map_err(|e| {
        reject(
            MescVerifyReason::MalformedManifest,
            format!("read manifest: {e}"),
        )
    })?;
    let manifest: MescReleaseManifestV0 = serde_json::from_slice(&bytes).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unknown field") {
            reject(MescVerifyReason::UnsupportedSchema, msg)
        } else {
            reject(MescVerifyReason::MalformedManifest, msg)
        }
    })?;

    if manifest.schema_version != MescReleaseManifestV0::SCHEMA_VERSION {
        return Err(reject(
            MescVerifyReason::UnsupportedSchema,
            format!(
                "schema_version {} != {}",
                manifest.schema_version,
                MescReleaseManifestV0::SCHEMA_VERSION
            ),
        ));
    }

    require_nonempty("producer_id", &manifest.producer_id)?;
    require_nonempty("release_id", &manifest.release_id)?;
    require_nonempty("release_tag", &manifest.release_tag)?;
    require_nonempty("source_commit", &manifest.source_commit)?;
    require_nonempty("source_tree", &manifest.source_tree)?;
    require_nonempty("model_id", &manifest.model_id)?;
    require_nonempty("tokenizer_id", &manifest.tokenizer_id)?;
    require_nonempty("base_model_id", &manifest.base_model_id)?;
    require_nonempty("corpus_id", &manifest.corpus_id)?;
    require_nonempty(
        "training_receipt_digest_hex",
        &manifest.training_receipt_digest_hex,
    )?;
    require_nonempty(
        "evaluation_receipt_digest_hex",
        &manifest.evaluation_receipt_digest_hex,
    )?;
    require_nonempty("sbom_path", &manifest.sbom_path)?;
    require_nonempty("sbom_digest_hex", &manifest.sbom_digest_hex)?;
    require_nonempty("rights_license", &manifest.rights_license)?;
    require_nonempty("rights_notice_path", &manifest.rights_notice_path)?;
    require_nonempty("runtime_requirements", &manifest.runtime_requirements)?;

    parse_hex_digest(&manifest.training_receipt_digest_hex).map_err(|_| {
        reject(
            MescVerifyReason::MissingTrainingReceipt,
            "training_receipt_digest_hex invalid",
        )
    })?;
    parse_hex_digest(&manifest.evaluation_receipt_digest_hex).map_err(|_| {
        reject(
            MescVerifyReason::MissingEvaluation,
            "evaluation_receipt_digest_hex invalid",
        )
    })?;
    let sbom_digest = parse_hex_digest(&manifest.sbom_digest_hex)
        .map_err(|_| reject(MescVerifyReason::MissingSbom, "sbom_digest_hex invalid"))?;

    let notice_path = release_dir.join(&manifest.rights_notice_path);
    if !notice_path.is_file() {
        return Err(reject(
            MescVerifyReason::MissingRights,
            format!("rights notice missing: {}", manifest.rights_notice_path),
        ));
    }

    let sbom_path = release_dir.join(&manifest.sbom_path);
    if !sbom_path.is_file() {
        return Err(reject(
            MescVerifyReason::MissingSbom,
            format!("sbom missing: {}", manifest.sbom_path),
        ));
    }
    let sbom_bytes = fs::read(&sbom_path)
        .map_err(|e| reject(MescVerifyReason::MissingSbom, format!("sbom read: {e}")))?;
    if DigestSha256::of(&sbom_bytes) != sbom_digest {
        return Err(reject(
            MescVerifyReason::DigestMismatch,
            "sbom digest mismatch",
        ));
    }

    if manifest.artifacts.is_empty() {
        return Err(reject(MescVerifyReason::MissingField, "artifacts[] empty"));
    }

    let mut seen = HashSet::new();
    for art in &manifest.artifacts {
        require_nonempty("artifacts[].kind", &art.kind)?;
        require_nonempty("artifacts[].path", &art.path)?;
        if !seen.insert(art.path.clone()) {
            return Err(reject(
                MescVerifyReason::DuplicateArtifactPath,
                format!("duplicate path: {}", art.path),
            ));
        }
        if art.path.contains("..") || Path::new(&art.path).is_absolute() {
            return Err(reject(
                MescVerifyReason::MalformedManifest,
                format!("artifact path must be relative without ..: {}", art.path),
            ));
        }
        let expected = parse_hex_digest(&art.sha256_hex)?;
        let file_path = release_dir.join(&art.path);
        if !file_path.is_file() {
            return Err(reject(
                MescVerifyReason::MissingArtifactFile,
                format!("missing file: {}", art.path),
            ));
        }
        let data = fs::read(&file_path).map_err(|e| {
            reject(
                MescVerifyReason::MissingArtifactFile,
                format!("read {}: {e}", art.path),
            )
        })?;
        if data.len() as u64 != art.byte_length {
            return Err(reject(
                MescVerifyReason::SizeMismatch,
                format!(
                    "{}: expected {} bytes, got {}",
                    art.path,
                    art.byte_length,
                    data.len()
                ),
            ));
        }
        if DigestSha256::of(&data) != expected {
            return Err(reject(
                MescVerifyReason::DigestMismatch,
                format!("{} digest mismatch", art.path),
            ));
        }
    }

    let report = MescVerifyReport::verified_synthetic(
        manifest.producer_id.clone(),
        manifest.release_id.clone(),
        manifest.epoch,
    );
    debug_assert_eq!(report.state, MescAdmissionState::Verified);
    debug_assert!(!report.product_admit_authorized);
    Ok(report)
}

/// In-memory producer→epoch anti-rollback for synthetic verifies (Spec 036).
#[derive(Debug, Default)]
pub struct MescEpochStore {
    last: std::collections::HashMap<String, u64>,
}

impl MescEpochStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a verified epoch; reject lower or equal epochs for the same producer.
    pub fn admit_epoch(&mut self, producer_id: &str, epoch: u64) -> Result<(), MescVerifyError> {
        match self.last.get(producer_id) {
            Some(&prev) if epoch < prev => Err(reject(
                MescVerifyReason::AntiRollback,
                format!("epoch {epoch} < recorded {prev} for {producer_id}"),
            )),
            Some(&prev) if epoch == prev => Err(reject(
                MescVerifyReason::ReplayRejected,
                format!("exact epoch {epoch} already recorded for {producer_id}"),
            )),
            _ => {
                self.last.insert(producer_id.to_owned(), epoch);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_good_fixture(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        let model = b"synthetic-model-bytes-v1";
        let sbom = br#"{"bomFormat":"CycloneDX","specVersion":"1.5","components":[]}"#;
        let notice = b"Synthetic NOTICE - no redistribution rights claimed.\n";
        fs::write(dir.join("model.bin"), model).unwrap();
        fs::write(dir.join("sbom.json"), sbom).unwrap();
        fs::write(dir.join("NOTICE"), notice).unwrap();
        let manifest = serde_json::json!({
            "schema_version": 1,
            "producer_id": "synthetic.medscale.mesc",
            "release_id": "syn-036-001",
            "release_tag": "syn-v0.0.1",
            "source_commit": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "source_tree": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "model_id": "syn-model",
            "tokenizer_id": "syn-tok",
            "base_model_id": "syn-base",
            "corpus_id": "syn-corpus",
            "training_receipt_digest_hex": DigestSha256::of(b"train-receipt").to_hex(),
            "evaluation_receipt_digest_hex": DigestSha256::of(b"eval-receipt").to_hex(),
            "sbom_path": "sbom.json",
            "sbom_digest_hex": DigestSha256::of(sbom).to_hex(),
            "rights_license": "SYNTHETIC-NO-RIGHTS",
            "rights_notice_path": "NOTICE",
            "provenance_note": "synthetic fixture for Spec 036",
            "limitations": ["not a real MESC release"],
            "runtime_requirements": "offline-fixture-only",
            "epoch": 1,
            "artifacts": [
                {
                    "kind": "model_weights",
                    "path": "model.bin",
                    "byte_length": model.len() as u64,
                    "sha256_hex": DigestSha256::of(model).to_hex()
                },
                {
                    "kind": "sbom",
                    "path": "sbom.json",
                    "byte_length": sbom.len() as u64,
                    "sha256_hex": DigestSha256::of(sbom).to_hex()
                },
                {
                    "kind": "notice",
                    "path": "NOTICE",
                    "byte_length": notice.len() as u64,
                    "sha256_hex": DigestSha256::of(notice).to_hex()
                }
            ]
        });
        let mut f = fs::File::create(dir.join("manifest.json")).unwrap();
        f.write_all(serde_json::to_vec_pretty(&manifest).unwrap().as_slice())
            .unwrap();
    }

    #[test]
    fn good_fixture_verifies_without_product_admit() {
        let dir = std::env::temp_dir().join(format!("mesc-036-good-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        write_good_fixture(&dir);
        let report = verify_mesc_release_dir(&dir).expect("verify");
        assert_eq!(report.state, MescAdmissionState::Verified);
        assert!(!report.product_admit_authorized);
        assert_eq!(report.reason, MescVerifyReason::Ok);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn digest_mismatch_rejects() {
        let dir = std::env::temp_dir().join(format!("mesc-036-bad-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        write_good_fixture(&dir);
        fs::write(
            dir.join("model.bin"),
            b"X".repeat(b"synthetic-model-bytes-v1".len()),
        )
        .unwrap();
        let err = verify_mesc_release_dir(&dir).unwrap_err();
        assert_eq!(err.report().reason, MescVerifyReason::DigestMismatch);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn anti_rollback_and_replay() {
        let mut store = MescEpochStore::new();
        store.admit_epoch("p", 2).unwrap();
        assert_eq!(
            store.admit_epoch("p", 1).unwrap_err().report().reason,
            MescVerifyReason::AntiRollback
        );
        assert_eq!(
            store.admit_epoch("p", 2).unwrap_err().report().reason,
            MescVerifyReason::ReplayRejected
        );
        store.admit_epoch("p", 3).unwrap();
    }
}
