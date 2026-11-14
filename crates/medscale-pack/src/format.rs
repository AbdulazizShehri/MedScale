//! Format gates and local-path admission (Spec 008 + Spec 026 signer).

use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use medscale_contracts::objects::{DigestSha256, OpaqueId};
use medscale_contracts::packs::{
    PackAdmitReason, PackArtifactEntry, PackArtifactKind, PackManifestV0, PackPromotionState,
};
use medscale_keys::{
    PackTrustError, SYNTHETIC_PACK_TRUST_ROOT_ID, pack_signing_payload, verify_pack_signature,
};
use serde::Deserialize;
use thiserror::Error;

/// Admission errors before store insert.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AdmitError {
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("missing rights_uri")]
    MissingRights,
    #[error("forbidden artifact kind")]
    ForbiddenKind,
    #[error("digest mismatch")]
    DigestMismatch,
    #[error("missing signature")]
    MissingSignature,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("unknown trust root")]
    UnknownTrustRoot,
    #[error("anti-rollback")]
    AntiRollback,
    #[error("io: {0}")]
    Io(String),
}

impl AdmitError {
    #[must_use]
    pub fn reason(&self) -> PackAdmitReason {
        match self {
            Self::MissingRights => PackAdmitReason::MissingRights,
            Self::ForbiddenKind => PackAdmitReason::ForbiddenArtifactKind,
            Self::DigestMismatch => PackAdmitReason::DigestMismatch,
            Self::MissingSignature => PackAdmitReason::MissingSignature,
            Self::InvalidSignature => PackAdmitReason::InvalidSignature,
            Self::UnknownTrustRoot => PackAdmitReason::UnknownTrustRoot,
            Self::AntiRollback => PackAdmitReason::AntiRollback,
            Self::InvalidManifest(_) | Self::Io(_) => PackAdmitReason::InvalidManifest,
        }
    }
}

/// Returns deny reason if kind is forbidden by Spec 008 default policy.
#[must_use]
pub fn forbidden_reason(kind: PackArtifactKind) -> Option<PackAdmitReason> {
    kind.is_forbidden_by_default()
        .then_some(PackAdmitReason::ForbiddenArtifactKind)
}

const MAX_MANIFEST_BYTES: u64 = 1_048_576;

pub(crate) const fn artifact_size_limit(kind: PackArtifactKind) -> u64 {
    match kind {
        PackArtifactKind::OnnxModel | PackArtifactKind::SafetensorsModel => 1_073_741_824,
        PackArtifactKind::TokenizerMeta | PackArtifactKind::FixtureBytes => 67_108_864,
        PackArtifactKind::ModelMetadata => 1_048_576,
        PackArtifactKind::Pickle | PackArtifactKind::CodeBin | PackArtifactKind::OnnxCustomOp => 0,
    }
}

fn read_bounded_artifact(path: &Path, kind: PackArtifactKind) -> Result<Vec<u8>, AdmitError> {
    let limit = artifact_size_limit(kind);
    let metadata = fs::metadata(path).map_err(|err| AdmitError::Io(err.to_string()))?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(AdmitError::InvalidManifest(
            "artifact exceeds admitted byte bound".into(),
        ));
    }
    let file = fs::File::open(path).map_err(|err| AdmitError::Io(err.to_string()))?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| AdmitError::Io(err.to_string()))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
        return Err(AdmitError::InvalidManifest(
            "artifact exceeds admitted byte bound".into(),
        ));
    }
    Ok(bytes)
}

fn parse_hex_digest(hex: &str) -> Result<DigestSha256, AdmitError> {
    let hex = hex.trim();
    if hex.len() != 64 {
        return Err(AdmitError::InvalidManifest(
            "digest must be 64 hex chars".into(),
        ));
    }
    let mut bytes = [0_u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk)
            .map_err(|_| AdmitError::InvalidManifest("digest utf8".into()))?;
        bytes[i] = u8::from_str_radix(s, 16)
            .map_err(|_| AdmitError::InvalidManifest("digest hex".into()))?;
    }
    Ok(DigestSha256::from_bytes(bytes))
}

fn safe_artifact_path(root: &Path, relative: &str) -> Result<PathBuf, AdmitError> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(AdmitError::InvalidManifest(
            "artifact path escapes pack root".into(),
        ));
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|err| AdmitError::Io(err.to_string()))?;
    let candidate = root.join(relative);
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|err| AdmitError::Io(err.to_string()))?;
    if !canonical_candidate.starts_with(&canonical_root) {
        return Err(AdmitError::InvalidManifest(
            "artifact path escapes pack root".into(),
        ));
    }
    Ok(canonical_candidate)
}

fn normalize_lf(bytes: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                out.push(b'\n');
                i += 2;
                continue;
            }
            out.push(b'\n');
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

#[derive(Debug, Deserialize)]
struct WireArtifact {
    relative_path: String,
    kind: PackArtifactKind,
    digest: String,
}

