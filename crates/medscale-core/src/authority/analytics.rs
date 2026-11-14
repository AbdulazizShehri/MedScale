//! Analytics Gate Core authority paths (Spec 082).
//!
//! Analytics runs on the Spec 075 data-source authority: it reads snapshots
//! only through the same scope checks and digest-verified loader, and it
//! never writes to a snapshot. Every query request that reaches a Project
//! leaves a `QueryReceipt` pinning the exact SQL, snapshot digests, engine
//! and limits; a completed query also leaves an immutable derived table.
//! The engine (`medscale_storage::analytics_engine`) is read-only SQLite
//! (founder decision 2026-09-23); R, Python and shell are not reachable.

use std::time::Duration;

use medscale_contracts::analytics::{
    ANALYTICS_SCHEMA_VERSION, CohortCriterion, CohortDefinition, DerivedTable, EngineIdentity,
    InputReproducibility, PinnedInput, QUERY_TIMEOUT_MS, QueryDenyReason, QueryOrigin,
    QueryOutcome, QueryReceipt, QueryRequest, QueryView, ReplayReport, ReplayVerdict,
    ResultTableDoc, StatisticKind, StatisticResult, StatisticValue, ViewBinding,
};
use medscale_contracts::data_sources::{
    CellValue, FieldType, SnapshotCanonicalDoc, SnapshotStatus,
};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::{DigestSha256, ObjectHeader, OpaqueId};
use medscale_storage::analytics_engine::{
    ENGINE_NAME, EngineRefusal, EngineTable, engine_version, run_readonly_query,
};
use medscale_storage::{MetaError, SnapshotRecord};

use super::data_sources::DataSources;

/// Table alias every cohort query uses for its bound snapshot.
pub const COHORT_ALIAS: &str = "cohort_input";

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

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Compiles a cohort into SQL plus bound parameters. Field names are quoted
/// identifiers and values are positional parameters; nothing from the
/// criteria is spliced into SQL text as a literal.
#[must_use]
pub fn compile_cohort(criteria: &[CohortCriterion]) -> (String, Vec<CellValue>) {
    let mut clauses = Vec::with_capacity(criteria.len());
    let mut params = Vec::new();
    for c in criteria {
        let field = quote_ident(&c.field);
        match &c.value {
            Some(value) if c.op.takes_value() => {
                params.push(value.clone());
                clauses.push(format!("{field} {} ?{}", c.op.sql(), params.len()));
            }
            _ => clauses.push(format!("{field} {}", c.op.sql())),
        }
    }
    (
        format!(
            "SELECT * FROM {COHORT_ALIAS} WHERE {}",
            clauses.join(" AND ")
        ),
        params,
    )
}

/// Whether a criterion value compares meaningfully with a field of this
/// type. SQLite would otherwise coerce or compare across storage classes
/// and silently match nothing.
const fn value_fits(field: FieldType, value: &CellValue) -> bool {
    matches!(
        (field, value),
        (
            FieldType::Integer | FieldType::Float,
            CellValue::Integer(_) | CellValue::Float(_)
        ) | (FieldType::Boolean, CellValue::Boolean(_))
            | (
                FieldType::Text | FieldType::Date | FieldType::Time | FieldType::DateTime,
                CellValue::Text(_)
            )
    )
}

fn numeric(cell: &CellValue) -> Option<f64> {
    match cell {
        CellValue::Integer(i) => Some(*i as f64),
        CellValue::Float(f) => Some(*f),
        _ => None,
    }
}

/// Computes one descriptive statistic. Missing values are counted, never
/// imputed; non-numeric columns get `not_numeric` for numeric statistics;
/// too few values give `insufficient`, never a silent zero.
#[must_use]
pub fn compute_statistic(values: &[CellValue], kind: StatisticKind) -> StatisticValue {
    let missing = values.iter().filter(|v| v.is_null()).count() as u64;
    let present = values.len() as u64 - missing;
    match kind {
        StatisticKind::Count => {
            return StatisticValue::Value {
                value: present as f64,
            };
        }
        StatisticKind::Missing => {
            return StatisticValue::Value {
                value: missing as f64,
            };
        }
        _ => {}
    }
    let nums: Vec<f64> = values
        .iter()
        .filter(|v| !v.is_null())
        .filter_map(numeric)
        .collect();
    if nums.len() as u64 != present {
        return StatisticValue::NotNumeric;
    }
    let needed = if kind == StatisticKind::StdDev { 2 } else { 1 };
    let n = nums.len() as u64;
    if n < needed {
        return StatisticValue::Insufficient {
            needed,
            available: n,
        };
    }
    let mean = nums.iter().sum::<f64>() / n as f64;
    let value = match kind {
        StatisticKind::Mean => mean,
        StatisticKind::StdDev => {
            let ss: f64 = nums.iter().map(|x| (x - mean) * (x - mean)).sum();
            (ss / (n - 1) as f64).sqrt()
        }
        StatisticKind::Min => nums.iter().copied().fold(f64::INFINITY, f64::min),
        StatisticKind::Max => nums.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        StatisticKind::Median => {
            let mut sorted = nums.clone();
            sorted.sort_by(f64::total_cmp);
            let mid = sorted.len() / 2;
            if sorted.len() % 2 == 1 {
                sorted[mid]
            } else {
                (sorted[mid - 1] + sorted[mid]) / 2.0
            }
        }
        StatisticKind::Count | StatisticKind::Missing => unreachable!("handled above"),
    };
    StatisticValue::Value { value }
}

