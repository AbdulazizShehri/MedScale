//! Spec 074 storage + migration integration tests (074-B).
//!
//! Synthetic data only. Every claim binds to exact behavior below; nothing is
//! inferred from a green compile.

use std::fs;

use medscale_contracts::objects::{AuthorityScopeId, ObjectHeader, OpaqueId, RealmId};
use medscale_contracts::project_graph::{
    ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding, EdgeStatus, Experiment,
    ExperimentStatus, GraphDirection, GraphEndpoint, Project, ProjectArtifactRef, ProjectGraphEdge,
    ProjectGraphPredicate, ProjectStatus, RefStatus,
};
use medscale_storage::{MetaError, SqliteMetaStore, SyntheticVault, backup_vault, restore_vault};

fn temp_root(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-074-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn open_meta(root: &std::path::Path) -> SqliteMetaStore {
    SqliteMetaStore::open_at(&root.join("meta.sqlite3")).unwrap()
}

fn header(id: &str) -> ObjectHeader {
    ObjectHeader {
        id: OpaqueId::new(id),
        schema_version: 1,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn scope() -> AuthorityScopeId {
    AuthorityScopeId::new("scope-1")
}

fn project(id: &str) -> Project {
    Project::new(header(id), format!("project-{id}"), None).unwrap()
}

fn experiment(id: &str, project_id: &str) -> Experiment {
    Experiment::new(
        header(id),
        OpaqueId::new(project_id),
        format!("exp-{id}"),
        None,
    )
    .unwrap()
}

fn descriptor(object: &str) -> ArtifactDescriptor {
    ArtifactDescriptor {
        object_id: OpaqueId::new(object),
        kind: ArtifactKind::SourceRecord,
        binding: ArtifactVersionBinding::IdentityOnly,
    }
}

fn reference(id: &str, project_id: &str, object: &str) -> ProjectArtifactRef {
    ProjectArtifactRef::new(
        header(id),
        OpaqueId::new(project_id),
        None,
        descriptor(object),
    )
    .unwrap()
}

fn edge(id: &str, project_id: &str, subject: &str, object: &str) -> ProjectGraphEdge {
    ProjectGraphEdge::new(
        header(id),
        OpaqueId::new(project_id),
        GraphEndpoint::Experiment(OpaqueId::new(subject)),
        ProjectGraphPredicate::References,
        GraphEndpoint::Artifact(descriptor(object)),
    )
    .unwrap()
}

#[test]
fn schema_v3_migrates_empty_store_additively() {
    let root = temp_root("empty");
    let meta = open_meta(&root);
    let journal = meta.migration_journal().unwrap();
    // Forward-fixed for Spec 077 (v5 -> v6); see interrupted_migration_fails_closed_on_reopen
    // below for why this must track the live top version, not this test's own.
    assert_eq!(
        journal.finished_version,
        medscale_storage::CURRENT_META_SCHEMA_VERSION
    );
    assert!(!journal.interrupted());
    // Pre-074 state still queryable after migration.
    assert!(meta.list_sources().unwrap().is_empty());
    assert!(meta.list_authority_objects().unwrap().is_empty());
    assert_eq!(meta.get_next_seq().unwrap(), 0);
}

#[test]
fn schema_v3_migrates_populated_v2_store_without_identity_loss() {
    use rusqlite::Connection;
    let root = temp_root("v2");
    let db_path = root.join("meta.sqlite3");
    {
        // Genuine v2-shaped store: only v1/v2 DDL, journal finished at 2.
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            r"
            CREATE TABLE migration_journal (version INTEGER PRIMARY KEY, state TEXT NOT NULL);
            CREATE TABLE sources (
              source_id TEXT PRIMARY KEY, realm_id TEXT NOT NULL,
              authority_scope_id TEXT NOT NULL, digest_hex TEXT NOT NULL,
              byte_length INTEGER NOT NULL, media_type TEXT NOT NULL,
              visible INTEGER NOT NULL, resource_type TEXT NOT NULL);
            CREATE UNIQUE INDEX idx_sources_scope_digest ON sources(authority_scope_id, digest_hex);
            CREATE TABLE gc_marks (digest_hex TEXT PRIMARY KEY, epoch INTEGER NOT NULL);
            CREATE TABLE authority_objects (
              object_id TEXT PRIMARY KEY, object_class TEXT NOT NULL,
              realm_id TEXT NOT NULL, authority_scope_id TEXT NOT NULL,
              body_json TEXT NOT NULL, content_digest_hex TEXT, updated_seq INTEGER NOT NULL);
            CREATE INDEX idx_authority_objects_scope_class
              ON authority_objects(authority_scope_id, object_class);
            CREATE TABLE store_state (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            INSERT INTO store_state(key, value) VALUES ('next_seq', '41');
            INSERT INTO migration_journal(version, state) VALUES (1, 'finished');
            INSERT INTO migration_journal(version, state) VALUES (2, 'finished');
            INSERT INTO sources VALUES ('src-keep', 'realm-1', 'scope-1',
              'ab', 3, 'text/plain', 1, 'authority_source');
            INSERT INTO authority_objects VALUES ('prop-keep', 'proposal', 'realm-1',
              'scope-1', '{}', NULL, 41);
            ",
        )
        .unwrap();
    }
    let meta = SqliteMetaStore::open_at(&db_path).unwrap();
    let journal = meta.migration_journal().unwrap();
    // Forward-fixed for Spec 077 (v5 -> v6).
    assert_eq!(
        journal.finished_version,
        medscale_storage::CURRENT_META_SCHEMA_VERSION
    );
    // Pre-074 identities survive the migration untouched.
    let source = meta.get_source(&OpaqueId::new("src-keep")).unwrap();
    assert_eq!(source.media_type, "text/plain");
    assert_eq!(source.byte_length, 3);
    let objects = meta.list_authority_objects().unwrap();
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].object_id, "prop-keep");
    assert_eq!(meta.get_next_seq().unwrap(), 41);
    // 074 tables are usable in the migrated store.
    meta.insert_project(&project("proj-mig")).unwrap();
    assert_eq!(
        meta.get_project(&OpaqueId::new("proj-mig"))
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn interrupted_migration_fails_closed_on_reopen() {
    let root = temp_root("interrupt");
    {
        let meta = open_meta(&root);
        // The journal table keys one row per version (INSERT OR REPLACE), so
        // re-`begin_migration` on an *older*, already-superseded version
        // would be invisible to the fail-closed check once a newer version
        // has finished (a higher finished_version already exists). The
        // interruption must be simulated at the current live top version
        // (7 since Spec 078's v6->v7 step) to stay meaningful. It is read
        // from the journal rather than pinned, so a later version bump
        // cannot silently turn this into a no-op check again.
        let top = meta.migration_journal().unwrap().finished_version;
        assert_eq!(top, medscale_storage::CURRENT_META_SCHEMA_VERSION);
        meta.begin_migration(top).unwrap();
        // Drop without finish: simulated crash mid-migration.
    }
    let top = medscale_storage::CURRENT_META_SCHEMA_VERSION;
    let err = SqliteMetaStore::open_at(&root.join("meta.sqlite3")).unwrap_err();
    assert!(
        matches!(err, MetaError::MigrationIncomplete(v) if v == top),
        "interrupted migration must fail closed, got {err:?}"
    );
}

#[test]
fn project_lifecycle_with_revision_gates() {
    let root = temp_root("proj");
    let meta = open_meta(&root);
    meta.insert_project(&project("proj-1")).unwrap();
    // Duplicate id is a conflict, not a silent overwrite.
    assert!(matches!(
        meta.insert_project(&project("proj-1")),
        Err(MetaError::Conflict(_))
    ));
    let got = meta.get_project(&OpaqueId::new("proj-1")).unwrap();
    assert_eq!((got.revision, got.status), (1, ProjectStatus::Active));
    // Missing reads are NotFound.
    assert!(matches!(
        meta.get_project(&OpaqueId::new("proj-nope")),
        Err(MetaError::NotFound)
    ));
    // Metadata update bumps exactly one revision.
    let updated = meta
        .update_project_meta(&OpaqueId::new("proj-1"), 1, "renamed", Some("desc"))
        .unwrap();
    assert_eq!(updated.revision, 2);
    assert_eq!(updated.name, "renamed");
    // Archive then restore, each revision-guarded.
    let archived = meta
        .set_project_status(&OpaqueId::new("proj-1"), 2, ProjectStatus::Archived)
        .unwrap();
    assert_eq!(
        (archived.revision, archived.status),
        (3, ProjectStatus::Archived)
    );
    assert!(archived.description.is_some());
    let restored = meta
        .set_project_status(&OpaqueId::new("proj-1"), 3, ProjectStatus::Active)
        .unwrap();
    assert_eq!(
        (restored.revision, restored.status),
        (4, ProjectStatus::Active)
    );
    // Scoped list honors status filter and cursors.
    let (active, next) = meta
        .list_projects(&scope(), Some(ProjectStatus::Active), 10, None)
        .unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(next, None);
    let (archived_list, _) = meta
        .list_projects(&scope(), Some(ProjectStatus::Archived), 10, None)
        .unwrap();
    assert!(archived_list.is_empty());
}

#[test]
fn stale_project_write_conflicts_without_write() {
    let root = temp_root("stale");
    let meta = open_meta(&root);
    meta.insert_project(&project("proj-1")).unwrap();
    let err = meta
        .update_project_meta(&OpaqueId::new("proj-1"), 9, "evil", None)
        .unwrap_err();
    assert!(matches!(err, MetaError::Conflict(_)), "got {err:?}");
    // Zero writes happened: revision and name unchanged.
    let got = meta.get_project(&OpaqueId::new("proj-1")).unwrap();
    assert_eq!(got.revision, 1);
    assert!(got.name.contains("proj-1"));
    // Status mutation on unknown id is NotFound, not Conflict.
    assert!(matches!(
        meta.set_project_status(&OpaqueId::new("proj-nope"), 1, ProjectStatus::Archived),
        Err(MetaError::NotFound)
    ));
}

#[test]
fn experiment_crud_counts_and_lists() {
    let root = temp_root("exp");
    let meta = open_meta(&root);
    meta.insert_project(&project("proj-1")).unwrap();
    meta.insert_experiment(&experiment("exp-1", "proj-1"))
        .unwrap();
    meta.insert_experiment(&experiment("exp-2", "proj-1"))
        .unwrap();
    assert_eq!(meta.count_experiments(&OpaqueId::new("proj-1")).unwrap(), 2);
    let moved = meta
        .update_experiment_meta(&OpaqueId::new("exp-1"), 1, "exp-renamed", None)
        .unwrap();
    assert_eq!(moved.revision, 2);
    let done = meta
        .set_experiment_status(&OpaqueId::new("exp-1"), 2, ExperimentStatus::Completed)
        .unwrap();
    assert_eq!(done.status, ExperimentStatus::Completed);
    // Paginated list walks in id order.
    let (page1, cursor) = meta
        .list_experiments(&OpaqueId::new("proj-1"), 1, None)
        .unwrap();
    assert_eq!(page1.len(), 1);
    let cursor = cursor.expect("must have next cursor");
    let (page2, end) = meta
        .list_experiments(&OpaqueId::new("proj-1"), 1, Some(&cursor))
        .unwrap();
    assert_eq!(page2.len(), 1);
    assert_eq!(end, None);
    assert_ne!(page1[0].header.id.as_str(), page2[0].header.id.as_str());
}

#[test]
fn attach_detach_leaves_canonical_untouched() {
    use medscale_contracts::objects::DigestSha256;
    let root = temp_root("attach");
    let meta = open_meta(&root);
    meta.insert_project(&project("proj-1")).unwrap();
    meta.insert_source(&medscale_storage::SourceMeta {
        source_id: OpaqueId::new("src-1"),
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: scope(),
        digest: DigestSha256::of(b"bytes"),
        byte_length: 5,
        media_type: "text/plain".to_owned(),
        visible: true,
        resource_type: "authority_source".to_owned(),
    })
    .unwrap();
    meta.attach_ref(&reference("ref-1", "proj-1", "src-1"))
        .unwrap();
    assert_eq!(meta.count_active_refs(&OpaqueId::new("proj-1")).unwrap(), 1);
    let detached = meta
        .set_ref_status(&OpaqueId::new("ref-1"), 1, RefStatus::Detached)
        .unwrap();
    assert_eq!(
        (detached.revision, detached.status),
        (2, RefStatus::Detached)
    );
    assert_eq!(meta.count_active_refs(&OpaqueId::new("proj-1")).unwrap(), 0);
    // No cascade: the canonical source row is byte-identical.
    let source = meta.get_source(&OpaqueId::new("src-1")).unwrap();
    assert_eq!(source.byte_length, 5);
    assert_eq!(source.digest, DigestSha256::of(b"bytes"));
}

#[test]
fn duplicate_active_attach_conflicts() {
    let root = temp_root("dup");
    let meta = open_meta(&root);
    meta.insert_project(&project("proj-1")).unwrap();
    meta.attach_ref(&reference("ref-1", "proj-1", "src-1"))
        .unwrap();
    // Same active tuple under a new id is still a duplicate.
    assert!(matches!(
        meta.attach_ref(&reference("ref-2", "proj-1", "src-1")),
        Err(MetaError::Conflict(_))
    ));
    // Detach frees the tuple: re-attach under a new id succeeds.
    meta.set_ref_status(&OpaqueId::new("ref-1"), 1, RefStatus::Detached)
        .unwrap();
    meta.attach_ref(&reference("ref-3", "proj-1", "src-1"))
        .unwrap();
    // Same target bound to an experiment is a distinct attachment slot.
    let exp_ref = ProjectArtifactRef::new(
        header("ref-4"),
        OpaqueId::new("proj-1"),
        Some(OpaqueId::new("exp-1")),
        descriptor("src-1"),
    )
    .unwrap();
    meta.attach_ref(&exp_ref).unwrap();
}

#[test]
fn graph_neighbors_bounded_and_paginated() {
    let root = temp_root("graph");
    let meta = open_meta(&root);
    meta.insert_project(&project("proj-1")).unwrap();
    meta.create_edge(&edge("edge-1", "proj-1", "exp-1", "src-1"))
        .unwrap();
    meta.create_edge(&edge("edge-2", "proj-1", "exp-1", "src-2"))
        .unwrap();
    meta.create_edge(&edge("edge-3", "proj-1", "exp-9", "src-1"))
        .unwrap();
    // Duplicate active tuple conflicts.
    assert!(matches!(
        meta.create_edge(&edge("edge-9", "proj-1", "exp-1", "src-1")),
        Err(MetaError::Conflict(_))
    ));
    let start = GraphEndpoint::Experiment(OpaqueId::new("exp-1"));
    let (both, _) = meta
        .query_neighbors(
            &OpaqueId::new("proj-1"),
            &start,
            None,
            GraphDirection::Both,
            10,
            None,
        )
        .unwrap();
    assert_eq!(both.len(), 2);
    let (outgoing, _) = meta
        .query_neighbors(
            &OpaqueId::new("proj-1"),
            &start,
            None,
            GraphDirection::Outgoing,
            10,
            None,
        )
        .unwrap();
    assert_eq!(outgoing.len(), 2);
    let (incoming, _) = meta
        .query_neighbors(
            &OpaqueId::new("proj-1"),
            &start,
            None,
            GraphDirection::Incoming,
            10,
            None,
        )
        .unwrap();
    assert!(incoming.is_empty());
    // Predicate filter narrows; unknown predicates match nothing (closed).
    let (filtered, _) = meta
        .query_neighbors(
            &OpaqueId::new("proj-1"),
            &start,
            Some(&[ProjectGraphPredicate::References]),
            GraphDirection::Both,
            10,
            None,
        )
        .unwrap();
    assert_eq!(filtered.len(), 2);
    let (none, _) = meta
        .query_neighbors(
            &OpaqueId::new("proj-1"),
            &start,
            Some(&[ProjectGraphPredicate::Contains]),
            GraphDirection::Both,
            10,
            None,
        )
        .unwrap();
    assert!(none.is_empty());
    // Pagination walks one edge per page in deterministic order.
    let (page1, cursor) = meta
        .query_neighbors(
            &OpaqueId::new("proj-1"),
            &start,
            None,
            GraphDirection::Both,
            1,
            None,
        )
        .unwrap();
    assert_eq!(page1.len(), 1);
    let cursor = cursor.expect("must have next cursor");
    let (page2, end) = meta
        .query_neighbors(
            &OpaqueId::new("proj-1"),
            &start,
            None,
            GraphDirection::Both,
            1,
            Some(&cursor),
        )
        .unwrap();
    assert_eq!(page2.len(), 1);
    assert_eq!(end, None);
    assert_ne!(page1[0].header.id.as_str(), page2[0].header.id.as_str());
    // Removal tombstones the relationship; endpoints are untouched.
    let removed = meta
        .set_edge_status(&OpaqueId::new("edge-1"), 1, EdgeStatus::Removed)
        .unwrap();
    assert_eq!(removed.status, EdgeStatus::Removed);
    assert_eq!(
        meta.count_active_edges(&OpaqueId::new("proj-1")).unwrap(),
        2
    );
    let (after_remove, _) = meta
        .query_neighbors(
            &OpaqueId::new("proj-1"),
            &start,
            None,
            GraphDirection::Both,
            10,
            None,
        )
        .unwrap();
    assert_eq!(after_remove.len(), 1);
}

#[test]
fn reopen_persists_074_state() {
    let root = temp_root("reopen");
    {
        let meta = open_meta(&root);
        meta.insert_project(&project("proj-1")).unwrap();
        meta.insert_experiment(&experiment("exp-1", "proj-1"))
            .unwrap();
        meta.attach_ref(&reference("ref-1", "proj-1", "src-1"))
            .unwrap();
        meta.create_edge(&edge("edge-1", "proj-1", "exp-1", "src-1"))
            .unwrap();
    }
    // Reopen without close: crash-shaped recovery still reads full state.
    let meta = open_meta(&root);
    assert_eq!(
        meta.get_project(&OpaqueId::new("proj-1")).unwrap().revision,
        1
    );
    assert_eq!(
        meta.get_experiment(&OpaqueId::new("exp-1"))
            .unwrap()
            .revision,
        1
    );
    assert_eq!(meta.get_ref(&OpaqueId::new("ref-1")).unwrap().revision, 1);
    assert_eq!(meta.get_edge(&OpaqueId::new("edge-1")).unwrap().revision, 1);
}

#[test]
fn backup_restore_carries_074_rows_with_revisions() {
    let root = temp_root("bak-src");
    let bak = temp_root("bak-dst");
    let restored_root = temp_root("bak-restored");
    {
        let vault = SyntheticVault::open("v-bak", &root).unwrap();
        vault.meta.insert_project(&project("proj-1")).unwrap();
        vault
            .meta
            .insert_experiment(&experiment("exp-1", "proj-1"))
            .unwrap();
        vault
            .meta
            .attach_ref(&reference("ref-1", "proj-1", "src-1"))
            .unwrap();
        vault
            .meta
            .create_edge(&edge("edge-1", "proj-1", "exp-1", "src-1"))
            .unwrap();
        // Advance one revision so restore must preserve non-initial state.
        vault
            .meta
            .update_project_meta(&OpaqueId::new("proj-1"), 1, "renamed", None)
            .unwrap();
        backup_vault(&vault, &bak).unwrap();
    }
    restore_vault(&bak, &restored_root).unwrap();
    let vault = SyntheticVault::open("v-bak", &restored_root).unwrap();
    let project = vault.meta.get_project(&OpaqueId::new("proj-1")).unwrap();
    assert_eq!((project.revision, project.name.as_str()), (2, "renamed"));
    assert_eq!(
        vault
            .meta
            .get_experiment(&OpaqueId::new("exp-1"))
            .unwrap()
            .revision,
        1
    );
    assert_eq!(
        vault
            .meta
            .get_ref(&OpaqueId::new("ref-1"))
            .unwrap()
            .revision,
        1
    );
    assert_eq!(
        vault
            .meta
            .get_edge(&OpaqueId::new("edge-1"))
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn project_id_sequence_is_durable_across_reopen() {
    let root = temp_root("idseq");
    {
        let meta = open_meta(&root);
        let a = meta.alloc_project_id("ref").unwrap();
        let b = meta.alloc_project_id("ref").unwrap();
        assert_eq!(a.as_str(), "ref-1");
        assert_eq!(b.as_str(), "ref-2");
        // Independent prefixes share nothing but the counter.
        assert_eq!(meta.alloc_project_id("edge").unwrap().as_str(), "edge-3");
    }
    let meta = open_meta(&root);
    assert_eq!(meta.alloc_project_id("ref").unwrap().as_str(), "ref-4");
}

#[test]
fn unknown_future_status_fails_closed() {
    use rusqlite::Connection;
    let root = temp_root("future");
    let meta = open_meta(&root);
    meta.insert_project(&project("proj-1")).unwrap();
    drop(meta);
    {
        let conn = Connection::open(root.join("meta.sqlite3")).unwrap();
        conn.execute(
            "UPDATE projects SET status = 'deleted' WHERE project_id = 'proj-1'",
            [],
        )
        .unwrap();
    }
    let meta = open_meta(&root);
    assert!(matches!(
        meta.get_project(&OpaqueId::new("proj-1")),
        Err(MetaError::UnsupportedSchema(_))
    ));
}

#[test]
fn negative_revision_tampering_fails_closed() {
    use rusqlite::Connection;
    let root = temp_root("negrev");
    let meta = open_meta(&root);
    meta.insert_project(&project("proj-1")).unwrap();
    meta.insert_experiment(&experiment("exp-1", "proj-1"))
        .unwrap();
    drop(meta);
    {
        let conn = Connection::open(root.join("meta.sqlite3")).unwrap();
        conn.execute(
            "UPDATE projects SET revision = -3 WHERE project_id = 'proj-1'",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE experiments SET revision = -1 WHERE experiment_id = 'exp-1'",
            [],
        )
        .unwrap();
    }
    let meta = open_meta(&root);
    assert!(matches!(
        meta.get_project(&OpaqueId::new("proj-1")),
        Err(MetaError::CorruptObjectBody(_))
    ));
    assert!(matches!(
        meta.get_experiment(&OpaqueId::new("exp-1")),
        Err(MetaError::CorruptObjectBody(_))
    ));
}
