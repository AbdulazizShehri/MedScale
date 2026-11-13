//! B4: the audit hash chain records every audit record written through the
//! snapshot, and reports records or entries changed outside it.

use medscale_storage::{AuthorityObjectRow, SqliteMetaStore};
use std::path::PathBuf;

fn temp_db(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ms-audit-chain-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("meta.sqlite3")
}

fn audit(id: &str, detail: &str) -> AuthorityObjectRow {
    AuthorityObjectRow {
        object_id: id.to_owned(),
        object_class: "audit".to_owned(),
        realm_id: "realm".to_owned(),
        authority_scope_id: "scope".to_owned(),
        body_json: format!("{{\"detail\":\"{detail}\"}}"),
        content_digest_hex: None,
        updated_seq: 1,
    }
}

fn other(id: &str) -> AuthorityObjectRow {
    AuthorityObjectRow {
        object_class: "project".to_owned(),
        ..audit(id, "not audit")
    }
}

#[test]
fn snapshots_extend_the_chain_and_it_verifies() {
    let path = temp_db("ok");
    let meta = SqliteMetaStore::open_at(&path).unwrap();
    meta.replace_authority_snapshot(&[audit("audit-1", "a"), other("p-1")], 2)
        .unwrap();
    // Unchanged records add nothing.
    meta.replace_authority_snapshot(&[audit("audit-1", "a"), other("p-1")], 2)
        .unwrap();
    let report = meta.verify_audit_chain().unwrap();
    assert_eq!((report.entries, report.records), (1, 1));
    assert!(report.problems.is_empty(), "{:?}", report.problems);

    // A changed record and a new one each add an entry; a removed one too.
    meta.replace_authority_snapshot(&[audit("audit-1", "b"), audit("audit-2", "c")], 3)
        .unwrap();
    meta.replace_authority_snapshot(&[audit("audit-2", "c")], 4)
        .unwrap();
    let report = meta.verify_audit_chain().unwrap();
    assert_eq!((report.entries, report.records), (4, 1));
    assert!(report.problems.is_empty(), "{:?}", report.problems);
}

#[test]
fn edits_outside_the_chain_are_reported() {
    let path = temp_db("tamper");
    let meta = SqliteMetaStore::open_at(&path).unwrap();
    meta.replace_authority_snapshot(&[audit("audit-1", "a"), audit("audit-2", "b")], 3)
        .unwrap();
    drop(meta);

    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE authority_objects SET body_json = '{\"detail\":\"edited\"}' WHERE object_id = 'audit-1'",
        [],
    )
    .unwrap();
    raw.execute(
        "DELETE FROM authority_objects WHERE object_id = 'audit-2'",
        [],
    )
    .unwrap();
    drop(raw);
    let meta = SqliteMetaStore::open_at(&path).unwrap();
    let problems = meta.verify_audit_chain().unwrap().problems;
    assert!(
        problems
            .iter()
            .any(|p| p.contains("audit-1") && p.contains("differs")),
        "{problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("audit-2") && p.contains("missing")),
        "{problems:?}"
    );
    drop(meta);

    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE audit_chain SET body_digest_hex = 'x' WHERE seq = 1",
        [],
    )
    .unwrap();
    drop(raw);
    let meta = SqliteMetaStore::open_at(&path).unwrap();
    let problems = meta.verify_audit_chain().unwrap().problems;
    assert!(
        problems.iter().any(|p| p.contains("entry 1 was changed")),
        "{problems:?}"
    );
}
