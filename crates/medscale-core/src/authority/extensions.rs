//! Community Extensions Core authority paths (Spec 087, foundation slice).
//!
//! Extensions are declarative: Core verifies a signed pack (size, exact
//! canonical manifest, closed vocabularies, Host API range, a publisher
//! the user trusts, a strict ed25519 signature over a domain-separated
//! digest, release not revoked), then records releases, installs, grants
//! and receipts. Invoking a command runs one read-only Host API operation
//! inside Core, only when the install is enabled, the capability is
//! granted for the Project, and the target's data class is within the
//! grant's ceiling. No extension code is ever loaded or executed.

use std::collections::BTreeSet;

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::extensions::{
    EXTENSION_SCHEMA_VERSION, ExtensionCapability, ExtensionGrant, ExtensionInstallRecord,
    ExtensionInvocation, ExtensionLifecycleReceipt, ExtensionManifest, ExtensionPack,
    ExtensionProjectView, ExtensionPublisher, ExtensionRefusal, ExtensionRelease,
    ExtensionRuntimeReceipt, HostOperation, InstallState, InvocationDenial, LifecycleAction,
    PACK_BYTES_MAX, PUBLIC_KEY_HEX_LEN, PublisherState, signing_payload,
};
use medscale_contracts::objects::{DigestSha256, ObjectHeader, OpaqueId};
use medscale_contracts::privacy_gate::DataClass;
use medscale_keys::{
    DeviceKeyError, generate_device_key, sign_device_payload, verify_device_signature,
};
use medscale_storage::{InstallChange, MetaError};

use super::data_sources::DataSources;

fn meta_err(err: MetaError) -> AuthorityError {
    match err {
        MetaError::NotFound => AuthorityError::NotFound,
        MetaError::Conflict(message) => AuthorityError::Conflict { message },
        MetaError::UnsupportedSchema(message) => AuthorityError::UnsupportedSchema { message },
        MetaError::CorruptObjectBody(message) => AuthorityError::Corrupt { message },
        other => AuthorityError::Internal {
            message: other.to_string(),
        },
    }
}

fn invalid(message: &str) -> AuthorityError {
    AuthorityError::InvalidArgument {
        message: message.to_owned(),
    }
}

/// Whether `key_hex` is a strict lowercase hex ed25519 public key that is
/// a valid, non-weak point.
#[must_use]
pub fn valid_public_key(key_hex: &str) -> bool {
    key_hex.len() == PUBLIC_KEY_HEX_LEN
        && !matches!(
            verify_device_signature(key_hex, b"", &"0".repeat(128)),
            Err(DeviceKeyError::PublicKey | DeviceKeyError::Encoding)
        )
}

/// Builds a signed pack from a manifest and the publisher's secret key
/// (the Extension SDK path; also used by tests).
pub fn sign_extension_pack(
    manifest: &ExtensionManifest,
    secret_hex: &str,
) -> Result<ExtensionPack, String> {
    manifest.validate()?;
    let manifest_json = String::from_utf8(manifest.canonical_bytes()).map_err(|e| e.to_string())?;
    let digest = DigestSha256::of(manifest_json.as_bytes());
    let signature_hex =
        sign_device_payload(secret_hex, &signing_payload(&digest)).map_err(|e| e.to_string())?;
    Ok(ExtensionPack {
        manifest_json,
        signature_hex,
    })
}

/// A fresh publisher key pair as `(secret_hex, public_hex)` (Extension SDK).
#[must_use]
pub fn generate_publisher_key() -> (String, String) {
    generate_device_key()
}

