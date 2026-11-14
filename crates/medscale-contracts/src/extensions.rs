//! Community Extensions contracts (Spec 087, foundation slice).
//!
//! An extension in this slice is **declarative**: a signed manifest that
//! names commands, each bound to one operation of a closed, versioned,
//! read-only Host API. No extension code is loaded or executed — no
//! native library, script, WASM, shell or interpreter. Executable
//! extensions need a qualified runtime (WASM or an isolated worker for
//! untrusted code) and a separate dependency and security admission.
//!
//! Trust is explicit and local: a publisher key is trusted only when the
//! user adds it; there is no built-in trust root and no registry that
//! confers trust. Every install, upgrade, rollback, enablement, grant,
//! revocation and invocation goes through Core and leaves a receipt.
//!
//! ```text
//! pack bytes != verified release != install (per Project)
//!   != grant (per capability, with a data-class ceiling)
//!   != invocation (Core runs the mapped Host API operation)
//!   != result (read-only; no external effect)
//! ```

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::objects::{DigestSha256, ObjectHeader, OpaqueId};
use crate::privacy_gate::DataClass;

/// Durable schema version for every extension object (Spec 087 v1).
pub const EXTENSION_SCHEMA_VERSION: u32 = 1;
/// The Host API this build offers. Manifests declare `[min, max]`.
pub const HOST_API_VERSION: u32 = 1;
/// Domain separator for release signatures.
pub const EXTENSION_SIGN_DOMAIN: &str = "MEDSCALE_EXTENSION_SIGN_V1";

pub const EXTENSION_ID_MAX_CHARS: usize = 64;
pub const TITLE_MAX_CHARS: usize = 128;
pub const DESCRIPTION_MAX_CHARS: usize = 1024;
pub const LICENSE_MAX_CHARS: usize = 64;
pub const COMMANDS_MAX: usize = 32;
pub const PACK_BYTES_MAX: u64 = 256 * 1024;
pub const ROWS_MAX: u32 = 1000;
pub const PUBLIC_KEY_HEX_LEN: usize = 64;
pub const SIGNATURE_HEX_LEN: usize = 128;

macro_rules! closed_vocabulary {
    ($name:ident, $what:literal, { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            pub fn parse(value: &str) -> Result<Self, String> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    other => Err(format!(concat!("unknown ", $what, " {}"), other)),
                }
            }
        }
    };
}

/// What an extension may ask for. Closed: an unknown capability fails the
/// manifest parse, so the pack is refused (fail closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionCapability {
    /// Project name, status and counts.
    ProjectRead,
    /// A snapshot's typed schema and row count.
    SnapshotSchemaRead,
    /// A bounded page of a snapshot's rows.
    SnapshotRowsRead,
}

closed_vocabulary!(ExtensionCapability, "extension capability", {
    ProjectRead => "project_read",
    SnapshotSchemaRead => "snapshot_schema_read",
    SnapshotRowsRead => "snapshot_rows_read",
});

/// How an extension runs. Only `declarative` exists in Host API v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntrypointKind {
    Declarative,
}

/// The Host API v1 operations a command may name. All are read-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostOperation {
    ProjectSummary,
    SnapshotSchema,
    SnapshotRows,
}

closed_vocabulary!(HostOperation, "host operation", {
    ProjectSummary => "project_summary",
    SnapshotSchema => "snapshot_schema",
    SnapshotRows => "snapshot_rows",
});

impl HostOperation {
    #[must_use]
    pub const fn capability(self) -> ExtensionCapability {
        match self {
            Self::ProjectSummary => ExtensionCapability::ProjectRead,
            Self::SnapshotSchema => ExtensionCapability::SnapshotSchemaRead,
            Self::SnapshotRows => ExtensionCapability::SnapshotRowsRead,
        }
    }

