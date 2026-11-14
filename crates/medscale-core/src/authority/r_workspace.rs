//! R Workspace Core authority paths (Spec 086, foundation slice).
//!
//! Core stages exact Spec 075 snapshots (digest-verified on load) as CSV
//! copies into a new directory under the host's staging directory, which
//! must lie outside the vault. The workspace receives copies only: no
//! vault path, database handle, key or credential. External programs are
//! started by the host launcher; managed `Rscript` runs are recorded and
//! refused while the OS sandbox is not platform qualified. Outputs come
//! back only through `r_publish`: one named regular file, read with
//! bounds and link checks, typed by the Spec 075 CSV parser, pinned by
//! digest and committed with its receipt in one transaction as a derived,
//! unreviewed table carrying the workspace's class.

use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::{DigestSha256, ObjectHeader, OpaqueId};
use medscale_contracts::privacy_gate::DataClass;
use medscale_contracts::r_workspace::{
    ClassBasis, DATA_DIR, DESCRIPTOR_FILE, EVIDENCE_FILE_BYTES_MAX, IdeKind, LaunchState,
    OUTPUT_ENTRIES_MAX, OUTPUTS_DIR, OutputCandidate, OutputPolicy, PUBLISH_BYTES_MAX,
    PublishRefusal, PublishState, R_WORKSPACE_LAYOUT_VERSION, R_WORKSPACE_SCHEMA_VERSION,
    README_FILE, RENV_LOCK_FILE, RLaunchReceipt, RLaunchRequest, RLockfileRef, RPublishReceipt,
    RPublishRequest, RPublishView, RPublishedTable, RRunReceipt, RRunRequest, RStageRequest,
    RStagedInput, RWorkspace, RWorkspaceHistory, RWorkspaceManifest, RWorkspaceStatus, RunRefusal,
    SCRIPTS_DIR, STAGE_BYTES_MAX, STAGE_ROOT_MAX_CHARS, StagedFile, StagingMode,
    WORKSPACE_INPUTS_MAX, WorkspaceInspection, WorkspaceIntegrity, bounded_name, data_file_name,
    most_restrictive, render_csv, render_readme, render_rproj, render_schema, schema_file_name,
    table_doc, validate_label, validate_output_name, validate_script_name,
};
use medscale_storage::MetaError;

use super::data_acquire::parse_delimited;
use super::data_sources::DataSources;
use crate::r_workspace_host::{RWorkspaceHost, launch};

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

fn io_unavailable(e: &std::io::Error) -> AuthorityError {
    AuthorityError::Unavailable {
        message: format!("R workspace staging failed: {}", e.kind()),
    }
}

/// Why a file in a workspace could not be read as admitted bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileProblem {
    NotFound,
    /// A link, directory or other non-regular file.
    NotRegular,
    TooLarge,
    /// The file changed identity or length while it was read.
    Changed,
}

/// `lstat` says a real directory (not a link or junction).
fn real_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_dir())
}

#[cfg(unix)]
fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    a.dev() == b.dev() && a.ino() == b.ino()
}

#[cfg(not(unix))]
fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    a.len() == b.len()
}

/// Reads one regular file with a byte bound. Links are refused before
/// opening; on Unix the opened file must be the one `lstat` saw (device
/// and inode), and the path is re-checked after reading.
fn read_regular(path: &Path, max: u64) -> Result<Vec<u8>, FileProblem> {
    let before = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(FileProblem::NotFound),
        Err(_) => return Err(FileProblem::NotRegular),
    };
    if !before.file_type().is_file() {
        return Err(FileProblem::NotRegular);
    }
    if before.len() > max {
        return Err(FileProblem::TooLarge);
    }
    let file = fs::File::open(path).map_err(|_| FileProblem::Changed)?;
    let opened = file.metadata().map_err(|_| FileProblem::Changed)?;
    if !opened.is_file() || !same_file(&before, &opened) || opened.len() != before.len() {
        return Err(FileProblem::Changed);
    }
    let mut bytes = Vec::new();
    file.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| FileProblem::Changed)?;
    if bytes.len() as u64 > max {
        return Err(FileProblem::TooLarge);
    }
    let after = fs::symlink_metadata(path).map_err(|_| FileProblem::Changed)?;
    if !after.file_type().is_file()
        || !same_file(&before, &after)
        || bytes.len() as u64 != before.len()
        || after.len() != before.len()
    {
        return Err(FileProblem::Changed);
    }
    Ok(bytes)
}