/// A loaded binding: its pinned identity and decoded rows.
struct LoadedInput {
    pinned: PinnedInput,
    record: SnapshotRecord,
    doc: SnapshotCanonicalDoc,
}

impl DataSources<'_> {
    fn analytics_header(&self, id: OpaqueId) -> ObjectHeader {
        ObjectHeader {
            id,
            schema_version: ANALYTICS_SCHEMA_VERSION,
            realm_id: self.realm.clone(),
            authority_scope_id: self.scope.clone(),
        }
    }

    fn analytics_in_scope(&self, header: &ObjectHeader) -> Result<(), AuthorityError> {
        if header.realm_id != self.realm || header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(())
    }

    fn engine_identity(max_rows: u32) -> EngineIdentity {
        EngineIdentity {
            engine: ENGINE_NAME.to_owned(),
            version: engine_version(),
            max_rows,
            timeout_ms: QUERY_TIMEOUT_MS,
        }
    }

    /// Loads one binding of this Project, or `None` when it cannot be used
    /// (missing, another Project or scope, or unreadable).
    fn load_binding(&self, project_id: &OpaqueId, binding: &ViewBinding) -> Option<LoadedInput> {
        let record = self.scoped_snapshot(&binding.snapshot_id).ok()?;
        if &record.project_id != project_id {
            return None;
        }
        let doc = self.load_table(&record).ok()?;
        Some(LoadedInput {
            pinned: PinnedInput {
                alias: binding.alias.clone(),
                snapshot_id: record.snapshot.header.id.clone(),
                content_digest: record.snapshot.content_digest.clone(),
                schema_fingerprint: record.snapshot.schema_fingerprint.clone(),
                row_count: record.snapshot.row_count,
                complete: record.snapshot.status == SnapshotStatus::Complete,
            },
            record,
            doc,
        })
    }

    /// Runs one read-only query and records its receipt (and result).
    pub fn analytics_query(
        &mut self,
        request: QueryRequest,
        origin: QueryOrigin,
        cohort: Option<(OpaqueId, Vec<CellValue>)>,
    ) -> Result<QueryView, AuthorityError> {
        self.require_project(&request.project_id)?;
        let max_rows = request.effective_max_rows();
        let (cohort_id, params) = match cohort {
            Some((id, params)) => (Some(id), params),
            None => (None, Vec::new()),
        };
        let mut inputs: Vec<LoadedInput> = Vec::new();
        let mut refusal: Option<EngineRefusal> =
            request.validate().err().map(EngineRefusal::Denied);
        if refusal.is_none() {
            for b in &request.bindings {
                match self.load_binding(&request.project_id, b) {
                    Some(loaded) => inputs.push(loaded),
                    None => {
                        refusal = Some(EngineRefusal::Denied(QueryDenyReason::SnapshotUnavailable));
                        break;
                    }
                }
            }
        }
        let output = match refusal {
            Some(r) => Err(r),
            None => {
                let tables: Vec<EngineTable<'_>> = inputs
                    .iter()
                    .map(|i| EngineTable {
                        alias: &i.pinned.alias,
                        fields: &i.record.schema.fields,
                        rows: &i.doc.rows,
                    })
                    .collect();
                run_readonly_query(
                    &tables,
                    &request.sql,
                    &params,
                    max_rows,
                    Duration::from_millis(QUERY_TIMEOUT_MS),
                )
            }
        };
        let pinned: Vec<PinnedInput> = inputs.into_iter().map(|i| i.pinned).collect();
        let reproducibility = if pinned.iter().all(|p| p.complete) {
            InputReproducibility::Exact
        } else {
            InputReproducibility::PartialInputs
        };
        let receipt_id = self
            .meta()
            .alloc_analytics_id("analytics-receipt")
            .map_err(meta_err)?;
        let sql: String = request
            .sql
            .chars()
            .take(medscale_contracts::analytics::SQL_MAX_CHARS)
            .collect();
        let mut receipt = QueryReceipt {
            header: self.analytics_header(receipt_id.clone()),
            project_id: request.project_id.clone(),
            origin,
            sql_digest: DigestSha256::of(sql.as_bytes()),
            sql,
            inputs: pinned,
            reproducibility,
            engine: Self::engine_identity(max_rows),
            outcome: QueryOutcome::Denied,
            deny_reason: None,
            failure: None,
            result_id: None,
            result_digest: None,
            row_count: 0,
            column_count: 0,
            cohort_id,
        };
        let mut result: Option<(DerivedTable, ResultTableDoc)> = None;
        match output {
            Err(EngineRefusal::Denied(reason)) => receipt.deny_reason = Some(reason),
            Err(EngineRefusal::TimedOut) => {
                receipt.outcome = QueryOutcome::TimedOut;
                receipt.failure = Some("time limit reached".to_owned());
            }
            Err(EngineRefusal::Failed(message)) => {
                receipt.outcome = QueryOutcome::Failed;
                receipt.failure = Some(message.to_owned());
            }
            Ok(out) => {
                let result_id = self
                    .meta()
                    .alloc_analytics_id("analytics-result")
                    .map_err(meta_err)?;
                let digest = out.table.digest();
                let row_count = out.table.rows.len() as u64;
                let column_count = u32::try_from(out.table.columns.len()).unwrap_or(u32::MAX);
                receipt.outcome = if out.truncated {
                    QueryOutcome::Truncated
                } else {
                    QueryOutcome::Completed
                };
                receipt.result_id = Some(result_id.clone());
                receipt.result_digest = Some(digest.clone());
                receipt.row_count = row_count;
                receipt.column_count = column_count;
                result = Some((
                    DerivedTable {
                        header: self.analytics_header(result_id),
                        project_id: request.project_id.clone(),
                        receipt_id: receipt_id.clone(),
                        content_digest: digest,
                        row_count,
                        column_count,
                        derived_from: receipt
                            .inputs
                            .iter()
                            .map(|i| i.snapshot_id.clone())
                            .collect(),
                        truncated: out.truncated,
                    },
                    out.table,
                ));
            }
        }
        receipt.validate().map_err(|e| AuthorityError::Internal {
            message: format!("query receipt invariant: {e}"),
        })?;
        self.meta()
            .commit_query(&receipt, result.as_ref().map(|(t, d)| (t, d)))
            .map_err(meta_err)?;
        let mut targets = vec![receipt_id];
        if let Some((t, _)) = &result {
            targets.push(t.header.id.clone());
        }
        self.audit("analytics.query", targets)?;
        Ok(QueryView {
            receipt,
            table: result.map(|(_, doc)| doc),
        })
    }

    pub fn analytics_receipt(&self, id: &OpaqueId) -> Result<QueryReceipt, AuthorityError> {
        let r = self.meta().get_query_receipt(id).map_err(meta_err)?;
        self.analytics_in_scope(&r.header)?;
        Ok(r)
    }

    pub fn analytics_receipts(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<QueryReceipt>, AuthorityError> {
        self.require_project(project_id)?;
        self.meta()
            .list_query_receipts(project_id)
            .map_err(meta_err)
    }

    pub fn analytics_result(
        &self,
        id: &OpaqueId,
    ) -> Result<(DerivedTable, ResultTableDoc), AuthorityError> {
        let (table, doc) = self.meta().get_derived_table(id).map_err(meta_err)?;
        self.analytics_in_scope(&table.header)?;
        Ok((table, doc))
    }

    /// Re-runs a stored receipt against its pinned inputs and compares the
    /// result digest. Nothing is written.
    pub fn analytics_replay(&self, receipt_id: &OpaqueId) -> Result<ReplayReport, AuthorityError> {
        let receipt = self.analytics_receipt(receipt_id)?;
        let report = |verdict, replay_digest| ReplayReport {
            receipt_id: receipt.header.id.clone(),
            verdict,
            original_digest: receipt.result_digest.clone(),
            replay_digest,
        };
        if !receipt.outcome.has_result() {
            return Ok(report(ReplayVerdict::NotReplayable, None));
        }
        let mut loaded = Vec::new();
        for p in &receipt.inputs {
            let binding = ViewBinding {
                alias: p.alias.clone(),
                snapshot_id: p.snapshot_id.clone(),
            };
            match self.load_binding(&receipt.project_id, &binding) {
                Some(l) if l.pinned.content_digest == p.content_digest => loaded.push(l),
                _ => return Ok(report(ReplayVerdict::InputUnavailable, None)),
            }
        }
        let params = match &receipt.cohort_id {
            Some(id) => {
                let cohort = self.meta().get_cohort(id).map_err(meta_err)?;
                compile_cohort(&cohort.criteria).1
            }
            None => Vec::new(),
        };
        let tables: Vec<EngineTable<'_>> = loaded
            .iter()
            .map(|i| EngineTable {
                alias: &i.pinned.alias,
                fields: &i.record.schema.fields,
                rows: &i.doc.rows,
            })
            .collect();
        let replay = run_readonly_query(
            &tables,
            &receipt.sql,
            &params,
            receipt.engine.max_rows,
            Duration::from_millis(QUERY_TIMEOUT_MS),
        );
        Ok(match replay {
            Ok(out) => {
                let digest = out.table.digest();
                let verdict = if Some(&digest) == receipt.result_digest.as_ref() {
                    ReplayVerdict::Reproduced
                } else {
                    ReplayVerdict::Diverged
                };
                report(verdict, Some(digest))
            }
            Err(_) => report(ReplayVerdict::Diverged, None),
        })
    }

    /// Descriptive statistics over one column of a stored derived table.
    pub fn analytics_statistics(
        &self,
        result_id: &OpaqueId,
        column: &str,
        kinds: &[StatisticKind],
    ) -> Result<Vec<StatisticResult>, AuthorityError> {
        if kinds.is_empty() || kinds.len() > StatisticKind::ALL.len() {
            return Err(invalid("request 1-7 statistics"));
        }
        let (table, doc) = self.analytics_result(result_id)?;
        let index = doc
            .columns
            .iter()
            .position(|c| c.name == column)
            .ok_or_else(|| invalid("no such result column"))?;
        let values: Vec<CellValue> = doc
            .rows
            .iter()
            .map(|r| r.get(index).cloned())
            .collect::<Option<_>>()
            .ok_or_else(|| AuthorityError::Corrupt {
                message: "a result row is shorter than its columns".to_owned(),
            })?;
        Ok(kinds
            .iter()
            .map(|kind| StatisticResult {
                result_id: table.header.id.clone(),
                result_digest: table.content_digest.clone(),
                column: column.to_owned(),
                kind: *kind,
                value: compute_statistic(&values, *kind),
            })
            .collect())
    }

    /// Stores a cohort over one snapshot of the Project. Fields must exist.
    pub fn analytics_cohort_create(
        &mut self,
        project_id: OpaqueId,
        label: String,
        snapshot_id: OpaqueId,
        criteria: Vec<CohortCriterion>,
    ) -> Result<CohortDefinition, AuthorityError> {
        self.require_project(&project_id)?;
        let record = self.scoped_snapshot(&snapshot_id)?;
        if record.project_id != project_id {
            return Err(AuthorityError::WrongScope);
        }
        for c in &criteria {
            let field = record
                .schema
                .fields
                .iter()
                .find(|f| f.name == c.field)
                .ok_or_else(|| invalid(format!("no field named {:?}", c.field)))?;
            if let Some(value) = &c.value
                && !value.is_null()
                && !value_fits(field.field_type, value)
            {
                return Err(invalid(format!(
                    "the value for {:?} does not fit its {} type",
                    c.field,
                    field.field_type.as_str()
                )));
            }
        }
        let mut cohort = CohortDefinition {
            header: self.analytics_header(OpaqueId::new("pending")),
            project_id,
            label,
            snapshot_id,
            criteria,
        };
        cohort.validate().map_err(invalid)?;
        cohort.header = self.analytics_header(
            self.meta()
                .alloc_analytics_id("analytics-cohort")
                .map_err(meta_err)?,
        );
        self.meta().insert_cohort(&cohort).map_err(meta_err)?;
        self.audit("analytics.cohort.create", vec![cohort.header.id.clone()])?;
        Ok(cohort)
    }

    pub fn analytics_cohorts(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<CohortDefinition>, AuthorityError> {
        self.require_project(project_id)?;
        self.meta().list_cohorts(project_id).map_err(meta_err)
    }

    /// Runs a stored cohort as a read-only query with bound parameters.
    pub fn analytics_cohort_run(
        &mut self,
        cohort_id: &OpaqueId,
        max_rows: Option<u32>,
    ) -> Result<QueryView, AuthorityError> {
        let cohort = self.meta().get_cohort(cohort_id).map_err(meta_err)?;
        self.analytics_in_scope(&cohort.header)?;
        let (sql, params) = compile_cohort(&cohort.criteria);
        self.analytics_query(
            QueryRequest {
                project_id: cohort.project_id.clone(),
                sql,
                bindings: vec![ViewBinding {
                    alias: COHORT_ALIAS.to_owned(),
                    snapshot_id: cohort.snapshot_id.clone(),
                }],
                max_rows,
            },
            QueryOrigin::CohortBuilder,
            Some((cohort.header.id.clone(), params)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::analytics::CohortOp;

    fn col(values: &[Option<f64>]) -> Vec<CellValue> {
        values
            .iter()
            .map(|v| v.map_or(CellValue::Null, CellValue::Float))
            .collect()
    }

    fn value(v: &StatisticValue) -> f64 {
        match v {
            StatisticValue::Value { value } => *value,
            other => panic!("{other:?}"),
        }
    }

    /// Independent fixtures: every expected value was computed by hand.
    #[test]
    fn statistics_match_hand_computed_fixtures() {
        // 2, 4, 4, 4, 5, 5, 7, 9: mean 5, sample variance 32/7.
        let v = col(&[
            Some(2.0),
            Some(4.0),
            Some(4.0),
            Some(4.0),
            Some(5.0),
            Some(5.0),
            Some(7.0),
            Some(9.0),
            None,
        ]);
        assert!((value(&compute_statistic(&v, StatisticKind::Count)) - 8.0).abs() < 1e-12);
        assert!((value(&compute_statistic(&v, StatisticKind::Missing)) - 1.0).abs() < 1e-12);
        assert!((value(&compute_statistic(&v, StatisticKind::Mean)) - 5.0).abs() < 1e-12);
        assert!(
            (value(&compute_statistic(&v, StatisticKind::StdDev)) - (32.0_f64 / 7.0).sqrt()).abs()
                < 1e-12
        );
        assert!((value(&compute_statistic(&v, StatisticKind::Median)) - 4.5).abs() < 1e-12);
        assert!((value(&compute_statistic(&v, StatisticKind::Min)) - 2.0).abs() < 1e-12);
        assert!((value(&compute_statistic(&v, StatisticKind::Max)) - 9.0).abs() < 1e-12);
        let odd = col(&[Some(3.0), Some(1.0), Some(2.0)]);
        assert!((value(&compute_statistic(&odd, StatisticKind::Median)) - 2.0).abs() < 1e-12);
        let ints = vec![CellValue::Integer(1), CellValue::Integer(2)];
        assert!((value(&compute_statistic(&ints, StatisticKind::Mean)) - 1.5).abs() < 1e-12);
    }

    #[test]
    fn statistics_state_why_they_did_not_run() {
        let one = col(&[Some(3.0), None]);
        assert_eq!(
            compute_statistic(&one, StatisticKind::StdDev),
            StatisticValue::Insufficient {
                needed: 2,
                available: 1
            }
        );
        let none = col(&[None, None]);
        assert_eq!(
            compute_statistic(&none, StatisticKind::Mean),
            StatisticValue::Insufficient {
                needed: 1,
                available: 0
            }
        );
        assert!((value(&compute_statistic(&none, StatisticKind::Count))).abs() < 1e-12);
        let text = vec![CellValue::Text("a".to_owned()), CellValue::Integer(1)];
        assert_eq!(
            compute_statistic(&text, StatisticKind::Mean),
            StatisticValue::NotNumeric
        );
        assert!((value(&compute_statistic(&text, StatisticKind::Count)) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn cohorts_compile_to_bound_parameters() {
        let (sql, params) = compile_cohort(&[
            CohortCriterion {
                field: "age".to_owned(),
                op: CohortOp::Ge,
                value: Some(CellValue::Integer(18)),
            },
            CohortCriterion {
                field: "na\"me".to_owned(),
                op: CohortOp::Eq,
                value: Some(CellValue::Text("x' OR 1=1 --".to_owned())),
            },
            CohortCriterion {
                field: "ldl".to_owned(),
                op: CohortOp::IsNotNull,
                value: None,
            },
        ]);
        assert_eq!(
            sql,
            "SELECT * FROM cohort_input WHERE \"age\" >= ?1 AND \"na\"\"me\" = ?2 AND \"ldl\" IS NOT NULL"
        );
        assert_eq!(params.len(), 2);
        assert!(!sql.contains("OR 1=1"));
    }
}