    /// Whether the operation reads a snapshot (and so needs a target).
    #[must_use]
    pub const fn reads_snapshot(self) -> bool {
        matches!(self, Self::SnapshotSchema | Self::SnapshotRows)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ExtensionVersion {
    #[must_use]
    pub fn render(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// One declarative command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionCommand {
    /// Plain identifier, unique within the manifest.
    pub name: String,
    pub title: String,
    pub operation: HostOperation,
    /// Row page size for `snapshot_rows` (1..=`ROWS_MAX`); absent otherwise.
    pub max_rows: Option<u32>,
}

/// The signed description of one extension release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionManifest {
    /// Plain, lowercase identifier (`a-z`, `0-9`, `.`, `-`).
    pub extension_id: String,
    pub version: ExtensionVersion,
    pub publisher_id: String,
    /// The publisher's ed25519 public key (lowercase hex).
    pub publisher_key_hex: String,
    pub host_api_min: u32,
    pub host_api_max: u32,
    /// SPDX license expression as declared (not verified).
    pub license: String,
    pub description: String,
    pub entrypoint: EntrypointKind,
    /// Sorted, unique.
    pub capabilities: Vec<ExtensionCapability>,
    pub commands: Vec<ExtensionCommand>,
}

fn plain_id(value: &str, what: &str) -> Result<(), String> {
    let ok = !value.is_empty()
        && value.chars().count() <= EXTENSION_ID_MAX_CHARS
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
        && value.starts_with(|c: char| c.is_ascii_lowercase())
        && !value.contains("..");
    if ok {
        Ok(())
    } else {
        Err(format!("{what} must be a plain lowercase identifier"))
    }
}

fn bounded(value: &str, max: usize, what: &str) -> Result<(), String> {
    if value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(format!("{what} is too long or has control characters"));
    }
    Ok(())
}

fn lower_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl ExtensionManifest {
    /// Canonical bytes: the digest and signature cover exactly these.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn validate(&self) -> Result<(), String> {
        plain_id(&self.extension_id, "extension id")?;
        plain_id(&self.publisher_id, "publisher id")?;
        if !lower_hex(&self.publisher_key_hex, PUBLIC_KEY_HEX_LEN) {
            return Err("publisher key must be 64 lowercase hex digits".to_owned());
        }
        if self.host_api_min == 0 || self.host_api_min > self.host_api_max {
            return Err("host API range is empty".to_owned());
        }
        if self.license.trim().is_empty() {
            return Err("license is required".to_owned());
        }
        bounded(&self.license, LICENSE_MAX_CHARS, "license")?;
        bounded(&self.description, DESCRIPTION_MAX_CHARS, "description")?;
        if !self.capabilities.windows(2).all(|w| w[0] < w[1]) {
            return Err("capabilities must be sorted and unique".to_owned());
        }
        if self.commands.is_empty() || self.commands.len() > COMMANDS_MAX {
            return Err("an extension declares 1-32 commands".to_owned());
        }
        let mut names = BTreeSet::new();
        for c in &self.commands {
            plain_id(&c.name, "command name")?;
            bounded(&c.title, TITLE_MAX_CHARS, "command title")?;
            if !names.insert(c.name.as_str()) {
                return Err("duplicate command name".to_owned());
            }
            if !self.capabilities.contains(&c.operation.capability()) {
                return Err(format!(
                    "command {} needs undeclared capability {}",
                    c.name,
                    c.operation.capability().as_str()
                ));
            }
            match (c.operation, c.max_rows) {
                (HostOperation::SnapshotRows, Some(n)) if (1..=ROWS_MAX).contains(&n) => {}
                (HostOperation::SnapshotRows, _) => {
                    return Err("snapshot_rows needs max_rows in 1..=1000".to_owned());
                }
                (_, Some(_)) => return Err("max_rows applies to snapshot_rows only".to_owned()),
                (_, None) => {}
            }
        }
        Ok(())
    }

    /// Whether this build's Host API is inside the declared range.
    #[must_use]
    pub const fn api_compatible(&self) -> bool {
        self.host_api_min <= HOST_API_VERSION && HOST_API_VERSION <= self.host_api_max
    }

    #[must_use]
    pub fn command(&self, name: &str) -> Option<&ExtensionCommand> {
        self.commands.iter().find(|c| c.name == name)
    }
}

