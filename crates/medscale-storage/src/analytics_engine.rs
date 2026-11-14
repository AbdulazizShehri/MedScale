//! Bounded read-only SQL engine for the Spec 082 Analytics Gate.
//!
//! Each query runs on a private in-memory SQLite database that holds only
//! the bound snapshot tables. Defenses, in order:
//! 1. text screen: the statement must start with `SELECT` or `WITH`, and
//!    the words `attach`, `detach`, `pragma`, `vacuum`, `load_extension`,
//!    `sqlcipher_export`, `readfile`, `writefile` and `fts3_tokenizer`
//!    are refused anywhere in the text, as are `pragma_*` table-valued
//!    functions (even inside literals; conservative);
//! 2. SQLite must accept exactly one statement, and report it read-only;
//! 3. the connection is switched to `query_only` before the query runs;
//! 4. rows, columns, text size and wall time are bounded; an interrupt
//!    ends a query at the time limit;
//! 5. SQLite run-time limits: no attached databases, and no string, BLOB
//!    or row larger than the engine value bound, so a query cannot build a
//!    giant value in memory before the result checks run. The connection
//!    is also in SQLite's `defensive` mode.
//!
//! The engine never opens a file, and it never sees vault storage: Core
//! passes decoded, digest-verified snapshot rows in.

use std::sync::mpsc;
use std::time::Duration;

use medscale_contracts::analytics::{
    QueryDenyReason, RESULT_COLUMNS_MAX, ResultColumn, ResultTableDoc,
};
use medscale_contracts::data_sources::{CELL_TEXT_MAX_BYTES, CellValue, FieldType, SchemaField};
use rusqlite::config::DbConfig;
use rusqlite::limits::Limit;
use rusqlite::types::{Value, ValueRef};

pub const ENGINE_NAME: &str = "sqlite";

/// Floor for the largest string, BLOB or row a query may build (16 MiB).
/// The bound is raised only to twice the widest bound input row, so any
/// loaded row can still be read, sorted and grouped.
pub const ENGINE_VALUE_MIN_BYTES: usize = 16 * 1024 * 1024;

/// The SQLite library version this build runs.
#[must_use]
pub fn engine_version() -> String {
    rusqlite::version().to_owned()
}

/// One bound input table.
#[derive(Debug, Clone, Copy)]
pub struct EngineTable<'a> {
    pub alias: &'a str,
    pub fields: &'a [SchemaField],
    pub rows: &'a [Vec<CellValue>],
}

