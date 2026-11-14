//! Research Pack authority (Spec 089, first domain: Clinical Research).
//!
//! Core owns the domain objects a Pack describes: it installs a shipped,
//! digest-pinned Pack version in a Project, creates and validates
//! artifacts against the Pack's schema, moves them only along the Pack's
//! workflow, and records evidence assessments. Upgrades apply the next
//! version's declared, non-destructive migrations to every artifact in one
//! transaction. Uninstalling disables the Pack; its artifacts remain
//! readable and untouched. Nothing here installs an Extension or grants a
//! capability.

use std::collections::BTreeMap;

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::{ObjectHeader, OpaqueId};
use medscale_contracts::research_packs::{
    EvidenceAssessment, FieldValue, PackActRequest, PackActResult, PackAction, PackInstallState,
    PackReceipt, RESEARCH_PACK_SCHEMA_VERSION, ResearchArtifact, ResearchPackInstall,
    ResearchPackManifest, ResearchPackMigration, clinical_research_pack,
};
use medscale_storage::{MetaError, PackChange};

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

fn invalid(message: impl Into<String>) -> AuthorityError {
    AuthorityError::InvalidArgument {
        message: message.into(),
    }
}

fn manifest(pack_id: &str, version: u32) -> Result<ResearchPackManifest, AuthorityError> {
    clinical_research_pack(version)
        .filter(|m| m.pack_id == pack_id)
        .ok_or_else(|| invalid(format!("no shipped pack {pack_id} version {version}")))
}