/// A distributable pack: the manifest's canonical JSON and the publisher's
/// signature over its digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionPack {
    pub manifest_json: String,
    pub signature_hex: String,
}

/// The exact bytes a publisher signs for a release.
#[must_use]
pub fn signing_payload(digest: &DigestSha256) -> Vec<u8> {
    format!("{EXTENSION_SIGN_DOMAIN}\n{}\n", digest.to_hex()).into_bytes()
}

/// Why a pack or an install/upgrade/rollback was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionRefusal {
    PackTooLarge,
    /// Not a pack, not canonical JSON, unknown field or capability, or an
    /// invalid manifest.
    ManifestInvalid,
    SignatureInvalid,
    PublisherUnknown,
    PublisherRevoked,
    /// The pack's key differs from the trusted publisher's key.
    PublisherKeyMismatch,
    ReleaseRevoked,
    ApiIncompatible,
    AlreadyInstalled,
    NotInstalled,
    /// An upgrade must be to a strictly higher version of the same
    /// extension from the same publisher.
    NotAnUpgrade,
    NoPreviousRelease,
}

closed_vocabulary!(ExtensionRefusal, "extension refusal", {
    PackTooLarge => "pack_too_large",
    ManifestInvalid => "manifest_invalid",
    SignatureInvalid => "signature_invalid",
    PublisherUnknown => "publisher_unknown",
    PublisherRevoked => "publisher_revoked",
    PublisherKeyMismatch => "publisher_key_mismatch",
    ReleaseRevoked => "release_revoked",
    ApiIncompatible => "api_incompatible",
    AlreadyInstalled => "already_installed",
    NotInstalled => "not_installed",
    NotAnUpgrade => "not_an_upgrade",
    NoPreviousRelease => "no_previous_release",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublisherState {
    Trusted,
    Revoked,
}

closed_vocabulary!(PublisherState, "publisher state", {
    Trusted => "trusted",
    Revoked => "revoked",
});

/// A publisher key the user chose to trust (vault-wide). Revocation is
/// terminal and quarantines every install of that publisher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionPublisher {
    pub header: ObjectHeader,
    pub publisher_id: String,
    pub key_hex: String,
    pub state: PublisherState,
}

/// A verified release, stored exactly as received.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionRelease {
    pub header: ObjectHeader,
    pub digest: DigestSha256,
    pub manifest: ExtensionManifest,
    pub manifest_json: String,
    pub signature_hex: String,
    pub revoked: bool,
}

impl ExtensionRelease {
    pub fn validate(&self) -> Result<(), String> {
        if DigestSha256::of(self.manifest_json.as_bytes()) != self.digest {
            return Err("release digest does not match its manifest bytes".to_owned());
        }
        let parsed: ExtensionManifest = serde_json::from_str(&self.manifest_json)
            .map_err(|e| format!("release manifest: {e}"))?;
        if parsed != self.manifest || parsed.canonical_bytes() != self.manifest_json.as_bytes() {
            return Err("release manifest is not its canonical bytes".to_owned());
        }
        self.manifest.validate()?;
        if !lower_hex(&self.signature_hex, SIGNATURE_HEX_LEN) {
            return Err("signature must be 128 lowercase hex digits".to_owned());
        }
        Ok(())
    }
}

/// Install state of one extension in one Project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallState {
    Enabled,
    Disabled,
    /// The active release asks for a capability the user has not granted
    /// since it was installed; nothing runs until re-consent.
    PendingConsent,
    /// Publisher or release revoked; nothing runs.
    Quarantined,
    Uninstalled,
}

closed_vocabulary!(InstallState, "install state", {
    Enabled => "enabled",
    Disabled => "disabled",
    PendingConsent => "pending_consent",
    Quarantined => "quarantined",
    Uninstalled => "uninstalled",
});

/// One extension installed in one Project. `revision` increases on every
/// change; changes are compare-and-set on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionInstallRecord {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub extension_id: String,
    pub publisher_id: String,
    pub active_release: DigestSha256,
    pub previous_release: Option<DigestSha256>,
    pub state: InstallState,
    pub revision: u64,
}