/// Why the engine did not return a result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineRefusal {
    Denied(QueryDenyReason),
    TimedOut,
    /// Fixed text, never data values.
    Failed(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineOutput {
    pub table: ResultTableDoc,
    pub truncated: bool,
}

const FORBIDDEN_WORDS: &[&str] = &[
    "attach",
    "detach",
    "pragma",
    "vacuum",
    "load_extension",
    "sqlcipher_export",
    "readfile",
    "writefile",
    "fts3_tokenizer",
];

/// Strips leading whitespace and SQL comments.
fn skip_leading_trivia(sql: &str) -> &str {
    let mut rest = sql;
    loop {
        let trimmed = rest.trim_start();
        if let Some(after) = trimmed.strip_prefix("--") {
            rest = after.split_once('\n').map_or("", |(_, tail)| tail);
        } else if let Some(after) = trimmed.strip_prefix("/*") {
            rest = after.split_once("*/").map_or("", |(_, tail)| tail);
        } else {
            return trimmed;
        }
    }
}

/// Text screen (defense 1).
pub fn screen_sql(sql: &str) -> Result<(), QueryDenyReason> {
    let head = skip_leading_trivia(sql);
    let first: String = head
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect::<String>()
        .to_ascii_lowercase();
    if first != "select" && first != "with" {
        return Err(QueryDenyReason::NotReadOnly);
    }
    let lower = sql.to_ascii_lowercase();
    let words = lower.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'));
    for word in words {
        // `pragma_*` table-valued functions count as pragmas.
        if FORBIDDEN_WORDS.contains(&word) || word.starts_with("pragma_") {
            return Err(QueryDenyReason::ForbiddenConstruct);
        }
    }
    Ok(())
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

const fn sql_type(t: FieldType) -> &'static str {
    match t {
        FieldType::Integer | FieldType::Boolean => "INTEGER",
        FieldType::Float => "REAL",
        _ => "TEXT",
    }
}

fn to_sql_value(cell: &CellValue) -> Value {
    match cell {
        CellValue::Null => Value::Null,
        CellValue::Text(t) => Value::Text(t.clone()),
        CellValue::Integer(i) => Value::Integer(*i),
        CellValue::Float(f) => Value::Real(*f),
        CellValue::Boolean(b) => Value::Integer(i64::from(*b)),
    }
}

fn load_tables(
    conn: &rusqlite::Connection,
    tables: &[EngineTable<'_>],
) -> Result<(), EngineRefusal> {
    let fail = |_| EngineRefusal::Failed("could not load a bound table");
    for t in tables {
        if t.fields.is_empty() {
            return Err(EngineRefusal::Failed("a bound table has no columns"));
        }
        let columns = t
            .fields
            .iter()
            .map(|f| format!("{} {}", quote_ident(&f.name), sql_type(f.field_type)))
            .collect::<Vec<_>>()
            .join(", ");
        conn.execute_batch(&format!(
            "CREATE TABLE {} ({columns});",
            quote_ident(t.alias)
        ))
        .map_err(fail)?;
        let placeholders = vec!["?"; t.fields.len()].join(", ");
        let mut insert = conn
            .prepare(&format!(
                "INSERT INTO {} VALUES ({placeholders})",
                quote_ident(t.alias)
            ))
            .map_err(fail)?;
        for row in t.rows {
            if row.len() != t.fields.len() {
                return Err(EngineRefusal::Failed(
                    "a bound row does not match its schema",
                ));
            }
            let values: Vec<Value> = row.iter().map(to_sql_value).collect();
            insert
                .execute(rusqlite::params_from_iter(values))
                .map_err(fail)?;
        }
    }
    Ok(())
}

/// Upper estimate of one bound row's record size in bytes.
fn row_bytes(row: &[CellValue]) -> usize {
    row.iter()
        .map(|cell| match cell {
            CellValue::Text(t) => t.len() + 16,
            _ => 16,
        })
        .sum()
}

/// Defense 5: run-time limits and defensive mode, set after the bound
/// tables are loaded.
fn restrict(conn: &rusqlite::Connection, tables: &[EngineTable<'_>]) -> Result<(), EngineRefusal> {
    let widest = tables
        .iter()
        .flat_map(|t| t.rows.iter())
        .map(|r| row_bytes(r.as_slice()))
        .max()
        .unwrap_or(0);
    let value_bound =
        i32::try_from(ENGINE_VALUE_MIN_BYTES.max(widest.saturating_mul(2))).unwrap_or(i32::MAX);
    let fail = |_| EngineRefusal::Failed("engine unavailable");
    conn.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0)
        .map_err(fail)?;
    conn.set_limit(Limit::SQLITE_LIMIT_LENGTH, value_bound)
        .map_err(fail)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)
        .map_err(fail)?;
    conn.execute_batch("PRAGMA query_only = ON;")
        .map_err(fail)?;
    Ok(())
}

fn observed(prev: Option<&str>, next: &str) -> String {
    match prev {
        None | Some("null") => next.to_owned(),
        Some(p) if p == next || next == "null" => p.to_owned(),
        Some(_) => "mixed".to_owned(),
    }
}

fn to_cell(v: ValueRef<'_>) -> Result<(CellValue, &'static str), EngineRefusal> {
    Ok(match v {
        ValueRef::Null => (CellValue::Null, "null"),
        ValueRef::Integer(i) => (CellValue::Integer(i), "integer"),
        ValueRef::Real(f) => {
            if !f.is_finite() {
                return Err(EngineRefusal::Failed("a result value is not finite"));
            }
            (CellValue::Float(f), "float")
        }
        ValueRef::Text(bytes) => {
            let text = std::str::from_utf8(bytes)
                .map_err(|_| EngineRefusal::Failed("a result text is not UTF-8"))?;
            if text.len() > CELL_TEXT_MAX_BYTES || text.contains('\0') {
                return Err(EngineRefusal::Failed("a result text exceeds its bound"));
            }
            (CellValue::Text(text.to_owned()), "text")
        }
        ValueRef::Blob(_) => return Err(EngineRefusal::Failed("binary results are not supported")),
    })
}

fn is_interrupt(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::OperationInterrupted
    )
}

