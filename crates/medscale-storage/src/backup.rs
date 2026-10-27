//! Backup / restore for synthetic unencrypted vaults (schema v1 sources-only + v2 objects).

use std::fs;
use std::path::Path;

use medscale_contracts::ingest::{BackupManifest, BlobRef};
use medscale_contracts::objects::DigestSha256;

use crate::blob::FsBlobStore;
use crate::claim::assert_claim_path;
use crate::sqlite_meta::{AuthorityObjectRow, SourceMeta, SqliteMetaStore};
use crate::vault::SyntheticVault;

/// Create a backup artifact directory.
pub fn backup_vault(vault: &SyntheticVault, dest: &Path) -> Result<BackupManifest, String> {
    let dest = assert_claim_path(dest).map_err(|e| e.to_string())?;
    fs::create_dir_all(dest.join("blobs")).map_err(|e| e.to_string())?;
    let snapshot = vault.meta.snapshot_bytes().map_err(|e| e.to_string())?;
    let snapshot_digest = DigestSha256::of(&snapshot);
    fs::write(dest.join("metadata.snapshot"), &snapshot).map_err(|e| e.to_string())?;

    let mut blob_entries = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();
    for source in vault.meta.list_sources().map_err(|e| e.to_string())? {
        let bytes = vault
            .blobs
            .get_blob(&source.digest)
            .map_err(|e| e.to_string())?;
        let name = hex(&source.digest);
        if seen.insert(name.clone()) {
            fs::write(dest.join("blobs").join(&name), &bytes).map_err(|e| e.to_string())?;
            blob_entries.push(BlobRef {
                digest: source.digest,
                byte_length: source.byte_length,
            });
        }
    }
    for obj in vault
        .meta
        .list_authority_objects()
        .map_err(|e| e.to_string())?
    {
        if let Some(hex_d) = obj.content_digest_hex.as_ref()
            && seen.insert(hex_d.clone())
        {
            let digest = parse_hex(hex_d);
            let bytes = vault.blobs.get_blob(&digest).map_err(|e| e.to_string())?;
            fs::write(dest.join("blobs").join(hex_d), &bytes).map_err(|e| e.to_string())?;
            blob_entries.push(BlobRef {
                digest,
                byte_length: bytes.len() as u64,
            });
        }
    }

    let manifest = BackupManifest {
        schema_version: crate::CURRENT_META_SCHEMA_VERSION,
        vault_id: vault.vault_id.clone(),
        created_at: "1970-01-01T00:00:00Z".to_owned(),
        metadata_snapshot_digest: snapshot_digest,
        blob_entries,
        claim_scope_note: "synthetic-unencrypted-h0a".to_owned(),
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(dest.join("manifest.json"), manifest_bytes).map_err(|e| e.to_string())?;
    Ok(manifest)
}

/// Restore a backup into an empty destination vault root.
pub fn restore_vault(src: &Path, dest_vault_root: &Path) -> Result<(u64, u64), String> {
    let src = assert_claim_path(src).map_err(|e| e.to_string())?;
    let dest_vault_root = assert_claim_path(dest_vault_root).map_err(|e| e.to_string())?;
    if dest_vault_root.exists()
        && dest_vault_root
            .read_dir()
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    {
        return Err("restore destination not empty".to_owned());
    }
    let manifest_bytes = fs::read(src.join("manifest.json")).map_err(|e| e.to_string())?;
    let manifest: BackupManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
    let snapshot = fs::read(src.join("metadata.snapshot")).map_err(|e| e.to_string())?;
    if DigestSha256::of(&snapshot) != manifest.metadata_snapshot_digest {
        return Err("tampered metadata snapshot".to_owned());
    }

    let vault =
        SyntheticVault::open(&manifest.vault_id, &dest_vault_root).map_err(|e| e.to_string())?;

    let mut sources = 0_u64;
    let mut blobs = 0_u64;
    for entry in &manifest.blob_entries {
        let name = hex(&entry.digest);
        let bytes = fs::read(src.join("blobs").join(&name)).map_err(|e| e.to_string())?;
        if DigestSha256::of(&bytes) != entry.digest {
            return Err("tampered blob".to_owned());
        }
        vault.blobs.put_blob(&bytes).map_err(|e| e.to_string())?;
        blobs += 1;
    }

    let snapshot_value: serde_json::Value =
        serde_json::from_slice(&snapshot).map_err(|e| e.to_string())?;
    let snapshot_schema = snapshot_value
        .get("schema_version")
        .and_then(|v| v.as_u64());
    if snapshot_schema == Some(21) {
        restore_v21(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(20) {
        restore_v20(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(19) {
        restore_v19(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(18) {
        restore_v18(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(17) {
        restore_v17(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(16) {
        restore_v16(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(15) {
        restore_v15(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(14) {
        restore_v14(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(13) {
        restore_v13(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(12) {
        restore_v12(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(11) {
        restore_v11(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(10) {
        restore_v10(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(9) {
        restore_v9(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(8) {
        restore_v8(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(7) {
        restore_v7(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(6) {
        restore_v6(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(5) {
        restore_v5(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(4) {
        restore_v4(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(3) {
        restore_v3(&vault, &snapshot_value, &mut sources)?;
    } else if snapshot_schema == Some(2) {
        restore_v2(&vault, &snapshot_value, &mut sources)?;
    } else {
        let entries: Vec<serde_json::Value> =
            serde_json::from_slice(&snapshot).map_err(|e| e.to_string())?;
        for value in entries {
            insert_source_from_json(&vault, &value)?;
            sources += 1;
        }
    }
    // Id sequences (backups written since the fix): restored last, so new
    // ids never collide with restored rows. Older backups have none.
    if let Some(value) = snapshot_value.get("id_sequences") {
        let sequences: std::collections::BTreeMap<String, u64> =
            serde_json::from_value(value.clone())
                .map_err(|e| format!("tampered metadata snapshot: id_sequences: {e}"))?;
        vault
            .meta
            .restore_id_sequences(&sequences)
            .map_err(|e| e.to_string())?;
    }
    let _ = FsBlobStore::open;
    let _ = SqliteMetaStore::open;
    Ok((sources, blobs))
}

fn restore_v2(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    if let Some(entries) = snapshot.get("sources").and_then(|v| v.as_array()) {
        for value in entries {
            insert_source_from_json(vault, value)?;
            *sources += 1;
        }
    }
    let mut rows = Vec::new();
    if let Some(objects) = snapshot.get("objects").and_then(|v| v.as_array()) {
        for value in objects {
            rows.push(AuthorityObjectRow {
                object_id: value["object_id"].as_str().unwrap_or_default().to_owned(),
                object_class: value["object_class"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                realm_id: value["realm_id"].as_str().unwrap_or_default().to_owned(),
                authority_scope_id: value["authority_scope_id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                body_json: value["body_json"].as_str().unwrap_or_default().to_owned(),
                content_digest_hex: value
                    .get("content_digest_hex")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned),
                updated_seq: value["updated_seq"].as_u64().unwrap_or(0),
            });
        }
    }
    let next_seq = snapshot
        .get("next_seq")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    vault
        .meta
        .replace_authority_snapshot(&rows, next_seq)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v3(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v2(vault, snapshot, sources)?;
    // Spec 074 rows replay exactly (ids/revisions preserved); every row is
    // re-validated so a tampered snapshot fails closed instead of persisting.
    if let Some(projects) = snapshot.get("projects").and_then(|v| v.as_array()) {
        for value in projects {
            let project: medscale_contracts::project_graph::Project =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            medscale_contracts::project_graph::validate_metadata_fields(
                &project.name,
                project.description.as_deref(),
            )
            .map_err(|e| e.to_string())?;
            if project.revision < 1 {
                return Err("tampered project revision".to_owned());
            }
            vault
                .meta
                .restore_project_row(&project)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(experiments) = snapshot.get("experiments").and_then(|v| v.as_array()) {
        for value in experiments {
            let experiment: medscale_contracts::project_graph::Experiment =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            medscale_contracts::project_graph::validate_metadata_fields(
                &experiment.name,
                experiment.description.as_deref(),
            )
            .map_err(|e| e.to_string())?;
            if experiment.revision < 1 {
                return Err("tampered experiment revision".to_owned());
            }
            vault
                .meta
                .restore_experiment_row(&experiment)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(refs) = snapshot.get("refs").and_then(|v| v.as_array()) {
        for value in refs {
            let reference: medscale_contracts::project_graph::ProjectArtifactRef =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            reference.artifact.validate().map_err(|e| e.to_string())?;
            if reference.revision < 1 {
                return Err("tampered ref revision".to_owned());
            }
            vault
                .meta
                .restore_ref_row(&reference)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(edges) = snapshot.get("edges").and_then(|v| v.as_array()) {
        for value in edges {
            let edge: medscale_contracts::project_graph::ProjectGraphEdge =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            edge.subject.validate().map_err(|e| e.to_string())?;
            edge.object.validate().map_err(|e| e.to_string())?;
            if edge.revision < 1 {
                return Err("tampered edge revision".to_owned());
            }
            vault
                .meta
                .restore_edge_row(&edge)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn restore_v4(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v3(vault, snapshot, sources)?;
    // Spec 075 rows replay exactly (ids/revisions/digests preserved); every
    // row is re-validated so a tampered snapshot fails closed instead of
    // persisting. Snapshots restore before releases so release scope
    // inheritance resolves.
    if let Some(entries) = snapshot.get("data_sources").and_then(|v| v.as_array()) {
        for value in entries {
            let manifest: medscale_contracts::data_sources::DataSourceManifest =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            medscale_contracts::data_sources::validate_display_name(&manifest.display_name)
                .map_err(|e| e.to_string())?;
            manifest.locator.validate().map_err(|e| e.to_string())?;
            if matches!(
                manifest.locator,
                medscale_contracts::data_sources::SourceLocator::Database { .. }
            ) && manifest.credential_ref.is_some()
            {
                return Err(
                    "tampered source: database credential references are not admitted in 075"
                        .to_owned(),
                );
            }
            if manifest.capabilities.is_empty() {
                return Err("tampered source capabilities".to_owned());
            }
            if manifest.revision < 1 {
                return Err("tampered source revision".to_owned());
            }
            vault
                .meta
                .restore_data_source_row(&manifest)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot.get("data_snapshots").and_then(|v| v.as_array()) {
        let parts = snapshot
            .get("data_snapshot_parts")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for value in entries {
            let record: crate::data_sources::SnapshotRecord =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            record.snapshot.validate().map_err(|e| e.to_string())?;
            record.schema.validate().map_err(|e| e.to_string())?;
            let id = record.snapshot.header.id.as_str().to_owned();
            let mut owned_parts = Vec::new();
            for part_value in &parts {
                let part: medscale_contracts::data_sources::SnapshotPart =
                    serde_json::from_value(part_value.clone()).map_err(|e| e.to_string())?;
                part.validate().map_err(|e| e.to_string())?;
                if part.snapshot_id.as_str() == id {
                    owned_parts.push(part);
                }
            }
            vault
                .meta
                .restore_snapshot_full(&record, &owned_parts)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot.get("data_receipts").and_then(|v| v.as_array()) {
        for value in entries {
            let (kind, snap, receipt): (String, String, serde_json::Value) =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            vault
                .meta
                .restore_receipt_row(&kind, &snap, &receipt)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot.get("data_saved_views").and_then(|v| v.as_array()) {
        for value in entries {
            let (view, project): (
                medscale_contracts::data_sources::SavedDataView,
                medscale_contracts::objects::OpaqueId,
            ) = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if view.revision < 1 {
                return Err("tampered view revision".to_owned());
            }
            vault
                .meta
                .restore_saved_view_row(&view, &project)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("data_transformations")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let record: crate::data_sources::TransformationRecord =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            record.receipt.validate().map_err(|e| e.to_string())?;
            vault
                .meta
                .restore_transformation_row(&record)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot.get("dataset_releases").and_then(|v| v.as_array()) {
        for value in entries {
            let release: medscale_contracts::data_sources::ReleaseManifest =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            release.card.validate().map_err(|e| e.to_string())?;
            vault
                .meta
                .restore_dataset_release_row(&release)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn restore_v5(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v4(vault, snapshot, sources)?;
    // Spec 076 rows replay exactly (ids/revisions/seqs/checkpoint digests
    // preserved); every row is re-validated where a standalone validator
    // exists so a tampered snapshot fails closed instead of persisting.
    // Participants restore first: every other 076 family references a
    // participant id.
    let agent_refs: std::collections::HashMap<String, Option<String>> =
        match snapshot.get("collab_participant_agent_refs") {
            Some(value) => serde_json::from_value(value.clone()).map_err(|e| e.to_string())?,
            None => std::collections::HashMap::new(),
        };
    if let Some(entries) = snapshot
        .get("collab_participants")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let participant: medscale_contracts::collaboration::ParticipantIdentity =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if participant.revision < 1 {
                return Err("tampered participant revision".to_owned());
            }
            let agent_ref = agent_refs
                .get(participant.header.id.as_str())
                .and_then(|v| v.as_deref())
                .map(medscale_contracts::objects::OpaqueId::new);
            vault
                .meta
                .restore_participant_row(&participant, agent_ref.as_ref())
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot.get("collab_rooms").and_then(|v| v.as_array()) {
        for value in entries {
            let room: medscale_contracts::collaboration::Room =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if room.revision < 1 {
                return Err("tampered room revision".to_owned());
            }
            vault
                .meta
                .restore_room_row(&room)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("collab_room_memberships")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let membership: medscale_contracts::collaboration::RoomMembership =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if membership.revision < 1 {
                return Err("tampered membership revision".to_owned());
            }
            vault
                .meta
                .restore_membership_row(&membership)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot.get("collab_threads").and_then(|v| v.as_array()) {
        for value in entries {
            let thread: medscale_contracts::collaboration::ThreadRef =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            thread.anchor.validate().map_err(|e| e.to_string())?;
            if thread.revision < 1 {
                return Err("tampered thread revision".to_owned());
            }
            vault
                .meta
                .restore_thread_row(&thread)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot.get("collab_messages").and_then(|v| v.as_array()) {
        for value in entries {
            let message: medscale_contracts::collaboration::Message =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            message.validate().map_err(|e| e.to_string())?;
            vault
                .meta
                .restore_message_row(&message)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("collab_message_edits")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let edit: medscale_contracts::collaboration::MessageEdit =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            edit.validate().map_err(|e| e.to_string())?;
            vault
                .meta
                .restore_message_edit_row(&edit)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot.get("collab_tasks").and_then(|v| v.as_array()) {
        for value in entries {
            let task: medscale_contracts::collaboration::Task =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if let Some(anchor) = &task.anchor {
                anchor.validate().map_err(|e| e.to_string())?;
            }
            if task.revision < 1 {
                return Err("tampered task revision".to_owned());
            }
            vault
                .meta
                .restore_task_row(&task)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot.get("collab_notes").and_then(|v| v.as_array()) {
        for value in entries {
            let note: medscale_contracts::collaboration::NoteDocument =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if note.revision < 1 {
                return Err("tampered note revision".to_owned());
            }
            vault
                .meta
                .restore_note_row(&note)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("collab_note_revisions")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let revision: medscale_contracts::collaboration::NoteRevision =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            revision.validate().map_err(|e| e.to_string())?;
            if revision.revision < 1 {
                return Err("tampered note revision number".to_owned());
            }
            vault
                .meta
                .restore_note_revision_row(&revision)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("collab_approval_requests")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let request: medscale_contracts::collaboration::ApprovalRequest =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            request.anchor.validate().map_err(|e| e.to_string())?;
            if request.assignee_participant_ids.is_empty() {
                return Err("tampered approval request: no assignees".to_owned());
            }
            if request.revision < 1 {
                return Err("tampered approval request revision".to_owned());
            }
            vault
                .meta
                .restore_approval_request_row(&request)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("collab_approval_decisions")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let decision: medscale_contracts::collaboration::ApprovalDecision =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            decision.validate().map_err(|e| e.to_string())?;
            vault
                .meta
                .restore_approval_decision_row(&decision)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("collab_activity_records")
        .and_then(|v| v.as_array())
    {
        let mut room_ids: std::collections::HashSet<medscale_contracts::objects::OpaqueId> =
            std::collections::HashSet::new();
        for value in entries {
            let record: medscale_contracts::collaboration::ActivityRecord =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            room_ids.insert(record.room_id.clone());
            vault
                .meta
                .restore_activity_record_row(&record)
                .map_err(|e| e.to_string())?;
        }
        // migration.md section 11: `checkpoint_digest` chains must still
        // verify after a restore. `restore_activity_record_row` preserves
        // each row's digest verbatim (never recomputed), so a tampered
        // middle row would otherwise persist silently; re-verify every
        // restored room's chain from seq = 1 and fail closed on mismatch.
        for room_id in &room_ids {
            vault
                .meta
                .verify_activity_chain(room_id)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Replays one Privacy Gate family: deserialize each row, then restore it.
fn restore_rows<T: serde::de::DeserializeOwned>(
    snapshot: &serde_json::Value,
    key: &str,
    mut restore: impl FnMut(&T) -> Result<(), crate::MetaError>,
) -> Result<(), String> {
    // Every writer emits each family as an array, so any other shape is a
    // hand-edited snapshot and must not restore as an empty family.
    let Some(family) = snapshot.get(key) else {
        return Ok(());
    };
    let entries = family
        .as_array()
        .ok_or_else(|| format!("tampered metadata snapshot: {key} is not an array"))?;
    for value in entries {
        let row: T = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        restore(&row).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Like `restore_rows`, for a family every snapshot of this schema carries:
/// a missing family is a hand-edited snapshot, not an empty one.
fn restore_required_rows<T: serde::de::DeserializeOwned>(
    snapshot: &serde_json::Value,
    key: &str,
    restore: impl FnMut(&T) -> Result<(), crate::MetaError>,
) -> Result<(), String> {
    if snapshot.get(key).is_none() {
        return Err(format!("tampered metadata snapshot: {key} is missing"));
    }
    restore_rows(snapshot, key, restore)
}

fn restore_v21(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v20(vault, snapshot, sources)?;
    restore_required_rows(snapshot, "audio_retention_removal_receipts", |row| {
        vault.meta.restore_audio_retention_removal_receipt_row(row)
    })?;
    vault
        .meta
        .verify_audio_retention_removal_receipts()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v20(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v19(vault, snapshot, sources)?;
    // Spec 091 rows replay through plain-insert paths. The identity comes
    // back without its secret: exporting needs a new identity.
    let meta = &vault.meta;
    restore_required_rows(snapshot, "fed_identity", |row| {
        meta.restore_federation_identity_row(row)
    })?;
    restore_required_rows(snapshot, "fed_peers", |row| {
        meta.restore_federation_peer_row(row)
    })?;
    restore_required_rows(snapshot, "fed_imported", |row| {
        meta.restore_imported_item_row(row)
    })?;
    restore_required_rows(snapshot, "fed_receipts", |row| {
        meta.restore_federation_receipt_row(row)
    })?;
    meta.verify_federation_consistency()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v19(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v18(vault, snapshot, sources)?;
    // Spec 090 rows replay through plain-insert paths. An intent backed up
    // while `sent` restores as `sent` and is recovered as `unknown`.
    let meta = &vault.meta;
    restore_required_rows(snapshot, "ia_adapters", |row| {
        meta.restore_institutional_adapter_row(row)
    })?;
    restore_required_rows(snapshot, "ia_intents", |row| {
        meta.restore_write_intent_row(row)
    })?;
    restore_required_rows(snapshot, "ia_receipts", |row| {
        meta.restore_adapter_receipt_row(row)
    })?;
    meta.verify_institutional_consistency()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v18(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v17(vault, snapshot, sources)?;
    // Spec 089 rows replay through plain-insert paths; each is re-checked
    // against the shipped Pack version it names; cross-row invariants are
    // re-verified once every family is replayed.
    let meta = &vault.meta;
    restore_required_rows(snapshot, "rp_installs", |row| {
        meta.restore_pack_install_row(row)
    })?;
    restore_required_rows(snapshot, "rp_artifacts", |row| {
        meta.restore_research_artifact_row(row)
    })?;
    restore_required_rows(snapshot, "rp_receipts", |row| {
        meta.restore_pack_receipt_row(row)
    })?;
    meta.verify_research_pack_consistency()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v17(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v16(vault, snapshot, sources)?;
    // Spec 088 rows replay through plain-insert paths; cross-row
    // invariants (including media against Spec 081 sources) are
    // re-verified once every family is replayed.
    let meta = &vault.meta;
    restore_required_rows(snapshot, "hud_huddles", |row| meta.restore_huddle_row(row))?;
    restore_required_rows(snapshot, "hud_participants", |row| {
        meta.restore_huddle_participant_row(row)
    })?;
    restore_required_rows(snapshot, "hud_media", |row| {
        meta.restore_huddle_media_row(row)
    })?;
    restore_required_rows(snapshot, "hud_proposals", |row| {
        meta.restore_huddle_proposal_row(row)
    })?;
    restore_required_rows(snapshot, "hud_receipts", |row| {
        meta.restore_huddle_receipt_row(row)
    })?;
    meta.verify_huddle_consistency()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v16(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v15(vault, snapshot, sources)?;
    // Spec 087 rows replay through plain-insert paths (installs keep their
    // revision); release bytes are re-checked; cross-row invariants are
    // re-verified once every family is replayed.
    let meta = &vault.meta;
    restore_required_rows(snapshot, "ext_publishers", |row| {
        meta.restore_ext_publisher_row(row)
    })?;
    restore_required_rows(snapshot, "ext_releases", |row| {
        meta.restore_ext_release_row(row)
    })?;
    restore_required_rows(snapshot, "ext_installs", |row| {
        meta.restore_ext_install_row(row)
    })?;
    restore_required_rows(snapshot, "ext_grants", |row| {
        meta.restore_ext_grant_row(row)
    })?;
    restore_required_rows(snapshot, "ext_lifecycle_receipts", |row| {
        meta.restore_ext_lifecycle_row(row)
    })?;
    restore_required_rows(snapshot, "ext_runtime_receipts", |row| {
        meta.restore_ext_runtime_row(row)
    })?;
    meta.verify_extension_consistency()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v15(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v14(vault, snapshot, sources)?;
    // Spec 086 rows replay through plain-INSERT paths; published table
    // bytes are re-checked against their digests and canonical encoding;
    // cross-row invariants are re-verified once every family is replayed.
    // Staged directories are not part of a backup: a restored workspace
    // whose directory is absent inspects as `missing`.
    let meta = &vault.meta;
    restore_required_rows(snapshot, "rws_workspaces", |row| {
        meta.restore_r_workspace_row(row)
    })?;
    restore_required_rows(snapshot, "rws_launch_receipts", |row| {
        meta.restore_r_launch_row(row)
    })?;
    restore_required_rows(snapshot, "rws_run_receipts", |row| {
        meta.restore_r_run_row(row)
    })?;
    restore_required_rows(snapshot, "rws_publish_receipts", |row| {
        meta.restore_r_publish_row(row)
    })?;
    restore_required_rows(snapshot, "rws_published_tables", |row| {
        meta.restore_r_table_row(row)
    })?;
    meta.verify_r_workspace_consistency()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v14(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v13(vault, snapshot, sources)?;
    // Spec 085 rows replay through plain-INSERT paths; output bytes are
    // re-checked against their digests and canonical encoding; cross-row
    // invariants are re-verified once every family is replayed. A job
    // backed up while `running` restores as `running` and is recovered as
    // `interrupted` by Core, never re-executed.
    let meta = &vault.meta;
    restore_required_rows(snapshot, "compute_jobs", |row| {
        meta.restore_compute_job_row(row)
    })?;
    restore_required_rows(snapshot, "compute_receipts", |row| {
        meta.restore_compute_receipt_row(row)
    })?;
    restore_required_rows(snapshot, "compute_outputs", |row| {
        meta.restore_compute_output_row(row)
    })?;
    meta.verify_compute_consistency()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v13(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v12(vault, snapshot, sources)?;
    // Spec 084 rows replay in their original order through plain-INSERT
    // paths; each Hub event must follow its Project's previous event, and
    // cross-row invariants are re-verified once every family is replayed.
    // Device secrets are never in a backup: a restored client re-enrolls.
    let meta = &vault.meta;
    restore_required_rows(snapshot, "hub_identity", |row| {
        meta.restore_hub_identity_row(row)
    })?;
    restore_required_rows(snapshot, "hub_invitations", |row| {
        meta.restore_invitation_row(row)
    })?;
    restore_required_rows(snapshot, "hub_devices", |row| meta.restore_device_row(row))?;
    restore_required_rows(snapshot, "hub_events", |row| {
        meta.restore_hub_event_row(row)
    })?;
    restore_required_rows(snapshot, "hub_links", |row| meta.restore_hub_link_row(row))?;
    restore_required_rows(snapshot, "hub_outbox", |row| {
        meta.restore_hub_outbox_row(row)
    })?;
    restore_required_rows(snapshot, "hub_mirror", |row| {
        meta.restore_hub_mirror_row(row)
    })?;
    meta.verify_hub_consistency().map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v12(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v11(vault, snapshot, sources)?;
    // Spec 083 rows replay through plain-INSERT paths in their original
    // order; chunk sets are re-checked against their manifest digests and
    // cross-row invariants are re-verified once every family is replayed.
    let meta = &vault.meta;
    restore_required_rows(snapshot, "knowledge_index_versions", |row| {
        meta.restore_index_version_row(row)
    })?;
    restore_required_rows(snapshot, "knowledge_receipts", |row| {
        meta.restore_retrieval_receipt_row(row)
    })?;
    restore_required_rows(snapshot, "knowledge_canvases", |row| {
        meta.restore_canvas_row(row)
    })?;
    meta.verify_knowledge_consistency()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v11(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v10(vault, snapshot, sources)?;
    // Spec 082 rows replay through plain-INSERT paths; derived table bytes
    // are re-checked against their digests; cross-row invariants are
    // re-verified once every family is replayed.
    let meta = &vault.meta;
    restore_rows(snapshot, "analytics_cohorts", |row| {
        meta.restore_cohort_row(row)
    })?;
    restore_rows(snapshot, "analytics_receipts", |row| {
        meta.restore_query_receipt_row(row)
    })?;
    restore_rows(snapshot, "analytics_results", |row| {
        meta.restore_derived_table_row(row)
    })?;
    meta.verify_analytics_consistency()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v10(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v9(vault, snapshot, sources)?;
    // Spec 081 rows replay through plain-INSERT paths; source audio bytes
    // are re-checked against their digests; cross-row invariants are
    // re-verified once every family is replayed.
    let meta = &vault.meta;
    restore_rows(snapshot, "audio_sources", |row| {
        meta.restore_audio_source_row(row)
    })?;
    restore_rows(snapshot, "audio_capture_sessions", |row| {
        meta.restore_capture_session_row(row)
    })?;
    restore_rows(snapshot, "audio_capture_chunks", |row| {
        meta.restore_capture_chunk_row(row)
    })?;
    restore_rows(snapshot, "audio_transcripts", |row| {
        meta.restore_transcript_row(row)
    })?;
    restore_rows(snapshot, "audio_transcript_receipts", |row| {
        meta.restore_transcript_receipt_row(row)
    })?;
    meta.verify_audio_consistency().map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v9(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v8(vault, snapshot, sources)?;
    // Spec 080 rows replay through plain-INSERT paths; evidence and download
    // bytes are re-checked against their digests; cross-row invariants are
    // re-verified once every family is replayed.
    let meta = &vault.meta;
    restore_rows(snapshot, "browse_allowlist", |row| {
        meta.restore_browse_allowlist_row(row)
    })?;
    restore_rows(snapshot, "browse_sessions", |row| {
        meta.restore_browse_session_row(row)
    })?;
    restore_rows(snapshot, "browse_evidence", |row| {
        meta.restore_browse_evidence_row(row)
    })?;
    restore_rows(snapshot, "browse_downloads", |row| {
        meta.restore_browse_download_row(row)
    })?;
    restore_rows(snapshot, "browse_receipts", |row| {
        meta.restore_browse_receipt_row(row)
    })?;
    meta.verify_browse_consistency()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v8(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v7(vault, snapshot, sources)?;
    // Spec 079 rows replay exactly through plain-INSERT paths (duplicates
    // fail closed). Each insert re-validates its contract; cross-row
    // invariants are re-verified once every family is replayed.
    let meta = &vault.meta;
    restore_rows(snapshot, "privacy_profiles", |row| {
        meta.restore_privacy_profile_row(row)
    })?;
    restore_rows(snapshot, "privacy_pseudonym_maps", |row| {
        meta.restore_pseudonym_map_row(row)
    })?;
    restore_rows(snapshot, "privacy_pseudonym_entries", |row| {
        meta.restore_pseudonym_entry_row(row)
    })?;
    restore_rows(snapshot, "privacy_deid_receipts", |row| {
        meta.restore_deid_receipt_row(row)
    })?;
    restore_rows(snapshot, "privacy_classifications", |row| {
        meta.restore_classification_row(row)
    })?;
    restore_rows(snapshot, "privacy_reid_audit", |row| {
        meta.restore_reid_audit_row(row)
    })?;
    restore_rows(snapshot, "privacy_egress_decisions", |row| {
        meta.restore_egress_decision_row(row)
    })?;
    meta.verify_privacy_gate_consistency()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v7(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v6(vault, snapshot, sources)?;
    // Spec 078 rows replay exactly (ids/revisions preserved) through the
    // plain-INSERT restore paths, so a duplicate id or a second binding of
    // one agent run fails closed. Shape is re-validated per row where the
    // contract exposes a validator; the cross-row invariants are
    // re-verified after every family is replayed.
    if let Some(entries) = snapshot.get("model_fleet_lanes").and_then(|v| v.as_array()) {
        for value in entries {
            let lane: medscale_contracts::model_fleet::AgentLane =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            // Re-run the constructor's shape checks (role label bound,
            // non-empty policy subsets) on the restored values.
            medscale_contracts::model_fleet::LanePolicy::new(
                lane.policy.granted_tool_kinds.clone(),
                lane.policy.context_artifact_ids.clone(),
            )?;
            medscale_contracts::model_fleet::AgentLane::new(
                lane.header.clone(),
                lane.project_id.clone(),
                lane.agent_identity_id.clone(),
                lane.context_manifest_id.clone(),
                lane.role_label.clone(),
                lane.policy.clone(),
            )?;
            vault
                .meta
                .restore_agent_lane_row(&lane)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot.get("model_fleet_runs").and_then(|v| v.as_array()) {
        for value in entries {
            let run: medscale_contracts::model_fleet::FleetRun =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            medscale_contracts::model_fleet::FleetRun::new(
                run.header.clone(),
                run.project_id.clone(),
                run.task_prompt.clone(),
            )?;
            vault
                .meta
                .restore_fleet_run_row(&run)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("model_fleet_lane_run_refs")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let lane_run_ref: medscale_contracts::model_fleet::LaneRunRef =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            vault
                .meta
                .restore_lane_run_ref_row(&lane_run_ref)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("model_fleet_comparison_reports")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let report: medscale_contracts::model_fleet::ComparisonReport =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            report.validate()?;
            vault
                .meta
                .restore_comparison_report_row(&report)
                .map_err(|e| e.to_string())?;
        }
    }
    // migration.md sections 6/11: a terminal fleet run whose bound lane
    // runs do not match its state, a lane run ref naming a missing run, or
    // a comparison report naming a lane outside its fleet run is a
    // corruption signal. Re-verify at restore time, not only at insert.
    vault
        .meta
        .verify_model_fleet_consistency()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_v6(
    vault: &SyntheticVault,
    snapshot: &serde_json::Value,
    sources: &mut u64,
) -> Result<(), String> {
    restore_v5(vault, snapshot, sources)?;
    // Spec 077 rows replay exactly (ids/revisions/seqs preserved); every
    // row is re-validated where a standalone validator exists, or via an
    // inline bound check otherwise, so a tampered snapshot fails closed.
    // Identities restore first: every other 077 family references one.
    if let Some(entries) = snapshot
        .get("medagent_identities")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let identity: medscale_contracts::medagent::AgentIdentity =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if identity.revision < 1 {
                return Err("tampered agent identity revision".to_owned());
            }
            vault
                .meta
                .restore_agent_identity_row(&identity)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("medagent_capability_manifests")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let pair: (String, String) =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            let kinds_str: Vec<String> =
                serde_json::from_str(&pair.1).map_err(|e| e.to_string())?;
            let granted_tool_kinds = kinds_str
                .iter()
                .map(|s| medscale_contracts::medagent::ToolKind::parse(s))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            let manifest = medscale_contracts::medagent::AgentCapabilityManifest::new(
                medscale_contracts::objects::OpaqueId::new(pair.0),
                granted_tool_kinds,
            )
            .map_err(|e| e.to_string())?;
            vault
                .meta
                .restore_capability_manifest_row(&manifest)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("medagent_context_manifests")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let manifest: medscale_contracts::medagent::ContextManifest =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if manifest.revision < 1 {
                return Err("tampered context manifest revision".to_owned());
            }
            vault
                .meta
                .restore_context_manifest_row(&manifest)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot.get("medagent_runs").and_then(|v| v.as_array()) {
        for value in entries {
            let run: medscale_contracts::medagent::AgentRun =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if run.revision < 1 {
                return Err("tampered agent run revision".to_owned());
            }
            vault
                .meta
                .restore_agent_run_row(&run)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot.get("medagent_turns").and_then(|v| v.as_array()) {
        for value in entries {
            let turn: medscale_contracts::medagent::AgentTurn =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            vault
                .meta
                .restore_agent_turn_row(&turn)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("medagent_tool_invocations")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let invocation: medscale_contracts::medagent::ToolInvocation =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            invocation.validate().map_err(|e| e.to_string())?;
            vault
                .meta
                .restore_tool_invocation_row(&invocation)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("medagent_tool_receipts")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let receipt: medscale_contracts::medagent::ToolReceipt =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            receipt.validate().map_err(|e| e.to_string())?;
            vault
                .meta
                .restore_tool_receipt_row(&receipt)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("medagent_run_receipts")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let receipt: medscale_contracts::medagent::RunReceipt =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            receipt.validate().map_err(|e| e.to_string())?;
            vault
                .meta
                .restore_run_receipt_row(&receipt)
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(entries) = snapshot
        .get("medagent_proposals")
        .and_then(|v| v.as_array())
    {
        for value in entries {
            let proposal: medscale_contracts::medagent::AgentProposal =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            vault
                .meta
                .restore_agent_proposal_row(&proposal)
                .map_err(|e| e.to_string())?;
        }
    }
    // migration.md section 5 / security.md T11: a terminal run never
    // exists without its RunReceipt, and a RunReceipt never exists without
    // a matching run. Re-verify this explicitly at restore time -- the
    // exact class of gap Spec 076's own exact-range review found missing
    // from its restore path (evidence/076-collaboration-substrate/EXACT_RANGE_REVIEW.md).
    vault
        .meta
        .verify_run_receipt_consistency()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn insert_source_from_json(
    vault: &SyntheticVault,
    value: &serde_json::Value,
) -> Result<(), String> {
    let meta = SourceMeta {
        source_id: medscale_contracts::objects::OpaqueId::new(
            value["source_id"].as_str().unwrap_or_default(),
        ),
        realm_id: medscale_contracts::objects::RealmId::new(
            value["realm_id"].as_str().unwrap_or_default(),
        ),
        authority_scope_id: medscale_contracts::objects::AuthorityScopeId::new(
            value["authority_scope_id"].as_str().unwrap_or_default(),
        ),
        digest: parse_hex(value["digest_hex"].as_str().unwrap_or_default()),
        byte_length: value["byte_length"].as_u64().unwrap_or(0),
        media_type: value["media_type"].as_str().unwrap_or_default().to_owned(),
        visible: value["visible"].as_bool().unwrap_or(false),
        resource_type: value["resource_type"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
    };
    vault
        .blobs
        .verify(&meta.digest, meta.byte_length)
        .map_err(|e| e.to_string())?;
    vault.meta.insert_source(&meta).map_err(|e| e.to_string())?;
    Ok(())
}

fn hex(digest: &DigestSha256) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn parse_hex(name: &str) -> DigestSha256 {
    let mut bytes = [0_u8; 32];
    for (i, chunk) in name.as_bytes().chunks(2).enumerate() {
        if i >= 32 {
            break;
        }
        let s = std::str::from_utf8(chunk).unwrap_or("00");
        bytes[i] = u8::from_str_radix(s, 16).unwrap_or(0);
    }
    DigestSha256::from_bytes(bytes)
}