/// A capability granted to one install, up to a data-class ceiling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionGrant {
    pub header: ObjectHeader,
    pub install_id: OpaqueId,
    pub project_id: OpaqueId,
    pub capability: ExtensionCapability,
    /// The most restrictive class the extension may read.
    pub data_class_ceiling: DataClass,
    pub revoked: bool,
}

impl ExtensionGrant {
    /// Whether this grant lets the extension read data of `class`.
    #[must_use]
    pub const fn covers(&self, class: DataClass) -> bool {
        !self.revoked && class.restrictiveness() <= self.data_class_ceiling.restrictiveness()
    }
}

/// Install-lifecycle actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleAction {
    Install,
    Upgrade,
    Rollback,
    Enable,
    Disable,
    Uninstall,
    Grant,
    RevokeGrant,
    Quarantine,
}

closed_vocabulary!(LifecycleAction, "lifecycle action", {
    Install => "install",
    Upgrade => "upgrade",
    Rollback => "rollback",
    Enable => "enable",
    Disable => "disable",
    Uninstall => "uninstall",
    Grant => "grant",
    RevokeGrant => "revoke_grant",
    Quarantine => "quarantine",
});

/// Every lifecycle attempt, applied or refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionLifecycleReceipt {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub action: LifecycleAction,
    pub extension_id: String,
    pub release: Option<DigestSha256>,
    pub install_id: Option<OpaqueId>,
    pub refusal: Option<ExtensionRefusal>,
    /// Capabilities the new release asks for beyond the old one.
    pub capability_expansion: Vec<ExtensionCapability>,
    pub resulting_state: Option<InstallState>,
}

/// Why an invocation did not run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationDenial {
    NotInstalled,
    NotEnabled,
    UnknownCommand,
    CapabilityNotGranted,
    DataClassAboveCeiling,
    TargetMissing,
    TargetNotInProject,
}

closed_vocabulary!(InvocationDenial, "invocation denial", {
    NotInstalled => "not_installed",
    NotEnabled => "not_enabled",
    UnknownCommand => "unknown_command",
    CapabilityNotGranted => "capability_not_granted",
    DataClassAboveCeiling => "data_class_above_ceiling",
    TargetMissing => "target_missing",
    TargetNotInProject => "target_not_in_project",
});

/// One invocation of a declarative command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionRuntimeReceipt {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub extension_id: String,
    pub release: Option<DigestSha256>,
    /// Bounded, as requested.
    pub command: String,
    pub operation: Option<HostOperation>,
    pub target: Option<OpaqueId>,
    pub denial: Option<InvocationDenial>,
    /// Digest of the canonical result JSON when the operation ran.
    pub result_digest: Option<DigestSha256>,
}

impl ExtensionRuntimeReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if self.denial.is_some() == self.result_digest.is_some() {
            return Err("exactly one of denial and result".to_owned());
        }
        if self.result_digest.is_some() && (self.release.is_none() || self.operation.is_none()) {
            return Err("a run names its release and operation".to_owned());
        }
        Ok(())
    }
}

/// Result of one invocation: its receipt and, when it ran, the result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionInvocation {
    pub receipt: ExtensionRuntimeReceipt,
    pub result: Option<serde_json::Value>,
}

