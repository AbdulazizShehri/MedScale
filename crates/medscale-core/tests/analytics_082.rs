//! Spec 082 Analytics Gate Core authority integration tests.
//!
//! Every call goes through `CliSession` -> `CoreFacade::dispatch`. Inputs are
//! synthetic CSV files imported as Spec 075 snapshots.

use std::path::PathBuf;

use medscale_contracts::analytics::{
    CohortCriterion, CohortOp, InputReproducibility, QueryDenyReason, QueryOrigin, QueryOutcome,
    QueryRequest, ReplayVerdict, StatisticKind, StatisticValue, ViewBinding,
};
use medscale_contracts::data_sources::{CellValue, LocalFileFormat, SourceLocator};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::{DigestSha256, OpaqueId};
use medscale_core::CliSession;

const LABS: &str = "id,age,ldl,sex\n1,34,3.1,f\n2,71,4.4,m\n3,58,,f\n4,45,5.0,m\n";

struct Lab {
    s: CliSession,
    dir: PathBuf,
    project: OpaqueId,
    source: OpaqueId,
    snapshot: OpaqueId,
}

fn setup(name: &str) -> Lab {
    let dir = std::env::temp_dir().join(format!("medscale-082c-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("labs.csv"), LABS).unwrap();
    let mut s = CliSession::connect(&format!("vault-082-{name}")).unwrap();
    s.open_synthetic_vault(&dir.display().to_string()).unwrap();
    let project = s
        .project_create("study".to_owned(), None)
        .unwrap()
        .header
        .id;
    let source = s
        .data_source_create(
            project.clone(),
            "labs".to_owned(),
            SourceLocator::LocalPath {
                path: "labs.csv".to_owned(),
                format: LocalFileFormat::Csv,
            },
            None,
        )
        .unwrap()
        .header
        .id;
    let (snapshot, _) = s.snapshot_import(source.clone()).unwrap();
    Lab {
        s,
        dir,
        project,
        source,
        snapshot: snapshot.header.id,
    }
}

fn request(lab: &Lab, sql: &str) -> QueryRequest {
    QueryRequest {
        project_id: lab.project.clone(),
        sql: sql.to_owned(),
        bindings: vec![ViewBinding {
            alias: "labs".to_owned(),
            snapshot_id: lab.snapshot.clone(),
        }],
        max_rows: None,
    }
}

fn value(v: &StatisticValue) -> f64 {
    match v {
        StatisticValue::Value { value } => *value,
        other => panic!("{other:?}"),
    }
}

#[test]
fn queries_run_read_only_over_pinned_snapshots_and_replay() {
    let mut lab = setup("query");
    let req = request(
        &lab,
        "SELECT sex, COUNT(*) AS n, AVG(ldl) AS mean_ldl FROM labs GROUP BY sex ORDER BY sex",
    );
    let view = lab.s.analytics_query(req).unwrap();
    let r = &view.receipt;
    assert_eq!(r.outcome, QueryOutcome::Completed);
    assert_eq!(r.origin, QueryOrigin::SqlEditor);
    assert_eq!(r.reproducibility, InputReproducibility::Exact);
    assert_eq!(r.engine.engine, "sqlite");
    let snapshot = lab.s.snapshot_get(lab.snapshot.clone()).unwrap();
    assert_eq!(r.inputs[0].content_digest, snapshot.content_digest);
    assert_eq!(r.inputs[0].row_count, 4);
    let table = view.table.unwrap();
    assert_eq!(
        table.rows,
        vec![
            vec![
                CellValue::Text("f".to_owned()),
                CellValue::Integer(2),
                CellValue::Float(3.1)
            ],
            vec![
                CellValue::Text("m".to_owned()),
                CellValue::Integer(2),
                CellValue::Float(4.7)
            ],
        ]
    );
    assert_eq!(r.result_digest.as_ref(), Some(&table.digest()));

    let (stored, doc) = lab
        .s
        .analytics_result_get(r.result_id.clone().unwrap())
        .unwrap();
    assert_eq!(doc, table);
    assert_eq!(stored.derived_from, vec![lab.snapshot.clone()]);
    assert!(!stored.truncated);

    let replay = lab.s.analytics_replay(r.header.id.clone()).unwrap();
    assert_eq!(replay.verdict, ReplayVerdict::Reproduced);
    assert_eq!(replay.replay_digest, r.result_digest);

    // The query did not change the snapshot.
    assert_eq!(lab.s.snapshot_get(lab.snapshot.clone()).unwrap(), snapshot);
    let receipts = lab.s.analytics_receipt_list(lab.project.clone()).unwrap();
    assert_eq!(receipts.len(), 1);
}

#[test]
fn writes_escapes_and_foreign_inputs_are_refused_with_receipts() {
    let mut lab = setup("deny");
    let before = lab.s.snapshot_get(lab.snapshot.clone()).unwrap();
    for (sql, reason) in [
        ("DELETE FROM labs", QueryDenyReason::NotReadOnly),
        ("UPDATE labs SET age = 0", QueryDenyReason::NotReadOnly),
        ("DROP TABLE labs", QueryDenyReason::NotReadOnly),
        (
            "SELECT 1 FROM labs; ATTACH DATABASE 'x.db' AS x",
            QueryDenyReason::ForbiddenConstruct,
        ),
        (
            "SELECT * FROM pragma_table_list",
            QueryDenyReason::ForbiddenConstruct,
        ),
        (
            "SELECT 1 FROM labs; SELECT 2 FROM labs",
            QueryDenyReason::MultipleStatements,
        ),
        ("SELECT * FROM audit_log", QueryDenyReason::UnknownTable),
        ("SELECT FROM", QueryDenyReason::SyntaxError),
    ] {
        let v = lab.s.analytics_query(request(&lab, sql)).unwrap();
        assert_eq!(v.receipt.outcome, QueryOutcome::Denied, "{sql}");
        assert_eq!(v.receipt.deny_reason, Some(reason), "{sql}");
        assert!(v.table.is_none() && v.receipt.result_id.is_none());
    }
    let mut bad_alias = request(&lab, "SELECT 1");
    bad_alias.bindings[0].alias = "sqlite_master".to_owned();
    assert_eq!(
        lab.s
            .analytics_query(bad_alias)
            .unwrap()
            .receipt
            .deny_reason,
        Some(QueryDenyReason::BadBinding)
    );
    let mut missing = request(&lab, "SELECT * FROM labs");
    missing.bindings[0].snapshot_id = OpaqueId::new("snap-missing");
    assert_eq!(
        lab.s.analytics_query(missing).unwrap().receipt.deny_reason,
        Some(QueryDenyReason::SnapshotUnavailable)
    );

    // A snapshot of another Project cannot be bound here.
    let other = lab
        .s
        .project_create("other".to_owned(), None)
        .unwrap()
        .header
        .id;
    let mut foreign = request(&lab, "SELECT * FROM labs");
    foreign.project_id = other;
    let v = lab.s.analytics_query(foreign).unwrap();
    assert_eq!(
        v.receipt.deny_reason,
        Some(QueryDenyReason::SnapshotUnavailable)
    );
    assert!(v.receipt.inputs.is_empty());

    assert_eq!(lab.s.snapshot_get(lab.snapshot.clone()).unwrap(), before);
    assert_eq!(
        lab.s
            .analytics_receipt_list(lab.project.clone())
            .unwrap()
            .len(),
        10
    );
    let denied = lab.s.analytics_receipt_list(lab.project.clone()).unwrap()[0].clone();
    assert_eq!(
        lab.s.analytics_replay(denied.header.id).unwrap().verdict,
        ReplayVerdict::NotReplayable
    );
}

#[test]
fn truncation_is_explicit_and_never_the_full_result() {
    let mut lab = setup("truncate");
    let mut req = request(&lab, "SELECT id FROM labs ORDER BY id");
    req.max_rows = Some(2);
    let v = lab.s.analytics_query(req).unwrap();
    assert_eq!(v.receipt.outcome, QueryOutcome::Truncated);
    assert_eq!(v.receipt.row_count, 2);
    let (stored, _) = lab
        .s
        .analytics_result_get(v.receipt.result_id.clone().unwrap())
        .unwrap();
    assert!(stored.truncated);
    assert_eq!(
        lab.s.analytics_replay(v.receipt.header.id).unwrap().verdict,
        ReplayVerdict::Reproduced
    );
    let mut too_many = request(&lab, "SELECT id FROM labs");
    too_many.max_rows = Some(0);
    assert_eq!(
        lab.s.analytics_query(too_many).unwrap().receipt.deny_reason,
        Some(QueryDenyReason::BadBinding)
    );
}

#[test]
fn a_refresh_never_mutates_a_completed_run() {
    let mut lab = setup("refresh");
    let v = lab
        .s
        .analytics_query(request(&lab, "SELECT COUNT(*) AS n FROM labs"))
        .unwrap();
    assert_eq!(
        v.table.as_ref().unwrap().rows,
        vec![vec![CellValue::Integer(4)]]
    );
    std::fs::write(lab.dir.join("labs.csv"), format!("{LABS}5,80,6.2,f\n")).unwrap();
    let (newer, _) = lab.s.snapshot_import(lab.source.clone()).unwrap();
    assert_ne!(newer.header.id, lab.snapshot);

    // The old receipt still reproduces against its pinned snapshot.
    assert_eq!(
        lab.s
            .analytics_replay(v.receipt.header.id.clone())
            .unwrap()
            .verdict,
        ReplayVerdict::Reproduced
    );
    let mut fresh = request(&lab, "SELECT COUNT(*) AS n FROM labs");
    fresh.bindings[0].snapshot_id = newer.header.id;
    let w = lab.s.analytics_query(fresh).unwrap();
    assert_eq!(w.table.unwrap().rows, vec![vec![CellValue::Integer(5)]]);
    assert_ne!(
        w.receipt.inputs[0].content_digest,
        v.receipt.inputs[0].content_digest
    );
}

#[test]
fn cohorts_bind_values_and_reproduce() {
    let mut lab = setup("cohort");
    let cohort = lab
        .s
        .analytics_cohort_create(
            lab.project.clone(),
            "older women".to_owned(),
            lab.snapshot.clone(),
            vec![
                CohortCriterion {
                    field: "age".to_owned(),
                    op: CohortOp::Ge,
                    value: Some(CellValue::Integer(40)),
                },
                CohortCriterion {
                    field: "sex".to_owned(),
                    op: CohortOp::Eq,
                    value: Some(CellValue::Text("f".to_owned())),
                },
            ],
        )
        .unwrap();
    let v = lab
        .s
        .analytics_cohort_run(cohort.header.id.clone(), None)
        .unwrap();
    assert_eq!(v.receipt.origin, QueryOrigin::CohortBuilder);
    assert_eq!(v.receipt.cohort_id.as_ref(), Some(&cohort.header.id));
    let rows = v.table.unwrap().rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], CellValue::Integer(3));
    assert_eq!(
        lab.s.analytics_replay(v.receipt.header.id).unwrap().verdict,
        ReplayVerdict::Reproduced
    );

    let injection = lab
        .s
        .analytics_cohort_create(
            lab.project.clone(),
            "injection".to_owned(),
            lab.snapshot.clone(),
            vec![CohortCriterion {
                field: "sex".to_owned(),
                op: CohortOp::Eq,
                value: Some(CellValue::Text("f' OR '1'='1".to_owned())),
            }],
        )
        .unwrap();
    let v = lab
        .s
        .analytics_cohort_run(injection.header.id, None)
        .unwrap();
    assert_eq!(v.receipt.outcome, QueryOutcome::Completed);
    assert!(
        v.table.unwrap().rows.is_empty(),
        "the value is data, not SQL"
    );

    assert!(matches!(
        lab.s.analytics_cohort_create(
            lab.project.clone(),
            "bad".to_owned(),
            lab.snapshot.clone(),
            vec![CohortCriterion {
                field: "no_such_field".to_owned(),
                op: CohortOp::IsNull,
                value: None,
            }],
        ),
        Err(AuthorityError::InvalidArgument { .. })
    ));
    // Type mismatches are refused, not left to match nothing.
    for (field, value) in [
        ("age", CellValue::Text("40".to_owned())),
        ("sex", CellValue::Integer(1)),
        ("ldl", CellValue::Boolean(true)),
    ] {
        assert!(
            matches!(
                lab.s.analytics_cohort_create(
                    lab.project.clone(),
                    "mismatch".to_owned(),
                    lab.snapshot.clone(),
                    vec![CohortCriterion {
                        field: field.to_owned(),
                        op: CohortOp::Eq,
                        value: Some(value),
                    }],
                ),
                Err(AuthorityError::InvalidArgument { .. })
            ),
            "{field}"
        );
    }
    // Malformed operators and null comparisons are refused.
    for (op, value) in [
        (CohortOp::Eq, None),
        (CohortOp::IsNull, Some(CellValue::Integer(1))),
        (CohortOp::Eq, Some(CellValue::Null)),
    ] {
        assert!(matches!(
            lab.s.analytics_cohort_create(
                lab.project.clone(),
                "malformed".to_owned(),
                lab.snapshot.clone(),
                vec![CohortCriterion {
                    field: "age".to_owned(),
                    op,
                    value,
                }],
            ),
            Err(AuthorityError::InvalidArgument { .. })
        ));
    }
    assert_eq!(
        lab.s
            .analytics_cohort_list(lab.project.clone())
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn statistics_are_hand_checked_and_honest() {
    let mut lab = setup("stats");
    let v = lab
        .s
        .analytics_query(request(&lab, "SELECT ldl, sex FROM labs ORDER BY id"))
        .unwrap();
    let result = v.receipt.result_id.unwrap();
    let stats = lab
        .s
        .analytics_statistics(
            result.clone(),
            "ldl".to_owned(),
            StatisticKind::ALL.to_vec(),
        )
        .unwrap();
    let by = |k: StatisticKind| stats.iter().find(|s| s.kind == k).unwrap().value.clone();
    // ldl = 3.1, 4.4, (missing), 5.0 -> mean 12.5/3, median 4.4.
    assert!((value(&by(StatisticKind::Count)) - 3.0).abs() < 1e-12);
    assert!((value(&by(StatisticKind::Missing)) - 1.0).abs() < 1e-12);
    assert!((value(&by(StatisticKind::Mean)) - 12.5 / 3.0).abs() < 1e-9);
    assert!((value(&by(StatisticKind::Median)) - 4.4).abs() < 1e-12);
    assert!((value(&by(StatisticKind::Min)) - 3.1).abs() < 1e-12);
    assert!((value(&by(StatisticKind::Max)) - 5.0).abs() < 1e-12);
    // Sample variance: deviations -1.0667, 0.2333, 0.8333 -> ss = 1.9267, /2.
    let mean = 12.5 / 3.0;
    let ss: f64 = [3.1_f64, 4.4, 5.0].iter().map(|x| (x - mean).powi(2)).sum();
    assert!((value(&by(StatisticKind::StdDev)) - (ss / 2.0).sqrt()).abs() < 1e-9);
    assert!(
        stats
            .iter()
            .all(|s| s.result_digest == v.receipt.result_digest.clone().unwrap())
    );

    let text = lab
        .s
        .analytics_statistics(result.clone(), "sex".to_owned(), vec![StatisticKind::Mean])
        .unwrap();
    assert_eq!(text[0].value, StatisticValue::NotNumeric);
    assert!(
        lab.s
            .analytics_statistics(result, "nope".to_owned(), vec![StatisticKind::Mean])
            .is_err()
    );
}

#[test]
fn analytics_state_survives_reopen() {
    let mut lab = setup("reopen");
    let v = lab
        .s
        .analytics_query(request(&lab, "SELECT MAX(age) AS oldest FROM labs"))
        .unwrap();
    let dir = lab.dir.clone();
    drop(lab.s);
    let mut s = CliSession::connect("vault-082-reopen").unwrap();
    s.open_synthetic_vault(&dir.display().to_string()).unwrap();
    assert_eq!(
        s.analytics_receipt_get(v.receipt.header.id.clone())
            .unwrap(),
        v.receipt
    );
    let (_, doc) = s
        .analytics_result_get(v.receipt.result_id.clone().unwrap())
        .unwrap();
    assert_eq!(doc.rows, vec![vec![CellValue::Integer(71)]]);
    assert_eq!(
        s.analytics_replay(v.receipt.header.id).unwrap().verdict,
        ReplayVerdict::Reproduced
    );
}

/// Rewrites one stored receipt body. The edited body stays self-consistent,
/// so only replay can notice the change.
fn edit_receipt(dir: &std::path::Path, id: &OpaqueId, edit: impl FnOnce(&mut serde_json::Value)) {
    let conn = rusqlite::Connection::open(dir.join("meta.sqlite3")).unwrap();
    let body: String = conn
        .query_row(
            "SELECT body_json FROM analytics_receipts WHERE receipt_id = ?1",
            [id.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&body).unwrap();
    edit(&mut value);
    conn.execute(
        "UPDATE analytics_receipts SET body_json = ?1 WHERE receipt_id = ?2",
        rusqlite::params![value.to_string(), id.as_str()],
    )
    .unwrap();
}

#[test]
fn replay_reports_moved_inputs_and_changed_results() {
    let mut lab = setup("replay-tamper");
    let moved = lab
        .s
        .analytics_query(request(&lab, "SELECT COUNT(*) AS n FROM labs"))
        .unwrap();
    let changed = lab
        .s
        .analytics_query(request(&lab, "SELECT MIN(age) AS youngest FROM labs"))
        .unwrap();
    let other = serde_json::to_value(DigestSha256::of(b"other")).unwrap();
    let dir = lab.dir.clone();
    drop(lab.s);
    edit_receipt(&dir, &moved.receipt.header.id, |v| {
        v["inputs"][0]["content_digest"] = other.clone();
    });
    edit_receipt(&dir, &changed.receipt.header.id, |v| {
        v["result_digest"] = other.clone();
    });
    let mut s = CliSession::connect("vault-082-replay-tamper").unwrap();
    s.open_synthetic_vault(&dir.display().to_string()).unwrap();
    // The pinned input no longer matches any stored snapshot: not re-run.
    let report = s.analytics_replay(moved.receipt.header.id).unwrap();
    assert_eq!(report.verdict, ReplayVerdict::InputUnavailable);
    assert!(report.replay_digest.is_none());
    // The recorded result no longer matches what the inputs produce.
    let report = s.analytics_replay(changed.receipt.header.id).unwrap();
    assert_eq!(report.verdict, ReplayVerdict::Diverged);
    assert_eq!(report.replay_digest, changed.receipt.result_digest);
}