fn evidence(path: &Path) -> Option<RLockfileRef> {
    read_regular(path, EVIDENCE_FILE_BYTES_MAX)
        .ok()
        .map(|bytes| RLockfileRef {
            digest: DigestSha256::of(&bytes),
            bytes: bytes.len() as u64,
        })
}

/// Writes a new file; an existing path (including a link) is an error.
fn write_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn set_read_only(path: &Path) -> std::io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)
}

/// Writes every generated file into `partial`, marks the data copies
/// read-only, and renames the directory to `final_root`.
fn write_workspace(
    partial: &Path,
    final_root: &Path,
    rendered: &[(String, Vec<u8>)],
    descriptor: &[u8],
) -> std::io::Result<()> {
    fs::create_dir(partial)?;
    for sub in [DATA_DIR, SCRIPTS_DIR, OUTPUTS_DIR] {
        fs::create_dir(partial.join(sub))?;
    }
    for (path, bytes) in rendered {
        write_new(&partial.join(path), bytes)?;
    }
    write_new(&partial.join(DESCRIPTOR_FILE), descriptor)?;
    for (path, _) in rendered {
        if path.starts_with(DATA_DIR) {
            set_read_only(&partial.join(path))?;
        }
    }
    fs::rename(partial, final_root)
}

/// Drops Windows' `\\?\` prefix from a canonical local drive path, so
/// the recorded path is the one users and IDEs recognise. UNC and other
/// verbatim forms are kept as they are.
fn plain_path(path: PathBuf) -> PathBuf {
    let stripped = path
        .to_str()
        .and_then(|t| t.strip_prefix(r"\\?\"))
        .filter(|rest| rest.as_bytes().get(1) == Some(&b':'))
        .map(PathBuf::from);
    stripped.unwrap_or(path)
}

fn workspace_root(ws: &RWorkspace) -> PathBuf {
    PathBuf::from(&ws.manifest.stage_root)
}

/// Checks the staged directory against the manifest.
fn integrity(ws: &RWorkspace) -> WorkspaceIntegrity {
    let root = workspace_root(ws);
    if !real_dir(&root) {
        return WorkspaceIntegrity::Missing;
    }
    if !real_dir(&root.join(DATA_DIR)) {
        return WorkspaceIntegrity::Changed;
    }
    let descriptor_ok = read_regular(
        &root.join(DESCRIPTOR_FILE),
        ws.manifest.canonical_bytes().len() as u64,
    )
    .is_ok_and(|b| DigestSha256::of(&b) == ws.descriptor_digest);
    let files_ok = ws.manifest.files.iter().all(|f| {
        read_regular(&root.join(&f.path), f.bytes)
            .is_ok_and(|b| b.len() as u64 == f.bytes && DigestSha256::of(&b) == f.digest)
    });
    if descriptor_ok && files_ok {
        WorkspaceIntegrity::Intact
    } else {
        WorkspaceIntegrity::Changed
    }
}

/// Publishable files directly in `outputs/`, sorted by name, and the count
/// of entries that are not publishable.
fn output_candidates(root: &Path) -> (Vec<OutputCandidate>, u64) {
    let dir = root.join(OUTPUTS_DIR);
    if !real_dir(&dir) {
        return (Vec::new(), 0);
    }
    let Ok(entries) = fs::read_dir(&dir) else {
        return (Vec::new(), 0);
    };
    let mut candidates = Vec::new();
    let mut skipped = 0_u64;
    for (seen, entry) in entries.enumerate() {
        let Ok(entry) = entry else {
            skipped += 1;
            continue;
        };
        if seen >= OUTPUT_ENTRIES_MAX {
            skipped += 1;
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str().filter(|n| validate_output_name(n).is_ok()) else {
            skipped += 1;
            continue;
        };
        match read_regular(&dir.join(name), PUBLISH_BYTES_MAX) {
            Ok(bytes) => candidates.push(OutputCandidate {
                name: name.to_owned(),
                bytes: bytes.len() as u64,
                digest: DigestSha256::of(&bytes),
            }),
            Err(_) => skipped += 1,
        }
    }
    candidates.sort_by(|a, b| a.name.cmp(&b.name));
    (candidates, skipped)
}

impl DataSources<'_> {
    fn rws_header(&self, id: OpaqueId) -> ObjectHeader {
        ObjectHeader {
            id,
            schema_version: R_WORKSPACE_SCHEMA_VERSION,
            realm_id: self.realm.clone(),
            authority_scope_id: self.scope.clone(),
        }
    }

    fn scoped_r_workspace(&self, id: &OpaqueId) -> Result<RWorkspace, AuthorityError> {
        let ws = self.meta().get_r_workspace(id).map_err(meta_err)?;
        let h = &ws.manifest.header;
        if h.realm_id != self.realm || h.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(ws)
    }

    /// Every input is still the Project's snapshot with its pinned bytes.
    fn r_inputs_current(&self, ws: &RWorkspace) -> bool {
        ws.manifest.inputs.iter().all(|input| {
            self.scoped_snapshot(&input.snapshot_id).is_ok_and(|r| {
                r.project_id == ws.manifest.project_id
                    && r.snapshot.content_digest == input.content_digest
                    && r.snapshot.schema_fingerprint == input.schema_fingerprint
            })
        })
    }

    fn r_input_class(&self, project_id: &OpaqueId, snapshot_id: &OpaqueId) -> DataClass {
        match self.meta().get_classification(project_id, snapshot_id) {
            Ok(Some(row)) => row.data_class,
            // No row (or an unreadable one) is `local_phi`: fail closed.
            _ => DataClass::LocalPhi,
        }
    }

    /// Stages exact snapshots into a new workspace directory.
    pub fn r_stage(
        &mut self,
        host: &RWorkspaceHost,
        request: RStageRequest,
    ) -> Result<RWorkspace, AuthorityError> {
        self.require_project(&request.project_id)?;
        validate_label(&request.label).map_err(|e| invalid(&e))?;
        if request.snapshot_ids.is_empty() || request.snapshot_ids.len() > WORKSPACE_INPUTS_MAX {
            return Err(invalid("a workspace stages 1-16 snapshots"));
        }
        let mut distinct = std::collections::BTreeSet::new();
        if !request
            .snapshot_ids
            .iter()
            .all(|s| distinct.insert(s.as_str()))
        {
            return Err(invalid("duplicate snapshot"));
        }
        let Some(stage_dir) = host.stage_dir.as_ref() else {
            return Err(AuthorityError::Unavailable {
                message: "no R workspace staging directory is configured".to_owned(),
            });
        };
        let stage_dir = fs::canonicalize(stage_dir)
            .ok()
            .filter(|p| p.is_dir())
            .ok_or_else(|| AuthorityError::Unavailable {
                message: "the R workspace staging directory does not exist".to_owned(),
            })?;
        let vault = fs::canonicalize(self.vault_dir()).map_err(|e| io_unavailable(&e))?;
        if stage_dir.starts_with(&vault) {
            return Err(AuthorityError::PathOutsideClaim);
        }
        let stage_dir = plain_path(stage_dir);

        // Load and render every input before touching the filesystem.
        let mut inputs = Vec::new();
        let mut rendered: Vec<(String, Vec<u8>)> = Vec::new();
        let mut staged_bytes = 0_u64;
        for snapshot_id in &request.snapshot_ids {
            let record = self
                .scoped_snapshot(snapshot_id)
                .ok()
                .filter(|r| r.project_id == request.project_id)
                .ok_or(AuthorityError::NotFound)?;
            let doc = self.load_table(&record)?;
            let input = RStagedInput {
                snapshot_id: record.snapshot.header.id.clone(),
                content_digest: record.snapshot.content_digest.clone(),
                schema_fingerprint: record.snapshot.schema_fingerprint.clone(),
                row_count: record.snapshot.row_count,
                data_class: self.r_input_class(&request.project_id, snapshot_id),
                data_file: data_file_name(snapshot_id),
                schema_file: schema_file_name(snapshot_id),
            };
            let csv = render_csv(&doc.fields, &doc.rows);
            staged_bytes += csv.len() as u64;
            if staged_bytes > STAGE_BYTES_MAX {
                return Err(invalid("inputs exceed the staging byte bound"));
            }
            rendered.push((
                input.schema_file.clone(),
                render_schema(&input, &doc.fields),
            ));
            rendered.push((input.data_file.clone(), csv));
            inputs.push(input);
        }
        let data_class = most_restrictive(inputs.iter().map(|i| i.data_class));
        rendered.push((
            README_FILE.to_owned(),
            render_readme(&request.label, data_class, &inputs),
        ));
        rendered.push((format!("{}.Rproj", request.label), render_rproj()));
        rendered.sort_by(|a, b| a.0.cmp(&b.0));

        let workspace_id = self.meta().alloc_rws_id("r-workspace").map_err(meta_err)?;
        let dir_name = format!("{}-{}", request.label, workspace_id.as_str());
        let final_root = stage_dir.join(&dir_name);
        let stage_root = final_root
            .to_str()
            .filter(|s| s.chars().count() <= STAGE_ROOT_MAX_CHARS)
            .ok_or_else(|| invalid("staging path is not UTF-8 or too long"))?
            .to_owned();
        let manifest = RWorkspaceManifest {
            header: self.rws_header(workspace_id.clone()),
            project_id: request.project_id.clone(),
            label: request.label.clone(),
            layout_version: R_WORKSPACE_LAYOUT_VERSION,
            stage_root,
            staging_mode: StagingMode::CsvCopy,
            output_policy: OutputPolicy::ExplicitPublishOnly,
            data_class,
            class_basis: ClassBasis::InheritedFromInputs,
            inputs,
            files: rendered
                .iter()
                .map(|(path, bytes)| StagedFile {
                    path: path.clone(),
                    digest: DigestSha256::of(bytes),
                    bytes: bytes.len() as u64,
                })
                .collect(),
        };
        manifest.validate().map_err(|e| AuthorityError::Internal {
            message: format!("R workspace invariant: {e}"),
        })?;
        let descriptor = manifest.canonical_bytes();
        let workspace = RWorkspace {
            descriptor_digest: DigestSha256::of(&descriptor),
            manifest,
        };

        // Write into a private partial directory, then rename it into
        // place: a crash leaves only a `.partial` directory, never a
        // workspace row without its files.
        let partial = stage_dir.join(format!(".{dir_name}.partial"));
        if let Err(e) = write_workspace(&partial, &final_root, &rendered, &descriptor) {
            let _ = fs::remove_dir_all(&partial);
            return Err(io_unavailable(&e));
        }
        if let Err(e) = self.meta().insert_r_workspace(&workspace) {
            let _ = fs::remove_dir_all(&final_root);
            return Err(meta_err(e));
        }
        self.audit("r_workspace.stage", vec![workspace_id])?;
        Ok(workspace)
    }

    /// Read-only view: integrity, input currency, lockfile evidence and
    /// publishable outputs.
    pub fn r_inspect(
        &self,
        workspace_id: &OpaqueId,
    ) -> Result<WorkspaceInspection, AuthorityError> {
        let ws = self.scoped_r_workspace(workspace_id)?;
        let state = integrity(&ws);
        let root = workspace_root(&ws);
        let (candidates, skipped, lockfile) = if state == WorkspaceIntegrity::Missing {
            (Vec::new(), 0, None)
        } else {
            let (c, s) = output_candidates(&root);
            (c, s, evidence(&root.join(RENV_LOCK_FILE)))
        };
        Ok(WorkspaceInspection {
            workspace_id: workspace_id.clone(),
            integrity: state,
            inputs_current: self.r_inputs_current(&ws),
            lockfile,
            candidates,
            skipped,
        })
    }

    /// Opens the workspace in a host-configured external program.
    pub fn r_launch(
        &mut self,
        host: &RWorkspaceHost,
        request: RLaunchRequest,
    ) -> Result<RLaunchReceipt, AuthorityError> {
        let ws = self.scoped_r_workspace(&request.workspace_id)?;
        let program = host.programs.get(&request.ide);
        let mut env_names = Vec::new();
        let state = match program {
            None => LaunchState::IdeNotConfigured,
            Some(p) if !(p.is_absolute() && p.is_file()) => LaunchState::IdeNotFound,
            Some(p) => match integrity(&ws) {
                WorkspaceIntegrity::Missing => LaunchState::WorkspaceMissing,
                WorkspaceIntegrity::Changed => LaunchState::WorkspaceChanged,
                WorkspaceIntegrity::Intact => match launch(p, &workspace_root(&ws)) {
                    Ok(names) => {
                        env_names = names;
                        LaunchState::Launched
                    }
                    Err(_) => LaunchState::SpawnFailed,
                },
            },
        };
        let receipt = RLaunchReceipt {
            header: self.rws_header(self.meta().alloc_rws_id("r-launch").map_err(meta_err)?),
            workspace_id: ws.id().clone(),
            project_id: ws.manifest.project_id.clone(),
            ide: request.ide,
            program: program.map(|p| p.to_string_lossy().into_owned()),
            descriptor_digest: ws.descriptor_digest.clone(),
            env_names,
            state,
        };
        self.meta()
            .insert_r_launch_receipt(&receipt)
            .map_err(meta_err)?;
        self.audit(
            "r_workspace.launch",
            vec![ws.id().clone(), receipt.header.id.clone()],
        )?;
        Ok(receipt)
    }

    /// Records a managed run request and refuses it: user R code is
    /// arbitrary code and nothing here executes it.
    pub fn r_run(
        &mut self,
        host: &RWorkspaceHost,
        request: RRunRequest,
    ) -> Result<RRunReceipt, AuthorityError> {
        let ws = self.scoped_r_workspace(&request.workspace_id)?;
        let root = workspace_root(&ws);
        let state = integrity(&ws);
        let mut script_digest = None;
        let refusal = if state == WorkspaceIntegrity::Missing {
            RunRefusal::WorkspaceMissing
        } else if state == WorkspaceIntegrity::Changed {
            RunRefusal::WorkspaceChanged
        } else if !self.r_inputs_current(&ws) {
            RunRefusal::InputStale
        } else if validate_script_name(&request.script).is_err()
            || !real_dir(&root.join(SCRIPTS_DIR))
        {
            RunRefusal::ScriptInvalid
        } else {
            match read_regular(
                &root.join(SCRIPTS_DIR).join(&request.script),
                EVIDENCE_FILE_BYTES_MAX,
            ) {
                Ok(bytes) => {
                    script_digest = Some(DigestSha256::of(&bytes));
                    RunRefusal::ComputeDeniedPlatformUnqualified
                }
                Err(_) => RunRefusal::ScriptInvalid,
            }
        };
        let lockfile = if state == WorkspaceIntegrity::Missing {
            None
        } else {
            evidence(&root.join(RENV_LOCK_FILE))
        };
        let receipt = RRunReceipt {
            header: self.rws_header(self.meta().alloc_rws_id("r-run").map_err(meta_err)?),
            workspace_id: ws.id().clone(),
            project_id: ws.manifest.project_id.clone(),
            descriptor_digest: ws.descriptor_digest.clone(),
            script: bounded_name(&request.script),
            script_digest,
            runtime: host.runtime_identity(),
            lockfile,
            refusal,
        };
        self.meta()
            .insert_r_run_receipt(&receipt)
            .map_err(meta_err)?;
        self.audit(
            "r_workspace.run_refused",
            vec![ws.id().clone(), receipt.header.id.clone()],
        )?;
        Ok(receipt)
    }

    /// Admits one output file as a derived, unreviewed table. Every
    /// attempt leaves a receipt.
    pub fn r_publish(&mut self, request: RPublishRequest) -> Result<RPublishView, AuthorityError> {
        let ws = self.scoped_r_workspace(&request.workspace_id)?;
        let root = workspace_root(&ws);
        let state = integrity(&ws);
        let mut source_digest = None;
        let mut parsed = None;
        let refusal = if state == WorkspaceIntegrity::Missing {
            Some(PublishRefusal::WorkspaceMissing)
        } else if state == WorkspaceIntegrity::Changed {
            Some(PublishRefusal::WorkspaceChanged)
        } else if !self.r_inputs_current(&ws) {
            Some(PublishRefusal::InputStale)
        } else if validate_output_name(&request.output_name).is_err() {
            Some(PublishRefusal::BadName)
        } else if !real_dir(&root.join(OUTPUTS_DIR)) {
            Some(PublishRefusal::NotRegularFile)
        } else {
            match read_regular(
                &root.join(OUTPUTS_DIR).join(&request.output_name),
                PUBLISH_BYTES_MAX,
            ) {
                Err(FileProblem::NotFound) => Some(PublishRefusal::NotFound),
                Err(FileProblem::NotRegular) => Some(PublishRefusal::NotRegularFile),
                Err(FileProblem::TooLarge) => Some(PublishRefusal::TooLarge),
                Err(FileProblem::Changed) => Some(PublishRefusal::OutputChanged),
                Ok(bytes) => {
                    let digest = DigestSha256::of(&bytes);
                    source_digest = Some(digest.clone());
                    if request
                        .expected_digest
                        .as_ref()
                        .is_some_and(|d| *d != digest)
                    {
                        Some(PublishRefusal::OutputChanged)
                    } else {
                        match parse_delimited(&bytes, b',') {
                            Ok(table) if table.rows_skipped == 0 => {
                                parsed = Some(table_doc(&table.fields, table.rows));
                                None
                            }
                            // Ragged rows would be dropped: not exact.
                            _ => Some(PublishRefusal::OutputInvalid),
                        }
                    }
                }
            }
        };
        let receipt_id = self.meta().alloc_rws_id("r-publish").map_err(meta_err)?;
        let lockfile = if state == WorkspaceIntegrity::Missing {
            None
        } else {
            evidence(&root.join(RENV_LOCK_FILE))
        };
        let (table, content) = match (&refusal, parsed, &source_digest) {
            (None, Some(doc), Some(source)) => {
                let table_id = self.meta().alloc_rws_id("r-table").map_err(meta_err)?;
                let table = RPublishedTable {
                    header: self.rws_header(table_id),
                    project_id: ws.manifest.project_id.clone(),
                    workspace_id: ws.id().clone(),
                    receipt_id: receipt_id.clone(),
                    output_name: request.output_name.clone(),
                    source_digest: source.clone(),
                    content_digest: doc.digest(),
                    row_count: doc.rows.len() as u64,
                    column_count: u32::try_from(doc.columns.len()).unwrap_or(u32::MAX),
                    derived_from: ws
                        .manifest
                        .inputs
                        .iter()
                        .map(|i| i.snapshot_id.clone())
                        .collect(),
                    data_class: ws.manifest.data_class,
                    reviewed: false,
                };
                (Some(table), Some(doc))
            }
            _ => (None, None),
        };
        let receipt = RPublishReceipt {
            header: self.rws_header(receipt_id),
            workspace_id: ws.id().clone(),
            project_id: ws.manifest.project_id.clone(),
            descriptor_digest: ws.descriptor_digest.clone(),
            output_name: bounded_name(&request.output_name),
            source_digest,
            lockfile,
            state: if table.is_some() {
                PublishState::Published
            } else {
                PublishState::Refused
            },
            refusal,
            table_id: table.as_ref().map(|t| t.header.id.clone()),
            table_digest: table.as_ref().map(|t| t.content_digest.clone()),
            row_count: table.as_ref().map_or(0, |t| t.row_count),
            column_count: table.as_ref().map_or(0, |t| t.column_count),
        };
        receipt.validate().map_err(|e| AuthorityError::Internal {
            message: format!("R publish receipt invariant: {e}"),
        })?;
        self.meta()
            .insert_r_publication(&receipt, table.as_ref().zip(content.as_ref()))
            .map_err(meta_err)?;
        let mut targets = vec![ws.id().clone(), receipt.header.id.clone()];
        if let Some(t) = &table {
            targets.push(t.header.id.clone());
        }
        self.audit("r_workspace.publish", targets)?;
        Ok(RPublishView {
            receipt,
            table,
            content,
        })
    }

    /// A workspace with every receipt recorded for it.
    pub fn r_workspace(
        &self,
        workspace_id: &OpaqueId,
    ) -> Result<RWorkspaceHistory, AuthorityError> {
        let workspace = self.scoped_r_workspace(workspace_id)?;
        let id = Some(workspace_id);
        Ok(RWorkspaceHistory {
            launches: self.meta().list_r_launch_receipts(id).map_err(meta_err)?,
            runs: self.meta().list_r_run_receipts(id).map_err(meta_err)?,
            publications: self.meta().list_r_publish_receipts(id).map_err(meta_err)?,
            workspace,
        })
    }

    pub fn r_workspaces(&self, project_id: &OpaqueId) -> Result<Vec<RWorkspace>, AuthorityError> {
        self.require_project(project_id)?;
        Ok(self
            .meta()
            .list_r_workspaces(project_id)
            .map_err(meta_err)?
            .into_iter()
            .filter(|w| {
                w.manifest.header.realm_id == self.realm
                    && w.manifest.header.authority_scope_id == self.scope
            })
            .collect())
    }

    /// A published table with its receipt and digest-checked content.
    pub fn r_published(&self, table_id: &OpaqueId) -> Result<RPublishView, AuthorityError> {
        let (table, content) = self
            .meta()
            .get_r_published_table(table_id)
            .map_err(meta_err)?;
        if table.header.realm_id != self.realm || table.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        let receipt = self
            .meta()
            .list_r_publish_receipts(Some(&table.workspace_id))
            .map_err(meta_err)?
            .into_iter()
            .find(|r| r.header.id == table.receipt_id)
            .ok_or_else(|| AuthorityError::Corrupt {
                message: "published R table without its receipt".to_owned(),
            })?;
        Ok(RPublishView {
            receipt,
            table: Some(table),
            content: Some(content),
        })
    }

    pub fn r_status(&self, host: &RWorkspaceHost) -> RWorkspaceStatus {
        RWorkspaceStatus {
            stage_dir_configured: host.stage_dir.as_ref().is_some_and(|p| p.is_dir()),
            ides: IdeKind::ALL
                .iter()
                .map(|k| {
                    (
                        *k,
                        host.programs
                            .get(k)
                            .is_some_and(|p| p.is_absolute() && p.is_file()),
                    )
                })
                .collect(),
            runtime: host.runtime_identity(),
            layout_version: R_WORKSPACE_LAYOUT_VERSION,
            staging_mode: StagingMode::CsvCopy,
            managed_run_admitted: false,
            platform_qualified: false,
        }
    }
}