impl DataSources<'_> {
    fn rp_header(&self, id: OpaqueId) -> ObjectHeader {
        ObjectHeader {
            id,
            schema_version: RESEARCH_PACK_SCHEMA_VERSION,
            realm_id: self.realm.clone(),
            authority_scope_id: self.scope.clone(),
        }
    }

    fn rp_receipt(
        &self,
        project_id: &OpaqueId,
        pack_id: &str,
        action: PackAction,
        targets: Vec<OpaqueId>,
    ) -> Result<PackReceipt, AuthorityError> {
        Ok(PackReceipt {
            header: self.rp_header(
                self.meta()
                    .alloc_pack_id("pack-receipt")
                    .map_err(meta_err)?,
            ),
            project_id: project_id.clone(),
            pack_id: pack_id.to_owned(),
            action,
            targets,
            from_version: None,
            to_version: None,
        })
    }

    fn rp_commit(&mut self, change: PackChange<'_>, audit: &str) -> Result<(), AuthorityError> {
        let targets = change
            .receipt
            .map(|r| vec![r.header.id.clone()])
            .unwrap_or_default();
        self.meta().commit_pack_change(&change).map_err(meta_err)?;
        self.audit(audit, targets)
    }

    fn rp_install_of(
        &self,
        project_id: &OpaqueId,
        pack_id: &str,
    ) -> Result<Option<ResearchPackInstall>, AuthorityError> {
        match self.meta().get_pack_install(project_id, pack_id) {
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

    fn rp_enabled(
        &self,
        project_id: &OpaqueId,
        pack_id: &str,
    ) -> Result<ResearchPackInstall, AuthorityError> {
        match self.rp_install_of(project_id, pack_id)? {
            Some(i) if i.state == PackInstallState::Enabled => Ok(i),
            Some(_) => Err(AuthorityError::Conflict {
                message: "the pack is uninstalled in this project; its data is read-only"
                    .to_owned(),
            }),
            None => Err(AuthorityError::NotFound),
        }
    }

    fn rp_artifact(&self, id: &OpaqueId) -> Result<ResearchArtifact, AuthorityError> {
        let a = self.meta().get_research_artifact(id).map_err(meta_err)?;
        if a.header.realm_id != self.realm || a.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(a)
    }

    /// Shipped Packs: `(pack_id, version, manifest)`.
    #[must_use]
    pub fn pack_catalog() -> Vec<ResearchPackManifest> {
        (1..=medscale_contracts::research_packs::CLINICAL_RESEARCH_PACK_LATEST)
            .filter_map(clinical_research_pack)
            .collect()
    }

    /// Installs version 1 of a shipped Pack, or re-enables an uninstalled
    /// one at its recorded version.
    pub fn pack_install(
        &mut self,
        project_id: &OpaqueId,
        pack_id: &str,
    ) -> Result<ResearchPackInstall, AuthorityError> {
        self.require_project(project_id)?;
        let (install, expected) = match self.rp_install_of(project_id, pack_id)? {
            Some(old) if old.state == PackInstallState::Enabled => {
                return Err(AuthorityError::Conflict {
                    message: "pack is already installed".to_owned(),
                });
            }
            Some(old) => (
                ResearchPackInstall {
                    state: PackInstallState::Enabled,
                    revision: old.revision + 1,
                    ..old.clone()
                },
                Some(old.revision),
            ),
            None => {
                let m = manifest(pack_id, 1)?;
                (
                    ResearchPackInstall {
                        header: self.rp_header(
                            self.meta()
                                .alloc_pack_id("pack-install")
                                .map_err(meta_err)?,
                        ),
                        project_id: project_id.clone(),
                        pack_id: pack_id.to_owned(),
                        version: 1,
                        manifest_digest: m.digest(),
                        state: PackInstallState::Enabled,
                        revision: 1,
                    },
                    None,
                )
            }
        };
        let mut receipt = self.rp_receipt(
            project_id,
            pack_id,
            PackAction::Install,
            vec![install.header.id.clone()],
        )?;
        receipt.to_version = Some(install.version);
        self.rp_commit(
            PackChange {
                install: Some((&install, expected)),
                artifacts: Vec::new(),
                receipt: Some(&receipt),
            },
            "pack.install",
        )?;
        Ok(install)
    }

    /// Upgrades to the next shipped version, migrating every artifact of
    /// the Pack in the Project in the same transaction.
    pub fn pack_upgrade(
        &mut self,
        project_id: &OpaqueId,
        pack_id: &str,
    ) -> Result<ResearchPackInstall, AuthorityError> {
        self.require_project(project_id)?;
        let old = self.rp_enabled(project_id, pack_id)?;
        let next = manifest(pack_id, old.version + 1)?;
        let mut migrated = Vec::new();
        for a in self
            .meta()
            .list_research_artifacts(Some((project_id, pack_id)))
            .map_err(meta_err)?
        {
            let mut b = a.clone();
            for step in &next.migrations {
                if let ResearchPackMigration::AddField {
                    type_id,
                    field,
                    default,
                } = step
                    && &b.type_id == type_id
                {
                    b.fields
                        .entry(field.name.clone())
                        .or_insert_with(|| default.clone());
                }
            }
            b.pack_version = next.version;
            b.revision = a.revision + 1;
            migrated.push((b, a.revision));
        }
        let install = ResearchPackInstall {
            version: next.version,
            manifest_digest: next.digest(),
            revision: old.revision + 1,
            ..old.clone()
        };
        let mut targets = vec![install.header.id.clone()];
        targets.extend(migrated.iter().map(|(a, _)| a.header.id.clone()));
        let mut receipt = self.rp_receipt(project_id, pack_id, PackAction::Upgrade, targets)?;
        receipt.from_version = Some(old.version);
        receipt.to_version = Some(next.version);
        self.rp_commit(
            PackChange {
                install: Some((&install, Some(old.revision))),
                artifacts: migrated.iter().map(|(a, e)| (a, Some(*e))).collect(),
                receipt: Some(&receipt),
            },
            "pack.upgrade",
        )?;
        Ok(install)
    }

    /// Disables a Pack in a Project. Its artifacts are kept, unchanged.
    pub fn pack_uninstall(
        &mut self,
        project_id: &OpaqueId,
        pack_id: &str,
    ) -> Result<ResearchPackInstall, AuthorityError> {
        self.require_project(project_id)?;
        let old = self.rp_enabled(project_id, pack_id)?;
        let install = ResearchPackInstall {
            state: PackInstallState::Disabled,
            revision: old.revision + 1,
            ..old.clone()
        };
        let receipt = self.rp_receipt(
            project_id,
            pack_id,
            PackAction::Uninstall,
            vec![install.header.id.clone()],
        )?;
        self.rp_commit(
            PackChange {
                install: Some((&install, Some(old.revision))),
                artifacts: Vec::new(),
                receipt: Some(&receipt),
            },
            "pack.uninstall",
        )?;
        Ok(install)
    }

    pub fn artifact_create(
        &mut self,
        project_id: &OpaqueId,
        pack_id: &str,
        type_id: &str,
        fields: BTreeMap<String, FieldValue>,
    ) -> Result<ResearchArtifact, AuthorityError> {
        self.require_project(project_id)?;
        let install = self.rp_enabled(project_id, pack_id)?;
        let m = manifest(pack_id, install.version)?;
        m.check_fields(type_id, &fields).map_err(invalid)?;
        let schema = m
            .schema(type_id)
            .ok_or_else(|| invalid("unknown artifact type"))?;
        let initial = m
            .workflow(&schema.workflow_id)
            .map(|w| w.initial.clone())
            .ok_or_else(|| invalid("unknown workflow"))?;
        let artifact = ResearchArtifact {
            header: self.rp_header(
                self.meta()
                    .alloc_pack_id("research-artifact")
                    .map_err(meta_err)?,
            ),
            project_id: project_id.clone(),
            pack_id: pack_id.to_owned(),
            pack_version: install.version,
            type_id: type_id.to_owned(),
            fields,
            workflow_state: initial,
            assessments: Vec::new(),
            revision: 1,
        };
        let receipt = self.rp_receipt(
            project_id,
            pack_id,
            PackAction::Create,
            vec![artifact.header.id.clone()],
        )?;
        self.rp_commit(
            PackChange {
                install: None,
                artifacts: vec![(&artifact, None)],
                receipt: Some(&receipt),
            },
            "pack.artifact_create",
        )?;
        Ok(artifact)
    }

    fn artifact_change(
        &mut self,
        artifact_id: &OpaqueId,
        expected_revision: u64,
        action: PackAction,
        edit: impl FnOnce(&ResearchPackManifest, &mut ResearchArtifact) -> Result<(), AuthorityError>,
    ) -> Result<ResearchArtifact, AuthorityError> {
        let old = self.rp_artifact(artifact_id)?;
        if old.revision != expected_revision {
            return Err(AuthorityError::Conflict {
                message: format!(
                    "stale revision: expected {expected_revision}, current is {}",
                    old.revision
                ),
            });
        }
        self.rp_enabled(&old.project_id, &old.pack_id)?;
        let m = manifest(&old.pack_id, old.pack_version)?;
        let mut next = old.clone();
        edit(&m, &mut next)?;
        next.revision = old.revision + 1;
        let receipt = self.rp_receipt(
            &old.project_id,
            &old.pack_id,
            action,
            vec![next.header.id.clone()],
        )?;
        self.rp_commit(
            PackChange {
                install: None,
                artifacts: vec![(&next, Some(old.revision))],
                receipt: Some(&receipt),
            },
            "pack.artifact_change",
        )?;
        Ok(next)
    }

    /// Replaces an artifact's fields (validated against its schema).
    pub fn artifact_update(
        &mut self,
        artifact_id: &OpaqueId,
        expected_revision: u64,
        fields: BTreeMap<String, FieldValue>,
    ) -> Result<ResearchArtifact, AuthorityError> {
        self.artifact_change(
            artifact_id,
            expected_revision,
            PackAction::Update,
            |m, a| {
                m.check_fields(&a.type_id, &fields).map_err(invalid)?;
                a.fields = fields;
                Ok(())
            },
        )
    }

    /// Moves an artifact along its Pack workflow.
    pub fn artifact_transition(
        &mut self,
        artifact_id: &OpaqueId,
        expected_revision: u64,
        to: &str,
    ) -> Result<ResearchArtifact, AuthorityError> {
        self.artifact_change(
            artifact_id,
            expected_revision,
            PackAction::Transition,
            |m, a| {
                let workflow = m
                    .schema(&a.type_id)
                    .and_then(|s| m.workflow(&s.workflow_id))
                    .ok_or_else(|| invalid("unknown workflow"))?;
                if !workflow.allows(&a.workflow_state, to) {
                    return Err(invalid(format!(
                        "the workflow does not allow {} -> {to}",
                        a.workflow_state
                    )));
                }
                a.workflow_state = to.to_owned();
                Ok(())
            },
        )
    }

    /// Records one evidence assessment on an artifact.
    pub fn artifact_assess(
        &mut self,
        artifact_id: &OpaqueId,
        expected_revision: u64,
        assessment: EvidenceAssessment,
    ) -> Result<ResearchArtifact, AuthorityError> {
        assessment.validate().map_err(invalid)?;
        self.artifact_change(
            artifact_id,
            expected_revision,
            PackAction::Assess,
            |_, a| {
                a.assessments.push(assessment);
                Ok(())
            },
        )
    }

    /// Runs one act.
    pub fn pack_act(&mut self, act: PackActRequest) -> Result<PackActResult, AuthorityError> {
        let install = |i| PackActResult {
            install: Some(i),
            artifact: None,
        };
        let artifact = |a| PackActResult {
            install: None,
            artifact: Some(a),
        };
        Ok(match act {
            PackActRequest::Install {
                project_id,
                pack_id,
            } => install(self.pack_install(&project_id, &pack_id)?),
            PackActRequest::Upgrade {
                project_id,
                pack_id,
            } => install(self.pack_upgrade(&project_id, &pack_id)?),
            PackActRequest::Uninstall {
                project_id,
                pack_id,
            } => install(self.pack_uninstall(&project_id, &pack_id)?),
            PackActRequest::Create {
                project_id,
                pack_id,
                type_id,
                fields,
            } => artifact(self.artifact_create(&project_id, &pack_id, &type_id, fields)?),
            PackActRequest::Update {
                artifact_id,
                expected_revision,
                fields,
            } => artifact(self.artifact_update(&artifact_id, expected_revision, fields)?),
            PackActRequest::Transition {
                artifact_id,
                expected_revision,
                to,
            } => artifact(self.artifact_transition(&artifact_id, expected_revision, &to)?),
            PackActRequest::Assess {
                artifact_id,
                expected_revision,
                assessment,
            } => artifact(self.artifact_assess(&artifact_id, expected_revision, assessment)?),
        })
    }

    pub fn artifact_get(&self, artifact_id: &OpaqueId) -> Result<ResearchArtifact, AuthorityError> {
        self.rp_artifact(artifact_id)
    }

    pub fn artifact_list(
        &self,
        project_id: &OpaqueId,
        pack_id: &str,
    ) -> Result<Vec<ResearchArtifact>, AuthorityError> {
        self.require_project(project_id)?;
        Ok(self
            .meta()
            .list_research_artifacts(Some((project_id, pack_id)))
            .map_err(meta_err)?
            .into_iter()
            .filter(|a| {
                a.header.realm_id == self.realm && a.header.authority_scope_id == self.scope
            })
            .collect())
    }

    pub fn pack_install_get(
        &self,
        project_id: &OpaqueId,
        pack_id: &str,
    ) -> Result<ResearchPackInstall, AuthorityError> {
        self.require_project(project_id)?;
        self.rp_install_of(project_id, pack_id)?
            .ok_or(AuthorityError::NotFound)
    }
}