fn is_too_big(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::TooBig
    )
}

const TOO_BIG: &str = "a value exceeds the engine bound";

/// Steps the prepared statement and converts rows (defense 4 bounds).
fn collect_rows(
    stmt: &mut rusqlite::Statement<'_>,
    values: Vec<Value>,
    names: &[String],
    max_rows: u32,
) -> Result<EngineOutput, EngineRefusal> {
    let column_count = names.len();
    let mut rows = stmt
        .query(rusqlite::params_from_iter(values))
        .map_err(|e| {
            if is_interrupt(&e) {
                EngineRefusal::TimedOut
            } else if is_too_big(&e) {
                EngineRefusal::Failed(TOO_BIG)
            } else {
                EngineRefusal::Failed("query could not start")
            }
        })?;
    let mut out: Vec<Vec<CellValue>> = Vec::new();
    let mut types: Vec<Option<String>> = vec![None; column_count];
    let mut truncated = false;
    loop {
        let row = match rows.next() {
            Ok(Some(row)) => row,
            Ok(None) => break,
            Err(e) if is_interrupt(&e) => return Err(EngineRefusal::TimedOut),
            Err(e) if is_too_big(&e) => return Err(EngineRefusal::Failed(TOO_BIG)),
            Err(_) => return Err(EngineRefusal::Failed("query failed while running")),
        };
        if out.len() as u64 >= u64::from(max_rows) {
            truncated = true;
            break;
        }
        let mut cells = Vec::with_capacity(column_count);
        for (i, ty) in types.iter_mut().enumerate() {
            let value = row
                .get_ref(i)
                .map_err(|_| EngineRefusal::Failed("query failed while running"))?;
            let (cell, kind) = to_cell(value)?;
            *ty = Some(observed(ty.as_deref(), kind));
            cells.push(cell);
        }
        out.push(cells);
    }
    let columns = names
        .iter()
        .zip(types)
        .map(|(name, ty)| ResultColumn {
            name: name.clone(),
            observed_type: ty.unwrap_or_else(|| "null".to_owned()),
        })
        .collect();
    Ok(EngineOutput {
        table: ResultTableDoc { columns, rows: out },
        truncated,
    })
}