impl DataSources<'_> {
    fn ext_header(&self, id: OpaqueId) -> ObjectHeader {
        ObjectHeader {
            id,
            schema_version: EXTENSION_SCHEMA_VERSION,
            realm_id: self.realm.clone(),
            authority_scope_id: self.scope.clone(),
        }
    }

    fn ext_alloc(&self, prefix: &str) -> Result<OpaqueId, AuthorityError> {
        self.meta().alloc_ext_id(prefix).map_err(meta_err)
    }

    fn ext_scoped_install(
        &self,
        project_id: &OpaqueId,
        extension_id: &str,
    ) -> Result<Option<ExtensionInstallRecord>, AuthorityError> {
        match self.meta().get_ext_install(project_id, extension_id) {
            Ok(i) => {
                if i.header.realm_id != self.realm || i.header.authority_scope_id != self.scope {
                    return Err(AuthorityError::WrongScope);
                }
                Ok(Some(i))
            }
            Err(MetaError::NotFound) => Ok(None),
            Err(e) => Err(meta_err(e)),
        }
    }

    fn ext_receipt(
        &self,
        project_id: &OpaqueId,
        action: LifecycleAction,
        extension_id: &str,
    ) -> Result<ExtensionLifecycleReceipt, AuthorityError> {
        Ok(ExtensionLifecycleReceipt {
            header: self.ext_header(self.ext_alloc("extension-receipt")?),
            project_id: project_id.clone(),
            action,
            extension_id: extension_id.chars().take(64).collect(),
            release: None,
            install_id: None,
            refusal: None,
            capability_expansion: Vec::new(),
            resulting_state: None,
        })
    }

    fn ext_refuse(
        &mut self,
        mut receipt: ExtensionLifecycleReceipt,
        refusal: ExtensionRefusal,
    ) -> Result<ExtensionLifecycleReceipt, AuthorityError> {
        receipt.refusal = Some(refusal);
        receipt.resulting_state = None;
        self.meta().insert_ext_refusal(&receipt).map_err(meta_err)?;
        self.audit("extension.refused", vec![receipt.header.id.clone()])?;
        Ok(receipt)
    }

    // ----- publishers and revocation -----

    /// Trusts a publisher key (vault-wide). Only the user does this; no
    /// key is trusted by default.
    pub fn ext_trust_publisher(
        &mut self,
        publisher_id: String,
        key_hex: String,
    ) -> Result<ExtensionPublisher, AuthorityError> {
        if publisher_id.is_empty()
            || publisher_id.chars().count() > 64
            || !publisher_id.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_')
            })
        {
            return Err(invalid("publisher id must be a plain lowercase identifier"));
        }
        if !valid_public_key(&key_hex) {
            return Err(invalid("publisher key is not a valid ed25519 public key"));
        }
        let publisher = ExtensionPublisher {
            header: self.ext_header(self.ext_alloc("extension-publisher")?),
            publisher_id,
            key_hex,
            state: PublisherState::Trusted,
        };
        self.meta()
            .insert_ext_publisher(&publisher)
            .map_err(meta_err)?;
        self.audit(
            "extension.trust_publisher",
            vec![publisher.header.id.clone()],
        )?;
        Ok(publisher)
    }

    /// Quarantines every live install matching `affected`.
    fn ext_quarantine_plan(
        &self,
        affected: impl Fn(&ExtensionInstallRecord) -> bool,
    ) -> Result<Vec<(ExtensionInstallRecord, u64, ExtensionLifecycleReceipt)>, AuthorityError> {
        let mut plan = Vec::new();
        for install in self.meta().list_ext_installs().map_err(meta_err)? {
            if install.header.realm_id != self.realm
                || install.header.authority_scope_id != self.scope
                || matches!(
                    install.state,
                    InstallState::Quarantined | InstallState::Uninstalled
                )
                || !affected(&install)
            {
                continue;
            }
            let expected = install.revision;
            let mut next = install;
            next.state = InstallState::Quarantined;
            next.revision = expected + 1;
            let mut receipt = self.ext_receipt(
                &next.project_id,
                LifecycleAction::Quarantine,
                &next.extension_id,
            )?;
            receipt.release = Some(next.active_release.clone());
            receipt.install_id = Some(next.header.id.clone());
            receipt.resulting_state = Some(InstallState::Quarantined);
            plan.push((next, expected, receipt));
        }
        Ok(plan)
    }

    /// Revokes a publisher (terminal) and quarantines its installs.
    pub fn ext_revoke_publisher(
        &mut self,
        publisher_id: &str,
    ) -> Result<Vec<ExtensionLifecycleReceipt>, AuthorityError> {
        let publisher = self
            .meta()
            .get_ext_publisher(publisher_id)
            .map_err(meta_err)?;
        if publisher.state == PublisherState::Revoked {
            return Err(AuthorityError::Conflict {
                message: "publisher is already revoked".to_owned(),
            });
        }
        let plan = self.ext_quarantine_plan(|i| i.publisher_id == publisher_id)?;
        self.meta()
            .commit_ext_revocation(Some(publisher_id), None, &plan)
            .map_err(meta_err)?;
        self.audit("extension.revoke_publisher", vec![publisher.header.id])?;
        Ok(plan.into_iter().map(|(_, _, r)| r).collect())
    }

    /// Revokes one release and quarantines installs running it.
    pub fn ext_revoke_release(
        &mut self,
        digest: &DigestSha256,
    ) -> Result<Vec<ExtensionLifecycleReceipt>, AuthorityError> {
        let release = self.meta().get_ext_release(digest).map_err(meta_err)?;
        if release.revoked {
            return Err(AuthorityError::Conflict {
                message: "release is already revoked".to_owned(),
            });
        }
        let plan = self.ext_quarantine_plan(|i| &i.active_release == digest)?;
        self.meta()
            .commit_ext_revocation(None, Some(digest), &plan)
            .map_err(meta_err)?;
        self.audit("extension.revoke_release", vec![release.header.id])?;
        Ok(plan.into_iter().map(|(_, _, r)| r).collect())
    }

    // ----- verification -----

    /// Verifies a pack and returns its release (stored or new).
    fn ext_verify(&self, pack_json: &str) -> Result<ExtensionRelease, ExtensionRefusal> {
        if pack_json.len() as u64 > PACK_BYTES_MAX {
            return Err(ExtensionRefusal::PackTooLarge);
        }
        let pack: ExtensionPack =
            serde_json::from_str(pack_json).map_err(|_| ExtensionRefusal::ManifestInvalid)?;
        let manifest: ExtensionManifest = serde_json::from_str(&pack.manifest_json)
            .map_err(|_| ExtensionRefusal::ManifestInvalid)?;
        if manifest.canonical_bytes() != pack.manifest_json.as_bytes()
            || manifest.validate().is_err()
        {
            return Err(ExtensionRefusal::ManifestInvalid);
        }
        let publisher = match self.meta().get_ext_publisher(&manifest.publisher_id) {
            Ok(p) => p,
            Err(_) => return Err(ExtensionRefusal::PublisherUnknown),
        };
        if publisher.header.realm_id != self.realm
            || publisher.header.authority_scope_id != self.scope
        {
            return Err(ExtensionRefusal::PublisherUnknown);
        }
        if publisher.state == PublisherState::Revoked {
            return Err(ExtensionRefusal::PublisherRevoked);
        }
        if publisher.key_hex != manifest.publisher_key_hex {
            return Err(ExtensionRefusal::PublisherKeyMismatch);
        }
        let digest = DigestSha256::of(pack.manifest_json.as_bytes());
        if verify_device_signature(
            &publisher.key_hex,
            &signing_payload(&digest),
            &pack.signature_hex,
        )
        .is_err()
        {
            return Err(ExtensionRefusal::SignatureInvalid);
        }
        if !manifest.api_compatible() {
            return Err(ExtensionRefusal::ApiIncompatible);
        }
        match self.meta().get_ext_release(&digest) {
            Ok(existing) if existing.revoked => Err(ExtensionRefusal::ReleaseRevoked),
            Ok(existing) => Ok(existing),
            Err(_) => Ok(ExtensionRelease {
                header: ObjectHeader {
                    id: OpaqueId::new("pending"),
                    schema_version: EXTENSION_SCHEMA_VERSION,
                    realm_id: self.realm.clone(),
                    authority_scope_id: self.scope.clone(),
                },
                digest,
                manifest,
                manifest_json: pack.manifest_json,
                signature_hex: pack.signature_hex,
                revoked: false,
            }),
        }
    }

    /// Stores a newly verified release (idempotent for a stored one).
    fn ext_store_release(
        &self,
        mut release: ExtensionRelease,
    ) -> Result<ExtensionRelease, AuthorityError> {
        if release.header.id.as_str() != "pending" {
            return Ok(release);
        }
        release.header.id = self.ext_alloc("extension-release")?;
        self.meta().insert_ext_release(&release).map_err(meta_err)?;
        Ok(release)
    }

    fn ext_commit(
        &mut self,
        install: &ExtensionInstallRecord,
        expected_revision: Option<u64>,
        new_grants: &[ExtensionGrant],
        revoke_grants: &[OpaqueId],
        receipt: &ExtensionLifecycleReceipt,
        audit: &str,
    ) -> Result<(), AuthorityError> {
        self.meta()
            .commit_ext_install_change(&InstallChange {
                install,
                expected_revision,
                new_grants,
                revoke_grants,
                receipt,
            })
            .map_err(meta_err)?;
        self.audit(
            audit,
            vec![install.header.id.clone(), receipt.header.id.clone()],
        )
    }

    // ----- lifecycle -----

    /// Installs a verified pack into a Project with no grants.
    pub fn ext_install(
        &mut self,
        project_id: &OpaqueId,
        pack_json: &str,
    ) -> Result<ExtensionLifecycleReceipt, AuthorityError> {
        self.require_project(project_id)?;
        let release = match self.ext_verify(pack_json) {
            Ok(r) => r,
            Err(refusal) => {
                let r = self.ext_receipt(project_id, LifecycleAction::Install, "")?;
                return self.ext_refuse(r, refusal);
            }
        };
        let extension_id = release.manifest.extension_id.clone();
        let mut receipt = self.ext_receipt(project_id, LifecycleAction::Install, &extension_id)?;
        receipt.release = Some(release.digest.clone());
        let existing = self.ext_scoped_install(project_id, &extension_id)?;
        if existing
            .as_ref()
            .is_some_and(|i| i.state != InstallState::Uninstalled)
        {
            return self.ext_refuse(receipt, ExtensionRefusal::AlreadyInstalled);
        }
        let release = self.ext_store_release(release)?;
        let (install, expected) = match existing {
            Some(old) => (
                ExtensionInstallRecord {
                    publisher_id: release.manifest.publisher_id.clone(),
                    active_release: release.digest.clone(),
                    previous_release: None,
                    state: InstallState::Enabled,
                    revision: old.revision + 1,
                    ..old.clone()
                },
                Some(old.revision),
            ),
            None => (
                ExtensionInstallRecord {
                    header: self.ext_header(self.ext_alloc("extension-install")?),
                    project_id: project_id.clone(),
                    extension_id,
                    publisher_id: release.manifest.publisher_id.clone(),
                    active_release: release.digest.clone(),
                    previous_release: None,
                    state: InstallState::Enabled,
                    revision: 1,
                },
                None,
            ),
        };
        receipt.install_id = Some(install.header.id.clone());
        receipt.resulting_state = Some(install.state);
        self.ext_commit(&install, expected, &[], &[], &receipt, "extension.install")?;
        Ok(receipt)
    }

    /// Upgrades to a strictly higher version of the same extension from
    /// the same publisher. New capabilities put the install in
    /// `pending_consent` until the user enables it again.
    pub fn ext_upgrade(
        &mut self,
        project_id: &OpaqueId,
        pack_json: &str,
    ) -> Result<ExtensionLifecycleReceipt, AuthorityError> {
        self.require_project(project_id)?;
        let release = match self.ext_verify(pack_json) {
            Ok(r) => r,
            Err(refusal) => {
                let r = self.ext_receipt(project_id, LifecycleAction::Upgrade, "")?;
                return self.ext_refuse(r, refusal);
            }
        };
        let extension_id = release.manifest.extension_id.clone();
        let mut receipt = self.ext_receipt(project_id, LifecycleAction::Upgrade, &extension_id)?;
        receipt.release = Some(release.digest.clone());
        let Some(old) = self
            .ext_scoped_install(project_id, &extension_id)?
            .filter(|i| {
                !matches!(
                    i.state,
                    InstallState::Uninstalled | InstallState::Quarantined
                )
            })
        else {
            return self.ext_refuse(receipt, ExtensionRefusal::NotInstalled);
        };
        let current = self
            .meta()
            .get_ext_release(&old.active_release)
            .map_err(meta_err)?;
        if current.manifest.publisher_id != release.manifest.publisher_id
            || release.manifest.version <= current.manifest.version
        {
            return self.ext_refuse(receipt, ExtensionRefusal::NotAnUpgrade);
        }
        let old_caps: BTreeSet<ExtensionCapability> =
            current.manifest.capabilities.iter().copied().collect();
        let expansion: Vec<ExtensionCapability> = release
            .manifest
            .capabilities
            .iter()
            .copied()
            .filter(|c| !old_caps.contains(c))
            .collect();
        let release = self.ext_store_release(release)?;
        let state = if expansion.is_empty() {
            old.state
        } else {
            InstallState::PendingConsent
        };
        let install = ExtensionInstallRecord {
            active_release: release.digest.clone(),
            previous_release: Some(old.active_release.clone()),
            state,
            revision: old.revision + 1,
            ..old.clone()
        };
        receipt.install_id = Some(install.header.id.clone());
        receipt.capability_expansion = expansion;
        receipt.resulting_state = Some(state);
        self.ext_commit(
            &install,
            Some(old.revision),
            &[],
            &[],
            &receipt,
            "extension.upgrade",
        )?;
        Ok(receipt)
    }

    /// Returns to the previous release (if it is not revoked).
    pub fn ext_rollback(
        &mut self,
        project_id: &OpaqueId,
        extension_id: &str,
    ) -> Result<ExtensionLifecycleReceipt, AuthorityError> {
        self.require_project(project_id)?;
        let mut receipt = self.ext_receipt(project_id, LifecycleAction::Rollback, extension_id)?;
        let Some(old) = self
            .ext_scoped_install(project_id, extension_id)?
            .filter(|i| {
                !matches!(
                    i.state,
                    InstallState::Uninstalled | InstallState::Quarantined
                )
            })
        else {
            return self.ext_refuse(receipt, ExtensionRefusal::NotInstalled);
        };
        let Some(previous) = old.previous_release.clone() else {
            return self.ext_refuse(receipt, ExtensionRefusal::NoPreviousRelease);
        };
        let release = self.meta().get_ext_release(&previous).map_err(meta_err)?;
        if release.revoked {
            return self.ext_refuse(receipt, ExtensionRefusal::ReleaseRevoked);
        }
        let state = if old.state == InstallState::Disabled {
            InstallState::Disabled
        } else {
            InstallState::Enabled
        };
        let install = ExtensionInstallRecord {
            active_release: previous.clone(),
            previous_release: None,
            state,
            revision: old.revision + 1,
            ..old.clone()
        };
        receipt.release = Some(previous);
        receipt.install_id = Some(install.header.id.clone());
        receipt.resulting_state = Some(state);
        self.ext_commit(
            &install,
            Some(old.revision),
            &[],
            &[],
            &receipt,
            "extension.rollback",
        )?;
        Ok(receipt)
    }

    /// Enables (also: consents to a pending capability expansion) or
    /// disables an install.
    pub fn ext_set_enabled(
        &mut self,
        project_id: &OpaqueId,
        extension_id: &str,
        enabled: bool,
    ) -> Result<ExtensionLifecycleReceipt, AuthorityError> {
        self.require_project(project_id)?;
        let action = if enabled {
            LifecycleAction::Enable
        } else {
            LifecycleAction::Disable
        };
        let mut receipt = self.ext_receipt(project_id, action, extension_id)?;
        let Some(old) = self.ext_scoped_install(project_id, extension_id)? else {
            return self.ext_refuse(receipt, ExtensionRefusal::NotInstalled);
        };
        match old.state {
            InstallState::Uninstalled => {
                return self.ext_refuse(receipt, ExtensionRefusal::NotInstalled);
            }
            InstallState::Quarantined => {
                return self.ext_refuse(receipt, ExtensionRefusal::ReleaseRevoked);
            }
            _ => {}
        }
        let state = if enabled {
            InstallState::Enabled
        } else {
            InstallState::Disabled
        };
        let install = ExtensionInstallRecord {
            state,
            revision: old.revision + 1,
            ..old.clone()
        };
        receipt.release = Some(install.active_release.clone());
        receipt.install_id = Some(install.header.id.clone());
        receipt.resulting_state = Some(state);
        self.ext_commit(
            &install,
            Some(old.revision),
            &[],
            &[],
            &receipt,
            "extension.set_enabled",
        )?;
        Ok(receipt)
    }

    /// Uninstalls and revokes every grant. Releases stay recorded.
    pub fn ext_uninstall(
        &mut self,
        project_id: &OpaqueId,
        extension_id: &str,
    ) -> Result<ExtensionLifecycleReceipt, AuthorityError> {
        self.require_project(project_id)?;
        let mut receipt = self.ext_receipt(project_id, LifecycleAction::Uninstall, extension_id)?;
        let Some(old) = self
            .ext_scoped_install(project_id, extension_id)?
            .filter(|i| i.state != InstallState::Uninstalled)
        else {
            return self.ext_refuse(receipt, ExtensionRefusal::NotInstalled);
        };
        let revoke: Vec<OpaqueId> = self
            .meta()
            .list_ext_grants(Some(&old.header.id))
            .map_err(meta_err)?
            .into_iter()
            .filter(|g| !g.revoked)
            .map(|g| g.header.id)
            .collect();
        let install = ExtensionInstallRecord {
            state: InstallState::Uninstalled,
            revision: old.revision + 1,
            ..old.clone()
        };
        receipt.release = Some(install.active_release.clone());
        receipt.install_id = Some(install.header.id.clone());
        receipt.resulting_state = Some(InstallState::Uninstalled);
        self.ext_commit(
            &install,
            Some(old.revision),
            &[],
            &revoke,
            &receipt,
            "extension.uninstall",
        )?;
        Ok(receipt)
    }

    /// Grants (or, with `ceiling = None`, revokes) one capability declared
    /// by the active release, for one Project.
    pub fn ext_grant(
        &mut self,
        project_id: &OpaqueId,
        extension_id: &str,
        capability: ExtensionCapability,
        ceiling: Option<DataClass>,
    ) -> Result<ExtensionLifecycleReceipt, AuthorityError> {
        self.require_project(project_id)?;
        let action = if ceiling.is_some() {
            LifecycleAction::Grant
        } else {
            LifecycleAction::RevokeGrant
        };
        let mut receipt = self.ext_receipt(project_id, action, extension_id)?;
        let Some(old) = self
            .ext_scoped_install(project_id, extension_id)?
            .filter(|i| i.state != InstallState::Uninstalled)
        else {
            return self.ext_refuse(receipt, ExtensionRefusal::NotInstalled);
        };
        let release = self
            .meta()
            .get_ext_release(&old.active_release)
            .map_err(meta_err)?;
        if ceiling.is_some() && !release.manifest.capabilities.contains(&capability) {
            return self.ext_refuse(receipt, ExtensionRefusal::ManifestInvalid);
        }
        let revoke: Vec<OpaqueId> = self
            .meta()
            .list_ext_grants(Some(&old.header.id))
            .map_err(meta_err)?
            .into_iter()
            .filter(|g| !g.revoked && g.capability == capability)
            .map(|g| g.header.id)
            .collect();
        let new_grants = match ceiling {
            Some(ceiling) => vec![ExtensionGrant {
                header: self.ext_header(self.ext_alloc("extension-grant")?),
                install_id: old.header.id.clone(),
                project_id: project_id.clone(),
                capability,
                data_class_ceiling: ceiling,
                revoked: false,
            }],
            None => Vec::new(),
        };
        let install = ExtensionInstallRecord {
            revision: old.revision + 1,
            ..old.clone()
        };
        receipt.release = Some(install.active_release.clone());
        receipt.install_id = Some(install.header.id.clone());
        receipt.resulting_state = Some(install.state);
        self.ext_commit(
            &install,
            Some(old.revision),
            &new_grants,
            &revoke,
            &receipt,
            "extension.grant",
        )?;
        Ok(receipt)
    }

    /// Installs, grants and receipts of one Project (this realm and scope).
    pub fn ext_list(&self, project_id: &OpaqueId) -> Result<ExtensionProjectView, AuthorityError> {
        self.require_project(project_id)?;
        let mine =
            |h: &ObjectHeader| h.realm_id == self.realm && h.authority_scope_id == self.scope;
        let installs: Vec<ExtensionInstallRecord> = self
            .meta()
            .list_ext_installs()
            .map_err(meta_err)?
            .into_iter()
            .filter(|i| &i.project_id == project_id && mine(&i.header))
            .collect();
        let grants = self
            .meta()
            .list_ext_grants(None)
            .map_err(meta_err)?
            .into_iter()
            .filter(|g| &g.project_id == project_id && mine(&g.header))
            .collect();
        let lifecycle = self
            .meta()
            .list_ext_lifecycle_receipts()
            .map_err(meta_err)?
            .into_iter()
            .filter(|r| &r.project_id == project_id && mine(&r.header))
            .collect();
        let runs = self
            .meta()
            .list_ext_runtime_receipts()
            .map_err(meta_err)?
            .into_iter()
            .filter(|r| &r.project_id == project_id && mine(&r.header))
            .collect();
        Ok(ExtensionProjectView {
            installs,
            grants,
            lifecycle,
            runs,
        })
    }

    // ----- invocation -----

    /// Runs one declarative command. Every call leaves a runtime receipt.
    pub fn ext_invoke(
        &mut self,
        project_id: &OpaqueId,
        extension_id: &str,
        command: &str,
        target: Option<OpaqueId>,
    ) -> Result<ExtensionInvocation, AuthorityError> {
        self.require_project(project_id)?;
        let mut receipt = ExtensionRuntimeReceipt {
            header: self.ext_header(self.ext_alloc("extension-run")?),
            project_id: project_id.clone(),
            extension_id: extension_id.chars().take(64).collect(),
            release: None,
            command: command.chars().take(64).collect(),
            operation: None,
            target: target.clone(),
            denial: None,
            result_digest: None,
        };
        let outcome = self.ext_evaluate(
            project_id,
            extension_id,
            command,
            target.as_ref(),
            &mut receipt,
        );
        let result = match outcome {
            Ok(value) => {
                let bytes = serde_json::to_vec(&value).map_err(|e| AuthorityError::Internal {
                    message: e.to_string(),
                })?;
                receipt.result_digest = Some(DigestSha256::of(&bytes));
                Some(value)
            }
            Err(denial) => {
                receipt.denial = Some(denial);
                None
            }
        };
        self.meta()
            .insert_ext_runtime_receipt(&receipt)
            .map_err(meta_err)?;
        self.audit("extension.invoke", vec![receipt.header.id.clone()])?;
        Ok(ExtensionInvocation { receipt, result })
    }

    fn ext_evaluate(
        &self,
        project_id: &OpaqueId,
        extension_id: &str,
        command: &str,
        target: Option<&OpaqueId>,
        receipt: &mut ExtensionRuntimeReceipt,
    ) -> Result<serde_json::Value, InvocationDenial> {
        let install = self
            .ext_scoped_install(project_id, extension_id)
            .ok()
            .flatten()
            .filter(|i| i.state != InstallState::Uninstalled)
            .ok_or(InvocationDenial::NotInstalled)?;
        if install.state != InstallState::Enabled {
            return Err(InvocationDenial::NotEnabled);
        }
        let release = self
            .meta()
            .get_ext_release(&install.active_release)
            .map_err(|_| InvocationDenial::NotEnabled)?;
        let publisher_trusted = self
            .meta()
            .get_ext_publisher(&release.manifest.publisher_id)
            .is_ok_and(|p| p.state == PublisherState::Trusted);
        if release.revoked || !publisher_trusted {
            return Err(InvocationDenial::NotEnabled);
        }
        receipt.release = Some(release.digest.clone());
        let cmd = release
            .manifest
            .command(command)
            .ok_or(InvocationDenial::UnknownCommand)?;
        receipt.operation = Some(cmd.operation);
        let grants = self
            .meta()
            .list_ext_grants(Some(&install.header.id))
            .map_err(|_| InvocationDenial::CapabilityNotGranted)?;
        let grant = grants
            .iter()
            .find(|g| !g.revoked && g.capability == cmd.operation.capability())
            .ok_or(InvocationDenial::CapabilityNotGranted)?;
        if cmd.operation.reads_snapshot() {
            let snapshot_id = target.ok_or(InvocationDenial::TargetMissing)?;
            let record = self
                .scoped_snapshot(snapshot_id)
                .map_err(|_| InvocationDenial::TargetMissing)?;
            if &record.project_id != project_id {
                return Err(InvocationDenial::TargetNotInProject);
            }
            let class = match self.meta().get_classification(project_id, snapshot_id) {
                Ok(Some(row)) => row.data_class,
                _ => DataClass::LocalPhi,
            };
            if !grant.covers(class) {
                return Err(InvocationDenial::DataClassAboveCeiling);
            }
            return match cmd.operation {
                HostOperation::SnapshotRows => {
                    let page = self
                        .snapshot_rows(snapshot_id, cmd.max_rows, &None, &[], &[])
                        .map_err(|_| InvocationDenial::TargetMissing)?;
                    Ok(serde_json::json!({
                        "snapshot_id": snapshot_id.as_str(),
                        "rows": page.rows,
                    }))
                }
                _ => Ok(serde_json::json!({
                    "snapshot_id": snapshot_id.as_str(),
                    "row_count": record.snapshot.row_count,
                    "fields": record.schema.fields,
                })),
            };
        }
        // Project metadata carries no classification row: fail closed.
        if !grant.covers(DataClass::LocalPhi) {
            return Err(InvocationDenial::DataClassAboveCeiling);
        }
        let project = self
            .meta()
            .get_project(project_id)
            .map_err(|_| InvocationDenial::TargetMissing)?;
        Ok(serde_json::json!({
            "project_id": project_id.as_str(),
            "name": project.name,
            "status": format!("{:?}", project.status),
        }))
    }
}
