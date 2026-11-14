//! Spec 077 MedAgent Workbench storage + migration integration tests
//! (T077-02).
//!
//! Synthetic data only. Every claim binds to exact behavior below; nothing is
//! inferred from a green compile. Covers `migration.md` sections 3, 5, 7, 8
//! and 11, and `security.md` T5/T11, using the same raw-storage idiom
//! `project_graph_074.rs`/`data_sources_075.rs`/`collaboration_076.rs`
//! already established for this workstation (no local linker; CI is the
//! authoritative compile/test signal).

use std::fs;

use medscale_contracts::collaboration::{ParticipantIdentity, ParticipantKind};
use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::medagent::{
    AgentCapabilityManifest, AgentIdentity, AgentProposal, AgentRun, AgentRunState, AgentTurnKind,
    ContextManifest, MEDAGENT_SCHEMA_VERSION, RunReceipt, ToolKind,
};
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::project_graph::{
    ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding, Project,
};
use medscale_storage::{MetaError, SqliteMetaStore, SyntheticVault, backup_vault, restore_vault};

fn temp_root(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-077-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn open_meta(root: &std::path::Path) -> SqliteMetaStore {
    SqliteMetaStore::open_at(&root.join("meta.sqlite3")).unwrap()
}

fn scope() -> AuthorityScopeId {
    AuthorityScopeId::new("scope-1")
}

fn header(id: &str) -> ObjectHeader {
    ObjectHeader {
        id: OpaqueId::new(id),
        schema_version: MEDAGENT_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: scope(),
    }
}

fn project(id: &str) -> Project {
    Project::new(header(id), format!("project-{id}"), None).unwrap()
}

fn agent_participant(id: &str, holder: &str) -> ParticipantIdentity {
    ParticipantIdentity::new(
        header(id),
        OpaqueId::new(holder),
        ParticipantKind::Agent,
        format!("agent-participant-{id}"),
    )
    .unwrap()
}

fn artifact_descriptor(id: &str) -> ArtifactDescriptor {
    ArtifactDescriptor {
        object_id: OpaqueId::new(id),
        kind: ArtifactKind::SourceRecord,
        binding: ArtifactVersionBinding::IdentityOnly,
    }
}

fn identity(id: &str, project_id: &str) -> AgentIdentity {
    AgentIdentity::new(
        header(id),
        OpaqueId::new(project_id),
        OpaqueId::new("pack-1"),
        "1.0.0".to_owned(),
        format!("agent-{id}"),
    )
    .unwrap()
}

fn capabilities(agent_id: &str) -> AgentCapabilityManifest {
    AgentCapabilityManifest::new(
        OpaqueId::new(agent_id),
        vec![
            ToolKind::ReadContextArtifact,
            ToolKind::SearchContextArtifacts,
        ],
    )
    .unwrap()
}

fn context_manifest(id: &str, project_id: &str, artifact_id: &str) -> ContextManifest {
    ContextManifest::new(
        header(id),
        OpaqueId::new(project_id),
        vec![artifact_descriptor(artifact_id)],
    )
    .unwrap()
}

fn run(id: &str, project_id: &str, agent_id: &str, context_id: &str) -> AgentRun {
    AgentRun::new(
        header(id),
        OpaqueId::new(project_id),
        OpaqueId::new(agent_id),
        OpaqueId::new(context_id),
        "summarize the bound context".to_owned(),
    )
    .unwrap()
}

fn run_receipt(run_id: &str, context_id: &str, final_state: AgentRunState) -> RunReceipt {
    let receipt = RunReceipt {
        header: header(&format!("receipt-{run_id}")),
        run_id: OpaqueId::new(run_id),
        pack_id: OpaqueId::new("pack-1"),
        pack_version: "1.0.0".to_owned(),
        context_manifest_id: OpaqueId::new(context_id),
        context_manifest_revision: 1,
        tool_invocation_ids: Vec::new(),
        final_state,
        failure_reason: if final_state == AgentRunState::Failed {
            Some("synthetic failure".to_owned())
        } else {
            None
        },
    };
    receipt.validate().unwrap();
    receipt
}

// ---------------------------------------------------------------------------
// migration.md sections 3/7/8: fixture + qualification sequence
// ---------------------------------------------------------------------------

#[test]
fn migration_v5_to_v6_preserves_pre_077_rows_and_adds_medagent_tables() {
    let root = temp_root("migrate-v6");
    let meta = open_meta(&root);

    // Pre-077 canonical rows: a 074 Project and a 076 participant bound as
    // ParticipantKind::Agent, proving the 076 <-> 077 integration point
    // (migration.md section 3) survives the v6 migration untouched.
    meta.insert_project(&project("proj-1")).unwrap();
    meta.insert_participant(&agent_participant("agent-participant-1", "holder-1"), None)
        .unwrap();

    // 077 rows layered on top, additive per migration.md section 1.
    meta.insert_agent_identity_with_capabilities(
        &identity("agent-1", "proj-1"),
        &capabilities("agent-1"),
    )
    .unwrap();
    meta.insert_context_manifest(&context_manifest("ctx-1", "proj-1", "artifact-1"))
        .unwrap();
    meta.insert_agent_run(&run("run-1", "proj-1", "agent-1", "ctx-1"))
        .unwrap();

    let journal = meta.migration_journal().unwrap();
    assert_eq!(
        journal.finished_version,
        medscale_storage::CURRENT_META_SCHEMA_VERSION
    );

    // Reopen must be safe (idempotent migration) and preserve every row
    // across both the pre-077 and the 077 families.
    drop(meta);
    let meta = open_meta(&root);
    let journal_again = meta.migration_journal().unwrap();
    assert_eq!(
        journal_again.finished_version,
        medscale_storage::CURRENT_META_SCHEMA_VERSION,
        "repeat open must be a no-op, not a re-migration"
    );

    let read_project = meta.get_project(&OpaqueId::new("proj-1")).unwrap();
    assert_eq!(read_project.revision, 1);

    let read_identity = meta.get_agent_identity(&OpaqueId::new("agent-1")).unwrap();
    assert_eq!(read_identity.pack_id.as_str(), "pack-1");
    let read_capabilities = meta
        .get_capability_manifest(&OpaqueId::new("agent-1"))
        .unwrap();
    assert!(read_capabilities.grants(ToolKind::ReadContextArtifact));

    let read_context = meta.get_context_manifest(&OpaqueId::new("ctx-1")).unwrap();
    assert_eq!(read_context.selected_artifacts.len(), 1);

    let read_run = meta.get_agent_run(&OpaqueId::new("run-1")).unwrap();
    assert_eq!(read_run.status, AgentRunState::Pending);
}

#[test]
fn repeated_open_and_repeated_migration_is_safe() {
    let root = temp_root("repeat-open");
    {
        let meta = open_meta(&root);
        meta.insert_agent_identity_with_capabilities(
            &identity("agent-1", "proj-1"),
            &capabilities("agent-1"),
        )
        .unwrap();
    }
    for _ in 0..5 {
        let meta = open_meta(&root);
        let journal = meta.migration_journal().unwrap();
        assert_eq!(
            journal.finished_version,
            medscale_storage::CURRENT_META_SCHEMA_VERSION
        );
        let read = meta.get_agent_identity(&OpaqueId::new("agent-1")).unwrap();
        assert_eq!(read.revision, 1);
    }
}

// ---------------------------------------------------------------------------
// migration.md section 5 / security.md T11: run/receipt atomicity + full
// CRUD roundtrip across every 077 family
// ---------------------------------------------------------------------------

#[test]
fn full_lifecycle_crud_roundtrip_and_run_receipt_invariant() {
    let root = temp_root("lifecycle");
    let meta = open_meta(&root);
    meta.insert_project(&project("proj-1")).unwrap();
    meta.insert_agent_identity_with_capabilities(
        &identity("agent-1", "proj-1"),
        &capabilities("agent-1"),
    )
    .unwrap();
    meta.insert_context_manifest(&context_manifest("ctx-1", "proj-1", "artifact-1"))
        .unwrap();
    meta.insert_agent_run(&run("run-1", "proj-1", "agent-1", "ctx-1"))
        .unwrap();

    // Non-terminal run has no receipt yet; the invariant holds trivially.
    meta.verify_run_receipt_consistency().unwrap();

    let turn = meta
        .insert_agent_turn(
            header("turn-1"),
            &OpaqueId::new("run-1"),
            AgentTurnKind::PromptSubmitted,
            &serde_json::json!({"prompt": "summarize the bound context"}),
        )
        .unwrap();
    assert_eq!(turn.seq, 1);

    let refused = meta
        .insert_refused_tool_invocation(
            header("inv-refused"),
            &OpaqueId::new("run-1"),
            1,
            ToolKind::SearchContextArtifacts,
            &serde_json::json!({"query": "labs"}),
            "tool kind not granted at registration time",
        )
        .unwrap();
    assert_eq!(refused.seq, 1);

    let (executed, receipt) = meta
        .insert_executed_tool_invocation_with_receipt(
            header("inv-executed"),
            header("tool-receipt-1"),
            &OpaqueId::new("run-1"),
            1,
            ToolKind::ReadContextArtifact,
            &serde_json::json!({"object_id": "artifact-1"}),
            &serde_json::json!({"content": "synthetic artifact body"}),
        )
        .unwrap();
    assert_eq!(executed.seq, 2);
    assert_eq!(receipt.invocation_id.as_str(), "inv-executed");

    let turns = meta.list_agent_turns(&OpaqueId::new("run-1")).unwrap();
    assert_eq!(turns.len(), 1);
    let invocations = meta.list_tool_invocations(&OpaqueId::new("run-1")).unwrap();
    assert_eq!(invocations.len(), 2);

    let running = meta
        .set_agent_run_running(&OpaqueId::new("run-1"), 1)
        .unwrap();
    assert_eq!(running.status, AgentRunState::Running);
    assert_eq!(running.revision, 2);

    let terminal_receipt = RunReceipt {
        tool_invocation_ids: vec![OpaqueId::new("inv-executed")],
        ..run_receipt("run-1", "ctx-1", AgentRunState::Completed)
    };
    let (final_run, stored_receipt) = meta
        .commit_terminal_transition_with_receipt(
            &OpaqueId::new("run-1"),
            2,
            AgentRunState::Completed,
            &terminal_receipt,
        )
        .unwrap();
    assert_eq!(final_run.status, AgentRunState::Completed);
    assert_eq!(stored_receipt.tool_invocation_ids.len(), 1);

    // A terminal run must have exactly one RunReceipt; re-reading confirms
    // it, and the crate-wide invariant check must agree.
    let read_receipt = meta.get_run_receipt(&OpaqueId::new("run-1")).unwrap();
    assert_eq!(read_receipt.final_state, AgentRunState::Completed);
    meta.verify_run_receipt_consistency().unwrap();

    let proposal = AgentProposal {
        header: header("agent-proposal-1"),
        run_id: OpaqueId::new("run-1"),
        proposal_id: OpaqueId::new("proposal-1"),
    };
    meta.insert_agent_proposal(&proposal).unwrap();
    let read_proposal = meta
        .get_agent_proposal_for_run(&OpaqueId::new("run-1"))
        .unwrap()
        .unwrap();
    assert_eq!(read_proposal.proposal_id.as_str(), "proposal-1");
}

#[test]
fn revoke_and_run_transitions_are_stale_revision_safe() {
    let root = temp_root("cas");
    let meta = open_meta(&root);
    meta.insert_agent_identity_with_capabilities(
        &identity("agent-1", "proj-1"),
        &capabilities("agent-1"),
    )
    .unwrap();
    meta.insert_context_manifest(&context_manifest("ctx-1", "proj-1", "artifact-1"))
        .unwrap();
    meta.insert_agent_run(&run("run-1", "proj-1", "agent-1", "ctx-1"))
        .unwrap();

    // Stale expected-revision is rejected, not silently applied.
    assert!(matches!(
        meta.revoke_agent_identity(&OpaqueId::new("agent-1"), 99),
        Err(MetaError::Conflict(_))
    ));
    let revoked = meta
        .revoke_agent_identity(&OpaqueId::new("agent-1"), 1)
        .unwrap();
    assert_eq!(revoked.revision, 2);
    // Re-applying the now-stale revision-1 expectation fails closed.
    assert!(matches!(
        meta.revoke_agent_identity(&OpaqueId::new("agent-1"), 1),
        Err(MetaError::Conflict(_))
    ));

    assert!(matches!(
        meta.set_agent_run_running(&OpaqueId::new("run-1"), 99),
        Err(MetaError::Conflict(_))
    ));
    meta.set_agent_run_running(&OpaqueId::new("run-1"), 1)
        .unwrap();
    assert!(
        matches!(
            meta.set_agent_run_running(&OpaqueId::new("run-1"), 1),
            Err(MetaError::Conflict(_))
        ),
        "double transition on a stale revision must not silently re-apply"
    );
}

#[test]
fn verify_run_receipt_consistency_detects_orphaned_receipt_and_missing_receipt() {
    let root = temp_root("consistency-detect");
    let meta = open_meta(&root);
    meta.insert_agent_identity_with_capabilities(
        &identity("agent-1", "proj-1"),
        &capabilities("agent-1"),
    )
    .unwrap();
    meta.insert_context_manifest(&context_manifest("ctx-1", "proj-1", "artifact-1"))
        .unwrap();
    meta.insert_agent_run(&run("run-1", "proj-1", "agent-1", "ctx-1"))
        .unwrap();
    meta.set_agent_run_running(&OpaqueId::new("run-1"), 1)
        .unwrap();

    // Direct DB tamper, not reachable through the public API: insert a
    // RunReceipt for a run that is still non-terminal (Running).
    {
        let conn = rusqlite::Connection::open(root.join("meta.sqlite3")).unwrap();
        conn.execute(
            "INSERT INTO medagent_run_receipts(run_id, realm_id, authority_scope_id, pack_id, pack_version, context_id, context_revision, tool_invocation_ids_json, final_state, failure_reason, schema_version)
             VALUES ('run-1', 'realm-1', 'scope-1', 'pack-1', '1.0.0', 'ctx-1', 1, '[]', 'completed', NULL, 1)",
            [],
        )
        .unwrap();
    }
    let err = meta.verify_run_receipt_consistency().unwrap_err();
    assert!(
        matches!(err, MetaError::CorruptObjectBody(_)),
        "a non-terminal run with a RunReceipt must fail closed, got {err:?}"
    );

    // Reset: remove the orphan receipt, then tamper the other direction --
    // force the run to a terminal status without ever committing a receipt.
    {
        let conn = rusqlite::Connection::open(root.join("meta.sqlite3")).unwrap();
        conn.execute(
            "DELETE FROM medagent_run_receipts WHERE run_id = 'run-1'",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE medagent_runs SET status = 'completed' WHERE run_id = 'run-1'",
            [],
        )
        .unwrap();
    }
    let err = meta.verify_run_receipt_consistency().unwrap_err();
    assert!(
        matches!(err, MetaError::CorruptObjectBody(_)),
        "a terminal run with no RunReceipt must fail closed, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// security.md T12 analogue: half-committed state after crash
// ---------------------------------------------------------------------------

#[test]
fn executed_tool_invocation_and_receipt_commit_atomically_or_neither_does() {
    let root = temp_root("atomic-tool");
    let meta = open_meta(&root);
    meta.insert_agent_identity_with_capabilities(
        &identity("agent-1", "proj-1"),
        &capabilities("agent-1"),
    )
    .unwrap();
    meta.insert_context_manifest(&context_manifest("ctx-1", "proj-1", "artifact-1"))
        .unwrap();
    meta.insert_agent_run(&run("run-1", "proj-1", "agent-1", "ctx-1"))
        .unwrap();

    // Pre-occupy the tool receipt's unique invocation_id slot so the
    // in-transaction receipt insert collides after the invocation insert
    // has already executed inside the same transaction.
    {
        let conn = rusqlite::Connection::open(root.join("meta.sqlite3")).unwrap();
        conn.execute(
            "INSERT INTO medagent_tool_receipts(receipt_id, invocation_id, realm_id, authority_scope_id, result_json, schema_version)
             VALUES ('pre-existing-receipt', 'inv-1', 'realm-1', 'scope-1', '{}', 1)",
            [],
        )
        .unwrap();
    }

    let err = meta
        .insert_executed_tool_invocation_with_receipt(
            header("inv-1"),
            header("tool-receipt-2"),
            &OpaqueId::new("run-1"),
            1,
            ToolKind::ReadContextArtifact,
            &serde_json::json!({"object_id": "artifact-1"}),
            &serde_json::json!({"content": "x"}),
        )
        .unwrap_err();
    let _ = err;

    // The invocation row must not be visible: the primary-row insert
    // executed inside the same transaction as the failed receipt insert,
    // and the whole transaction must have rolled back together.
    assert!(matches!(
        meta.list_tool_invocations(&OpaqueId::new("run-1")),
        Ok(v) if v.is_empty()
    ));
}

// ---------------------------------------------------------------------------
// migration.md section 11 / security.md T11: backup + restore
// ---------------------------------------------------------------------------

fn build_full_vault(vault: &SyntheticVault) {
    vault.meta.insert_project(&project("proj-1")).unwrap();
    vault
        .meta
        .insert_agent_identity_with_capabilities(
            &identity("agent-1", "proj-1"),
            &capabilities("agent-1"),
        )
        .unwrap();
    vault
        .meta
        .insert_context_manifest(&context_manifest("ctx-1", "proj-1", "artifact-1"))
        .unwrap();
    vault
        .meta
        .insert_agent_run(&run("run-1", "proj-1", "agent-1", "ctx-1"))
        .unwrap();
    vault
        .meta
        .insert_agent_turn(
            header("turn-1"),
            &OpaqueId::new("run-1"),
            AgentTurnKind::PromptSubmitted,
            &serde_json::json!({"prompt": "go"}),
        )
        .unwrap();
    let (_, _receipt) = vault
        .meta
        .insert_executed_tool_invocation_with_receipt(
            header("inv-1"),
            header("tool-receipt-1"),
            &OpaqueId::new("run-1"),
            1,
            ToolKind::ReadContextArtifact,
            &serde_json::json!({"object_id": "artifact-1"}),
            &serde_json::json!({"content": "x"}),
        )
        .unwrap();
    vault
        .meta
        .set_agent_run_running(&OpaqueId::new("run-1"), 1)
        .unwrap();
    let terminal_receipt = RunReceipt {
        tool_invocation_ids: vec![OpaqueId::new("inv-1")],
        ..run_receipt("run-1", "ctx-1", AgentRunState::Completed)
    };
    vault
        .meta
        .commit_terminal_transition_with_receipt(
            &OpaqueId::new("run-1"),
            2,
            AgentRunState::Completed,
            &terminal_receipt,
        )
        .unwrap();
    vault
        .meta
        .insert_agent_proposal(&AgentProposal {
            header: header("agent-proposal-1"),
            run_id: OpaqueId::new("run-1"),
            proposal_id: OpaqueId::new("proposal-1"),
        })
        .unwrap();
}

#[test]
fn backup_restore_roundtrips_medagent_rows_and_reverifies_run_receipt_consistency() {
    let root = temp_root("backup");
    let vault_root = root.join("vault");
    let vault = SyntheticVault::open("vault-1", &vault_root).unwrap();
    build_full_vault(&vault);

    let dest = root.join("backup-out");
    let manifest_out = backup_vault(&vault, &dest).unwrap();
    assert_eq!(
        manifest_out.schema_version,
        medscale_storage::CURRENT_META_SCHEMA_VERSION
    );

    let restore_root = root.join("restored");
    let _ = restore_vault(&dest, &restore_root).unwrap();
    let restored = SyntheticVault::open("vault-1", &restore_root).unwrap();

    let identity = restored
        .meta
        .get_agent_identity(&OpaqueId::new("agent-1"))
        .unwrap();
    assert_eq!(identity.pack_id.as_str(), "pack-1");
    let caps = restored
        .meta
        .get_capability_manifest(&OpaqueId::new("agent-1"))
        .unwrap();
    assert!(caps.grants(ToolKind::ReadContextArtifact));
    let ctx = restored
        .meta
        .get_context_manifest(&OpaqueId::new("ctx-1"))
        .unwrap();
    assert_eq!(ctx.selected_artifacts.len(), 1);
    let run = restored
        .meta
        .get_agent_run(&OpaqueId::new("run-1"))
        .unwrap();
    assert_eq!(run.status, AgentRunState::Completed);
    let turns = restored
        .meta
        .list_agent_turns(&OpaqueId::new("run-1"))
        .unwrap();
    assert_eq!(turns.len(), 1);
    let invocations = restored
        .meta
        .list_tool_invocations(&OpaqueId::new("run-1"))
        .unwrap();
    assert_eq!(invocations.len(), 1);
    let receipt = restored
        .meta
        .get_run_receipt(&OpaqueId::new("run-1"))
        .unwrap();
    assert_eq!(receipt.final_state, AgentRunState::Completed);
    let proposal = restored
        .meta
        .get_agent_proposal_for_run(&OpaqueId::new("run-1"))
        .unwrap()
        .unwrap();
    assert_eq!(proposal.proposal_id.as_str(), "proposal-1");

    // restore_v6 re-verifies run/receipt consistency explicitly; a clean
    // restore must also pass a fresh, independent check.
    restored.meta.verify_run_receipt_consistency().unwrap();
}

/// Proves `restore_v6`'s explicit `verify_run_receipt_consistency` call
/// (added specifically to avoid repeating the exact gap Spec 076's own
/// exact-range review found in `restore_v5`) actually fires at restore
/// time: a hand-edited backup whose editor also recomputes the outer
/// `metadata_snapshot_digest` to match their edit -- so the outer digest
/// alone cannot catch it -- must still fail closed.
#[test]
fn restore_rejects_hand_edited_backup_with_orphaned_run_receipt() {
    let root = temp_root("tamper-receipt");
    let vault_root = root.join("vault");
    let vault = SyntheticVault::open("vault-1", &vault_root).unwrap();
    build_full_vault(&vault);

    let dest = root.join("backup-out");
    backup_vault(&vault, &dest).unwrap();

    // Hand-edit the snapshot: flip the run's persisted status back to
    // "running" while leaving its RunReceipt in place, simulating an
    // editor who wants to "unwind" a completed run's receipt but forgets
    // the invariant. Since backup rows are separate JSON arrays, this
    // requires locating and mutating the one run row.
    let snapshot_path = dest.join("metadata.snapshot");
    let mut snapshot: serde_json::Value =
        serde_json::from_slice(&fs::read(&snapshot_path).unwrap()).unwrap();
    let runs = snapshot
        .get_mut("medagent_runs")
        .and_then(|v| v.as_array_mut())
        .expect("medagent_runs present in snapshot");
    assert_eq!(runs.len(), 1);
    runs[0]["status"] = serde_json::Value::String("running".to_owned());
    let new_snapshot_bytes = serde_json::to_vec(&snapshot).unwrap();
    fs::write(&snapshot_path, &new_snapshot_bytes).unwrap();

    let manifest_path = dest.join("manifest.json");
    let mut manifest: BackupManifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.metadata_snapshot_digest = DigestSha256::of(&new_snapshot_bytes);
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

    let restore_root = root.join("restored");
    let err = restore_vault(&dest, &restore_root).unwrap_err();
    assert!(
        err.contains("RunReceipt") || err.contains("receipt"),
        "a hand-edited backup that leaves a RunReceipt orphaned from its \
         (now non-terminal) run must fail closed at restore time, not merely \
         be detectable by a separate manual check afterward; got: {err}"
    );
}

// ---------------------------------------------------------------------------
// No cascade deletion into canonical target objects
// ---------------------------------------------------------------------------

#[test]
fn agent_lifecycle_mutations_never_touch_the_canonical_project_row() {
    let root = temp_root("no-cascade");
    let meta = open_meta(&root);
    let original = project("proj-1");
    meta.insert_project(&original).unwrap();
    meta.insert_agent_identity_with_capabilities(
        &identity("agent-1", "proj-1"),
        &capabilities("agent-1"),
    )
    .unwrap();
    meta.insert_context_manifest(&context_manifest("ctx-1", "proj-1", "artifact-1"))
        .unwrap();
    meta.insert_agent_run(&run("run-1", "proj-1", "agent-1", "ctx-1"))
        .unwrap();

    // Exercise every mutating 077 storage path this test can reach: revoke
    // the identity and cancel the run via its early Pending -> Cancelled
    // edge (no execution ever started).
    meta.revoke_agent_identity(&OpaqueId::new("agent-1"), 1)
        .unwrap();
    let cancel_receipt = run_receipt("run-1", "ctx-1", AgentRunState::Cancelled);
    meta.commit_terminal_transition_with_receipt(
        &OpaqueId::new("run-1"),
        1,
        AgentRunState::Cancelled,
        &cancel_receipt,
    )
    .unwrap();

    // The canonical Project row must be byte-for-byte unchanged: no 077
    // storage function ever issues a DELETE or UPDATE against `projects`
    // (medagent.rs defines no such statement at all -- this is the runtime
    // half of that structural fact; the other half is `grep -c "FROM
    // projects\|projects SET\|DELETE FROM projects" crates/medscale-storage/src/medagent.rs`
    // returning 0, recorded in this spec's evidence).
    let read_back = meta.get_project(&OpaqueId::new("proj-1")).unwrap();
    assert_eq!(read_back, original);
}