#[derive(Debug, Deserialize)]
struct WireManifest {
    pack_id: String,
    version: String,
    pack_epoch: u64,
    content_digest: String,
    artifacts: Vec<WireArtifact>,
    rights_uri: String,
    sbom_ref: String,
    runtime_requirements: String,
    #[serde(default)]
    benchmark_links: Vec<String>,
    promotion_state: PackPromotionState,
    trust_root_id: String,
    signature_hex: String,
}

/// Load and validate a pack directory containing `pack.manifest.json` + artifacts.
pub fn admit_pack_dir(path: &Path) -> Result<PackManifestV0, AdmitError> {
    let manifest_path = safe_artifact_path(path, "pack.manifest.json")?;
    let metadata = fs::metadata(&manifest_path).map_err(|err| AdmitError::Io(err.to_string()))?;
    if !metadata.is_file() || metadata.len() > MAX_MANIFEST_BYTES {
        return Err(AdmitError::InvalidManifest(
            "pack manifest exceeds admitted byte bound".into(),
        ));
    }
    let file = fs::File::open(&manifest_path).map_err(|err| AdmitError::Io(err.to_string()))?;
    let mut raw = Vec::new();
    file.take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|err| AdmitError::Io(err.to_string()))?;
    if u64::try_from(raw.len()).unwrap_or(u64::MAX) > MAX_MANIFEST_BYTES {
        return Err(AdmitError::InvalidManifest(
            "pack manifest exceeds admitted byte bound".into(),
        ));
    }
    let wire: WireManifest =
        serde_json::from_slice(&raw).map_err(|err| AdmitError::InvalidManifest(err.to_string()))?;

    if wire.rights_uri.trim().is_empty() {
        return Err(AdmitError::MissingRights);
    }
    if wire.sbom_ref.trim().is_empty() {
        return Err(AdmitError::InvalidManifest("empty sbom_ref".into()));
    }
    if wire.signature_hex.trim().is_empty() {
        return Err(AdmitError::MissingSignature);
    }
    if wire.trust_root_id.trim().is_empty() {
        return Err(AdmitError::UnknownTrustRoot);
    }

    let mut artifacts = Vec::new();
    for art in &wire.artifacts {
        if forbidden_reason(art.kind).is_some() {
            return Err(AdmitError::ForbiddenKind);
        }
        let file = safe_artifact_path(path, &art.relative_path)?;
        let mut bytes = read_bounded_artifact(&file, art.kind)?;
        // Text fixture kinds are LF-canonical (Windows checkouts must not change digests).
        if matches!(
            art.kind,
            PackArtifactKind::FixtureBytes
                | PackArtifactKind::TokenizerMeta
                | PackArtifactKind::ModelMetadata
        ) {
            bytes = normalize_lf(bytes);
        }
        let dig = DigestSha256::of(&bytes);
        let expected = parse_hex_digest(&art.digest)?;
        if dig != expected {
            return Err(AdmitError::DigestMismatch);
        }
        artifacts.push(PackArtifactEntry {
            relative_path: art.relative_path.clone(),
            kind: art.kind,
            digest: dig,
        });
    }

    let mut arts = artifacts.clone();
    arts.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    let mut concat = String::new();
    for a in &arts {
        concat.push_str(&a.digest.to_hex());
        concat.push('\n');
    }
    let expected_content = DigestSha256::of(concat.as_bytes());
    let declared = parse_hex_digest(&wire.content_digest)?;
    if expected_content != declared {
        return Err(AdmitError::DigestMismatch);
    }

    let payload = pack_signing_payload(
        &wire.pack_id,
        &wire.version,
        wire.pack_epoch,
        &wire.content_digest,
        &wire.rights_uri,
        &wire.sbom_ref,
    );
    match verify_pack_signature(&wire.trust_root_id, &payload, &wire.signature_hex) {
        Ok(()) => {}
        Err(PackTrustError::UnknownTrustRoot) => return Err(AdmitError::UnknownTrustRoot),
        Err(PackTrustError::InvalidEncoding | PackTrustError::VerifyFailed) => {
            return Err(AdmitError::InvalidSignature);
        }
    }
    // Keep synthetic root id constant visible for audits.
    if wire.trust_root_id != SYNTHETIC_PACK_TRUST_ROOT_ID {
        return Err(AdmitError::UnknownTrustRoot);
    }

    let mut promotion_state = wire.promotion_state;
    if !matches!(
        promotion_state,
        PackPromotionState::Candidate | PackPromotionState::LastGreen
    ) {
        promotion_state = PackPromotionState::Candidate;
    }

    Ok(PackManifestV0 {
        pack_id: OpaqueId::new(wire.pack_id),
        version: wire.version,
        pack_epoch: wire.pack_epoch,
        content_digest: declared,
        artifacts,
        rights_uri: wire.rights_uri,
        sbom_ref: wire.sbom_ref,
        runtime_requirements: wire.runtime_requirements,
        benchmark_links: wire.benchmark_links,
        promotion_state,
        trust_root_id: wire.trust_root_id,
        signature_hex: wire.signature_hex,
    })
}