/// Everything recorded for one Project's extensions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionProjectView {
    pub installs: Vec<ExtensionInstallRecord>,
    pub grants: Vec<ExtensionGrant>,
    pub lifecycle: Vec<ExtensionLifecycleReceipt>,
    pub runs: Vec<ExtensionRuntimeReceipt>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> ExtensionManifest {
        ExtensionManifest {
            extension_id: "org.example.cohort-peek".to_owned(),
            version: ExtensionVersion {
                major: 1,
                minor: 0,
                patch: 0,
            },
            publisher_id: "example".to_owned(),
            publisher_key_hex: "ab".repeat(32),
            host_api_min: 1,
            host_api_max: 1,
            license: "Apache-2.0".to_owned(),
            description: "Shows a snapshot's schema.".to_owned(),
            entrypoint: EntrypointKind::Declarative,
            capabilities: vec![ExtensionCapability::SnapshotSchemaRead],
            commands: vec![ExtensionCommand {
                name: "schema".to_owned(),
                title: "Show schema".to_owned(),
                operation: HostOperation::SnapshotSchema,
                max_rows: None,
            }],
        }
    }

    #[test]
    fn manifests_validate_strictly() {
        manifest().validate().unwrap();
        let mut undeclared = manifest();
        undeclared.commands[0].operation = HostOperation::SnapshotRows;
        undeclared.commands[0].max_rows = Some(10);
        assert!(
            undeclared.validate().is_err(),
            "capability must be declared"
        );
        let mut unsorted = manifest();
        unsorted.capabilities = vec![
            ExtensionCapability::SnapshotSchemaRead,
            ExtensionCapability::ProjectRead,
        ];
        assert!(unsorted.validate().is_err());
        let mut bad_id = manifest();
        bad_id.extension_id = "../evil".to_owned();
        assert!(bad_id.validate().is_err());
        let mut upper_key = manifest();
        upper_key.publisher_key_hex = "AB".repeat(32);
        assert!(upper_key.validate().is_err());
        let mut rows = manifest();
        rows.capabilities = vec![ExtensionCapability::SnapshotRowsRead];
        rows.commands[0].operation = HostOperation::SnapshotRows;
        assert!(rows.validate().is_err(), "rows need a bound");
        rows.commands[0].max_rows = Some(ROWS_MAX + 1);
        assert!(rows.validate().is_err());
        rows.commands[0].max_rows = Some(50);
        rows.validate().unwrap();
        let mut api = manifest();
        api.host_api_min = 2;
        api.host_api_max = 3;
        api.validate().unwrap();
        assert!(!api.api_compatible());
    }

    #[test]
    fn unknown_capabilities_and_entrypoints_fail_closed() {
        let json = String::from_utf8(manifest().canonical_bytes()).unwrap();
        for (from, to) in [
            ("\"snapshot_schema_read\"", "\"network_fetch\""),
            ("\"declarative\"", "\"wasm\""),
            ("\"declarative\"", "\"native\""),
            ("\"snapshot_schema\"", "\"shell\""),
        ] {
            let tampered = json.replacen(from, to, 1);
            assert!(
                serde_json::from_str::<ExtensionManifest>(&tampered).is_err(),
                "{to}"
            );
        }
        let extra = json.replacen('{', "{\"entry_script\":\"x.js\",", 1);
        assert!(serde_json::from_str::<ExtensionManifest>(&extra).is_err());
    }

    #[test]
    fn grants_respect_their_ceiling() {
        let grant = |ceiling, revoked| ExtensionGrant {
            header: ObjectHeader {
                id: OpaqueId::new("extension-grant-1"),
                schema_version: EXTENSION_SCHEMA_VERSION,
                realm_id: crate::objects::RealmId::new("realm"),
                authority_scope_id: crate::objects::AuthorityScopeId::new("scope"),
            },
            install_id: OpaqueId::new("extension-install-1"),
            project_id: OpaqueId::new("project-1"),
            capability: ExtensionCapability::SnapshotSchemaRead,
            data_class_ceiling: ceiling,
            revoked,
        };
        assert!(!grant(DataClass::Public, false).covers(DataClass::LocalPhi));
        assert!(grant(DataClass::LocalPhi, false).covers(DataClass::LocalPhi));
        assert!(grant(DataClass::LocalPhi, false).covers(DataClass::Public));
        assert!(!grant(DataClass::LocalPhi, true).covers(DataClass::Public));
    }

    #[test]
    fn signing_payload_is_domain_separated() {
        let d = DigestSha256::of(b"m");
        let p = String::from_utf8(signing_payload(&d)).unwrap();
        assert_eq!(p, format!("MEDSCALE_EXTENSION_SIGN_V1\n{}\n", d.to_hex()));
    }
}