/// Runs one read-only statement over the bound tables. `params` are bound
/// positionally (`?1`, `?2`, ...) and never spliced into SQL.
pub fn run_readonly_query(
    tables: &[EngineTable<'_>],
    sql: &str,
    params: &[CellValue],
    max_rows: u32,
    timeout: Duration,
) -> Result<EngineOutput, EngineRefusal> {
    screen_sql(sql).map_err(EngineRefusal::Denied)?;
    let conn = rusqlite::Connection::open_in_memory()
        .map_err(|_| EngineRefusal::Failed("engine unavailable"))?;
    load_tables(&conn, tables)?;
    restrict(&conn, tables)?;

    let mut stmt = match conn.prepare(sql) {
        Ok(stmt) => stmt,
        Err(rusqlite::Error::MultipleStatement) => {
            return Err(EngineRefusal::Denied(QueryDenyReason::MultipleStatements));
        }
        Err(e) if e.to_string().contains("no such table") => {
            return Err(EngineRefusal::Denied(QueryDenyReason::UnknownTable));
        }
        Err(_) => return Err(EngineRefusal::Denied(QueryDenyReason::SyntaxError)),
    };
    if !stmt.readonly() {
        return Err(EngineRefusal::Denied(QueryDenyReason::NotReadOnly));
    }
    let column_count = stmt.column_count();
    if column_count == 0 || column_count > RESULT_COLUMNS_MAX {
        return Err(EngineRefusal::Failed("result column count out of bounds"));
    }
    let names: Vec<String> = stmt
        .column_names()
        .iter()
        .map(|n| (*n).to_owned())
        .collect();

    // Defense 4: interrupt at the time limit.
    let interrupt = conn.get_interrupt_handle();
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let watchdog = std::thread::spawn(move || {
        if done_rx.recv_timeout(timeout) == Err(mpsc::RecvTimeoutError::Timeout) {
            interrupt.interrupt();
        }
    });

    let values: Vec<Value> = params.iter().map(to_sql_value).collect();
    let outcome = collect_rows(&mut stmt, values, &names, max_rows);
    let _ = done_tx.send(());
    let _ = watchdog.join();
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str, t: FieldType) -> SchemaField {
        SchemaField {
            name: name.to_owned(),
            field_type: t,
            nullable: true,
            declared_unit: None,
        }
    }

    fn fixture() -> (Vec<SchemaField>, Vec<Vec<CellValue>>) {
        (
            vec![
                field("id", FieldType::Integer),
                field("age", FieldType::Integer),
                field("ldl", FieldType::Float),
                field("sex", FieldType::Text),
            ],
            vec![
                vec![
                    CellValue::Integer(1),
                    CellValue::Integer(34),
                    CellValue::Float(3.1),
                    CellValue::Text("f".to_owned()),
                ],
                vec![
                    CellValue::Integer(2),
                    CellValue::Integer(71),
                    CellValue::Float(4.4),
                    CellValue::Text("m".to_owned()),
                ],
                vec![
                    CellValue::Integer(3),
                    CellValue::Integer(58),
                    CellValue::Null,
                    CellValue::Text("f".to_owned()),
                ],
            ],
        )
    }

    fn run(sql: &str) -> Result<EngineOutput, EngineRefusal> {
        let (fields, rows) = fixture();
        let t = EngineTable {
            alias: "p",
            fields: &fields,
            rows: &rows,
        };
        run_readonly_query(&[t], sql, &[], 100, Duration::from_secs(5))
    }

    #[test]
    fn select_queries_run_and_are_typed() {
        let out =
            run("SELECT sex, COUNT(*) AS n, AVG(ldl) AS mean_ldl FROM p GROUP BY sex ORDER BY sex")
                .unwrap();
        assert!(!out.truncated);
        assert_eq!(out.table.columns[0].name, "sex");
        assert_eq!(out.table.columns[1].observed_type, "integer");
        assert_eq!(
            out.table.rows,
            vec![
                vec![
                    CellValue::Text("f".to_owned()),
                    CellValue::Integer(2),
                    CellValue::Float(3.1)
                ],
                vec![
                    CellValue::Text("m".to_owned()),
                    CellValue::Integer(1),
                    CellValue::Float(4.4)
                ],
            ]
        );
        let with = run(
            "-- note\nWITH old AS (SELECT * FROM p WHERE age > 50) SELECT id FROM old ORDER BY id",
        )
        .unwrap();
        assert_eq!(with.table.rows.len(), 2);
    }

    #[test]
    fn writes_ddl_and_escapes_are_refused() {
        let denied = |sql: &str| match run(sql) {
            Err(EngineRefusal::Denied(r)) => r,
            other => panic!("{sql}: {other:?}"),
        };
        for sql in [
            "DELETE FROM p",
            "UPDATE p SET age = 1",
            "INSERT INTO p VALUES (9, 9, 9, 'x')",
            "DROP TABLE p",
            "CREATE TABLE x (a)",
            "/* hi */ REPLACE INTO p VALUES (1,1,1,'f')",
        ] {
            assert_eq!(denied(sql), QueryDenyReason::NotReadOnly, "{sql}");
        }
        for sql in [
            "SELECT * FROM p; ATTACH DATABASE '/tmp/x.db' AS x",
            "SELECT * FROM pragma_table_info('p')",
            "SELECT load_extension('evil')",
            "SELECT sqlcipher_export('x')",
            "WITH a AS (SELECT 1) SELECT * FROM a -- vacuum",
        ] {
            assert_eq!(denied(sql), QueryDenyReason::ForbiddenConstruct, "{sql}");
        }
        assert_eq!(
            denied("SELECT 1 FROM p; SELECT 2 FROM p"),
            QueryDenyReason::MultipleStatements
        );
        assert_eq!(
            denied("SELECT * FROM secrets"),
            QueryDenyReason::UnknownTable
        );
        assert_eq!(denied("SELECT FROM WHERE"), QueryDenyReason::SyntaxError);
        assert_eq!(
            denied("WITH x AS (SELECT 1) DELETE FROM p"),
            QueryDenyReason::NotReadOnly
        );
    }

    #[test]
    fn results_are_bounded_and_honest() {
        let (fields, rows) = fixture();
        let t = EngineTable {
            alias: "p",
            fields: &fields,
            rows: &rows,
        };
        let capped = run_readonly_query(
            &[t],
            "SELECT id FROM p ORDER BY id",
            &[],
            2,
            Duration::from_secs(5),
        )
        .unwrap();
        assert!(capped.truncated);
        assert_eq!(capped.table.rows.len(), 2);
        let exact = run_readonly_query(
            &[t],
            "SELECT id FROM p ORDER BY id",
            &[],
            3,
            Duration::from_secs(5),
        )
        .unwrap();
        assert!(!exact.truncated);
        assert_eq!(
            run("SELECT x'00ff' FROM p"),
            Err(EngineRefusal::Failed("binary results are not supported"))
        );
        assert_eq!(
            run("SELECT 1e308 * 10 FROM p"),
            Err(EngineRefusal::Failed("a result value is not finite"))
        );
        let mixed = run("SELECT CASE WHEN id = 1 THEN 'a' ELSE id END AS v FROM p").unwrap();
        assert_eq!(mixed.table.columns[0].observed_type, "mixed");
    }

    #[test]
    fn parameters_are_bound_not_spliced() {
        let (fields, rows) = fixture();
        let t = EngineTable {
            alias: "p",
            fields: &fields,
            rows: &rows,
        };
        let out = run_readonly_query(
            &[t],
            "SELECT id FROM p WHERE sex = ?1 ORDER BY id",
            &[CellValue::Text("f' OR '1'='1".to_owned())],
            10,
            Duration::from_secs(5),
        )
        .unwrap();
        assert!(out.table.rows.is_empty(), "the value is data, not SQL");
    }

    #[test]
    fn statement_tricks_are_refused() {
        let denied = |sql: &str| match run(sql) {
            Err(EngineRefusal::Denied(r)) => r,
            other => panic!("{sql}: {other:?}"),
        };
        for sql in [
            "SELECT 1 FROM p; DELETE FROM p",
            "SELECT 1 FROM p /* c */ ; SELECT 2 FROM p",
            "SELECT 1 FROM p;SELECT 2",
        ] {
            assert_eq!(denied(sql), QueryDenyReason::MultipleStatements, "{sql}");
        }
        for sql in [
            "PrAgMa query_only = OFF",
            "BEGIN",
            "SAVEPOINT s",
            "EXPLAIN SELECT 1",
            "/* SELECT */ DROP TABLE p",
        ] {
            assert_eq!(denied(sql), QueryDenyReason::NotReadOnly, "{sql}");
        }
        for sql in [
            "SELECT 1 FROM p; PRAGMA writable_schema = ON",
            "SELECT 'attach' FROM p",
            "SELECT * FROM PRAGMA_database_list",
            "SELECT readfile('/etc/passwd')",
            "SELECT fts3_tokenizer('simple')",
            "WITH x AS (SELECT 1) SELECT * FROM x; VACUUM INTO 'f.db'",
            "SELECT 1 FROM p; DETACH main",
        ] {
            assert_eq!(denied(sql), QueryDenyReason::ForbiddenConstruct, "{sql}");
        }
        for sql in [
            "WITH x AS (SELECT 1) UPDATE p SET age = 0",
            "WITH x AS (SELECT 1) INSERT INTO p SELECT * FROM p",
        ] {
            assert_eq!(denied(sql), QueryDenyReason::NotReadOnly, "{sql}");
        }
        // A trailing semicolon or comment is still one statement.
        assert_eq!(
            run("SELECT id FROM p; -- done").unwrap().table.rows.len(),
            3
        );
        // The engine database holds only the bound table.
        let schema = run("SELECT name FROM sqlite_schema ORDER BY name").unwrap();
        assert_eq!(
            schema.table.rows,
            vec![vec![CellValue::Text("p".to_owned())]]
        );
    }

    #[test]
    fn giant_values_and_wide_results_are_bounded() {
        let failed = |sql: &str| match run(sql) {
            Err(EngineRefusal::Failed(m)) => m,
            other => panic!("{sql}: {other:?}"),
        };
        // SQLite refuses to build the value at all (SQLITE_LIMIT_LENGTH).
        assert_eq!(
            failed("SELECT length(zeroblob(1000000000)) FROM p"),
            TOO_BIG
        );
        assert_eq!(
            failed("SELECT length(hex(zeroblob(20000000))) FROM p"),
            TOO_BIG
        );
        // Growth past the bound inside a function is refused too.
        assert_eq!(
            failed("SELECT length(replace(hex(zeroblob(5000000)), '0', '0000')) FROM p"),
            TOO_BIG
        );
        // Values under the engine bound but over the cell bound.
        assert_eq!(
            failed("SELECT hex(zeroblob(40000)) FROM p"),
            "a result text exceeds its bound"
        );
        assert_eq!(
            failed("SELECT char(0) FROM p"),
            "a result text exceeds its bound"
        );
        assert_eq!(
            failed("SELECT CAST(x'c328' AS TEXT) FROM p"),
            "a result text is not UTF-8"
        );
        let wide = format!(
            "SELECT {} FROM p",
            vec!["1"; RESULT_COLUMNS_MAX + 1].join(", ")
        );
        assert_eq!(failed(&wide), "result column count out of bounds");
        let widest = format!("SELECT {} FROM p", vec!["1"; RESULT_COLUMNS_MAX].join(", "));
        assert_eq!(
            run(&widest).unwrap().table.columns.len(),
            RESULT_COLUMNS_MAX
        );
    }

    #[test]
    fn engine_connection_refuses_escapes_without_the_text_screen() {
        let (fields, rows) = fixture();
        let t = EngineTable {
            alias: "p",
            fields: &fields,
            rows: &rows,
        };
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        load_tables(&conn, &[t]).unwrap();
        restrict(&conn, &[t]).unwrap();
        for sql in [
            "ATTACH DATABASE ':memory:' AS x",
            "DELETE FROM p",
            "CREATE TEMP TABLE t (a)",
            "PRAGMA writable_schema = ON; UPDATE sqlite_schema SET sql = ''",
        ] {
            assert!(conn.execute_batch(sql).is_err(), "{sql}");
        }
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM p", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn cartesian_explosions_time_out() {
        let (fields, rows) = fixture();
        let t = EngineTable {
            alias: "p",
            fields: &fields,
            rows: &rows,
        };
        // 3^24 row combinations: far beyond the time limit.
        let from = (0..24)
            .map(|i| format!("p AS t{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let r = run_readonly_query(
            &[t],
            &format!("SELECT COUNT(*) FROM {from}"),
            &[],
            10,
            Duration::from_millis(200),
        );
        assert_eq!(r, Err(EngineRefusal::TimedOut));
    }

    #[test]
    fn runaway_queries_time_out() {
        let r = run_readonly_query(
            &[],
            "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c) SELECT COUNT(*) FROM c",
            &[],
            10,
            Duration::from_millis(200),
        );
        assert_eq!(r, Err(EngineRefusal::TimedOut));
    }
}
