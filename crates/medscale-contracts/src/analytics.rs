//! Analytics Gate contracts (Spec 082).
//!
//! Reproducible, read-only analysis over exact immutable `DataSnapshot`
//! revisions. Every query binds snapshots by id and content digest, runs on
//! a bounded engine that refuses writes, DDL, attachment and pragmas, and
//! leaves a `QueryReceipt` naming the exact SQL, inputs, engine and limits.
//! Results are derived tables, never new sources of truth; nothing here
//! executes R, Python or shell code.

use serde::{Deserialize, Serialize};

use crate::data_sources::CellValue;
use crate::objects::{DigestSha256, ObjectHeader, OpaqueId};

/// Durable schema version for every Analytics object (Spec 082 v1).
pub const ANALYTICS_SCHEMA_VERSION: u32 = 1;

pub const SQL_MAX_CHARS: usize = 8_000;
pub const MAX_BINDINGS: usize = 8;
pub const ALIAS_MAX_CHARS: usize = 48;
pub const RESULT_ROWS_MAX: u32 = 10_000;
pub const RESULT_ROWS_DEFAULT: u32 = 1_000;
pub const RESULT_COLUMNS_MAX: usize = 128;
pub const QUERY_TIMEOUT_MS: u64 = 10_000;
pub const COHORT_CRITERIA_MAX: usize = 16;
pub const LABEL_MAX_CHARS: usize = 200;
pub const VALUE_MAX_CHARS: usize = 512;

macro_rules! closed_vocabulary {
    ($name:ident, $what:literal, { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            pub fn parse(value: &str) -> Result<Self, String> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    other => Err(format!(concat!("unknown ", $what, " {}"), other)),
                }
            }
        }
    };
}

/// Why a query was refused before or during execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryDenyReason {
    /// The statement is not a single `SELECT`/`WITH` query.
    NotReadOnly,
    MultipleStatements,
    /// `ATTACH`, `PRAGMA`, extension loading and similar are never allowed.
    ForbiddenConstruct,
    SqlTooLong,
    BadBinding,
    /// The query references a table that is not bound.
    UnknownTable,
    SnapshotUnavailable,
    SyntaxError,
}

closed_vocabulary!(QueryDenyReason, "query deny reason", {
    NotReadOnly => "not_read_only",
    MultipleStatements => "multiple_statements",
    ForbiddenConstruct => "forbidden_construct",
    SqlTooLong => "sql_too_long",
    BadBinding => "bad_binding",
    UnknownTable => "unknown_table",
    SnapshotUnavailable => "snapshot_unavailable",
    SyntaxError => "syntax_error",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryOutcome {
    Completed,
    /// Completed, but more rows existed than `max_rows`; only the first
    /// `max_rows` were kept (never presented as the full result).
    Truncated,
    Denied,
    TimedOut,
    Failed,
}

closed_vocabulary!(QueryOutcome, "query outcome", {
    Completed => "completed",
    Truncated => "truncated",
    Denied => "denied",
    TimedOut => "timed_out",
    Failed => "failed",
});

impl QueryOutcome {
    #[must_use]
    pub const fn has_result(self) -> bool {
        matches!(self, Self::Completed | Self::Truncated)
    }
}

/// How the query was produced. `CohortBuilder` SQL is compiled from typed
/// criteria by Core; values are always bound parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryOrigin {
    SqlEditor,
    CohortBuilder,
}

closed_vocabulary!(QueryOrigin, "query origin", {
    SqlEditor => "sql_editor",
    CohortBuilder => "cohort_builder",
});

/// Reproducibility of a receipt's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputReproducibility {
    /// Every bound snapshot is complete and immutable.
    Exact,
    /// At least one bound snapshot is `partial`; the result says so.
    PartialInputs,
}

closed_vocabulary!(InputReproducibility, "input reproducibility", {
    Exact => "exact",
    PartialInputs => "partial_inputs",
});

/// Result of re-running a receipt against its pinned inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayVerdict {
    Reproduced,
    Diverged,
    /// A pinned input can no longer be read (missing or corrupt).
    InputUnavailable,
    /// The original run produced no result to compare.
    NotReplayable,
}

closed_vocabulary!(ReplayVerdict, "replay verdict", {
    Reproduced => "reproduced",
    Diverged => "diverged",
    InputUnavailable => "input_unavailable",
    NotReplayable => "not_replayable",
});

/// One governed view: a SQL table alias bound to an exact snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewBinding {
    pub alias: String,
    pub snapshot_id: OpaqueId,
}

/// A binding as pinned by a receipt: the exact content it read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedInput {
    pub alias: String,
    pub snapshot_id: OpaqueId,
    pub content_digest: DigestSha256,
    pub schema_fingerprint: DigestSha256,
    pub row_count: u64,
    pub complete: bool,
}

/// True for a safe SQL table alias: lowercase ASCII letter first, then
/// lowercase letters, digits or `_`; not a SQLite keyword-like `sqlite_` name.
#[must_use]
pub fn is_valid_alias(alias: &str) -> bool {
    !alias.is_empty()
        && alias.len() <= ALIAS_MAX_CHARS
        && alias.as_bytes()[0].is_ascii_lowercase()
        && alias
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        && !alias.starts_with("sqlite_")
        && !matches!(alias, "main" | "temp")
}

/// A read-only query request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryRequest {
    pub project_id: OpaqueId,
    pub sql: String,
    pub bindings: Vec<ViewBinding>,
    pub max_rows: Option<u32>,
}

impl QueryRequest {
    /// Shape checks that need no data (lengths, aliases, row cap).
    pub fn validate(&self) -> Result<(), QueryDenyReason> {
        if self.sql.trim().is_empty() || self.sql.chars().count() > SQL_MAX_CHARS {
            return Err(QueryDenyReason::SqlTooLong);
        }
        if self.bindings.is_empty() || self.bindings.len() > MAX_BINDINGS {
            return Err(QueryDenyReason::BadBinding);
        }
        let mut seen = std::collections::HashSet::new();
        for b in &self.bindings {
            if !is_valid_alias(&b.alias) || !seen.insert(b.alias.as_str()) {
                return Err(QueryDenyReason::BadBinding);
            }
        }
        if self.max_rows.is_some_and(|n| n == 0 || n > RESULT_ROWS_MAX) {
            return Err(QueryDenyReason::BadBinding);
        }
        Ok(())
    }

    #[must_use]
    pub fn effective_max_rows(&self) -> u32 {
        self.max_rows.unwrap_or(RESULT_ROWS_DEFAULT)
    }
}

/// Engine identity and the limits it ran under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineIdentity {
    /// `sqlite` in this build (founder decision 2026-09-23; DataFusion
    /// qualification deferred behind the same contract).
    pub engine: String,
    pub version: String,
    pub max_rows: u32,
    pub timeout_ms: u64,
}

/// One result column. The type is the declared SQLite affinity observed
/// for the column (`integer`, `float`, `text`, `null`, `mixed`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultColumn {
    pub name: String,
    pub observed_type: String,
}

/// The canonical form of a result, digested for reproducibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultTableDoc {
    pub columns: Vec<ResultColumn>,
    pub rows: Vec<Vec<CellValue>>,
}

impl ResultTableDoc {
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    #[must_use]
    pub fn digest(&self) -> DigestSha256 {
        DigestSha256::of(&self.canonical_bytes())
    }
}

/// One query execution, allowed or not. Immutable once written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryReceipt {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub origin: QueryOrigin,
    /// The exact SQL that ran (or was refused).
    pub sql: String,
    pub sql_digest: DigestSha256,
    pub inputs: Vec<PinnedInput>,
    pub reproducibility: InputReproducibility,
    pub engine: EngineIdentity,
    pub outcome: QueryOutcome,
    pub deny_reason: Option<QueryDenyReason>,
    /// Fixed text for `failed`/`timed_out` (no data values).
    pub failure: Option<String>,
    /// Set exactly when the outcome has a result.
    pub result_id: Option<OpaqueId>,
    pub result_digest: Option<DigestSha256>,
    pub row_count: u64,
    pub column_count: u32,
    /// The cohort definition that produced the SQL, if any.
    pub cohort_id: Option<OpaqueId>,
}

impl QueryReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if self.sql.chars().count() > SQL_MAX_CHARS {
            return Err("receipt SQL exceeds bound".to_owned());
        }
        if self.sql_digest != DigestSha256::of(self.sql.as_bytes()) {
            return Err("receipt SQL digest mismatch".to_owned());
        }
        if (self.outcome == QueryOutcome::Denied) != self.deny_reason.is_some() {
            return Err("exactly denied receipts carry a deny reason".to_owned());
        }
        if matches!(self.outcome, QueryOutcome::Failed | QueryOutcome::TimedOut)
            != self.failure.is_some()
        {
            return Err("exactly failed or timed-out receipts carry a failure".to_owned());
        }
        let has = self.outcome.has_result();
        if has != self.result_id.is_some() || has != self.result_digest.is_some() {
            return Err("exactly completed receipts name a result".to_owned());
        }
        if !has && (self.row_count != 0 || self.column_count != 0) {
            return Err("a receipt without a result has no rows".to_owned());
        }
        if (self.origin == QueryOrigin::CohortBuilder) != self.cohort_id.is_some() {
            return Err("exactly cohort queries name their cohort".to_owned());
        }
        let expected = if self.inputs.iter().all(|i| i.complete) {
            InputReproducibility::Exact
        } else {
            InputReproducibility::PartialInputs
        };
        if self.reproducibility != expected {
            return Err("reproducibility disagrees with the pinned inputs".to_owned());
        }
        Ok(())
    }
}

/// A stored, immutable derived table (the result of one receipt).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedTable {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub receipt_id: OpaqueId,
    pub content_digest: DigestSha256,
    pub row_count: u64,
    pub column_count: u32,
    /// Inputs this table derives from (provenance; never a copy of truth).
    pub derived_from: Vec<OpaqueId>,
    pub truncated: bool,
}

/// Comparison operators for cohort criteria. Values are always bound as
/// parameters, never spliced into SQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CohortOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    IsNull,
    IsNotNull,
}

closed_vocabulary!(CohortOp, "cohort operator", {
    Eq => "eq",
    Ne => "ne",
    Lt => "lt",
    Le => "le",
    Gt => "gt",
    Ge => "ge",
    IsNull => "is_null",
    IsNotNull => "is_not_null",
});

impl CohortOp {
    #[must_use]
    pub const fn sql(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "<>",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::IsNull => "IS NULL",
            Self::IsNotNull => "IS NOT NULL",
        }
    }

    #[must_use]
    pub const fn takes_value(self) -> bool {
        !matches!(self, Self::IsNull | Self::IsNotNull)
    }
}

/// One cohort criterion over a named field of the bound snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortCriterion {
    pub field: String,
    pub op: CohortOp,
    pub value: Option<CellValue>,
}

/// A saved cohort: all criteria must hold (AND). Immutable once stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortDefinition {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub label: String,
    pub snapshot_id: OpaqueId,
    pub criteria: Vec<CohortCriterion>,
}

impl CohortDefinition {
    pub fn validate(&self) -> Result<(), String> {
        if self.label.trim().is_empty()
            || self.label.chars().count() > LABEL_MAX_CHARS
            || self.label.chars().any(char::is_control)
        {
            return Err("cohort label must be 1-200 printable characters".to_owned());
        }
        if self.criteria.is_empty() || self.criteria.len() > COHORT_CRITERIA_MAX {
            return Err("a cohort has 1-16 criteria".to_owned());
        }
        for c in &self.criteria {
            if c.field.is_empty() || c.field.chars().count() > VALUE_MAX_CHARS {
                return Err("criterion field out of bounds".to_owned());
            }
            match (&c.value, c.op.takes_value()) {
                (Some(CellValue::Null), _) => {
                    return Err("use is_null / is_not_null for missing values".to_owned());
                }
                (Some(v), true) => v.validate()?,
                (None, false) => {}
                _ => return Err("criterion value disagrees with its operator".to_owned()),
            }
        }
        Ok(())
    }
}

/// Descriptive statistics over one column of a derived table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatisticKind {
    Count,
    Missing,
    Mean,
    /// Sample standard deviation (n - 1).
    StdDev,
    Min,
    Median,
    Max,
}

closed_vocabulary!(StatisticKind, "statistic", {
    Count => "count",
    Missing => "missing",
    Mean => "mean",
    StdDev => "std_dev",
    Min => "min",
    Median => "median",
    Max => "max",
});

/// A statistic value, or why it was not computed. Never a silent zero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", deny_unknown_fields)]
pub enum StatisticValue {
    Value {
        value: f64,
    },
    /// Not enough numeric values (for example a standard deviation of one).
    Insufficient {
        needed: u64,
        available: u64,
    },
    /// The column holds non-numeric values; numeric statistics were not run.
    NotNumeric,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticResult {
    pub result_id: OpaqueId,
    pub result_digest: DigestSha256,
    pub column: String,
    pub kind: StatisticKind,
    pub value: StatisticValue,
}

/// A receipt plus, when it completed, its result table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryView {
    pub receipt: QueryReceipt,
    pub table: Option<ResultTableDoc>,
}

/// Replay of a stored receipt against its pinned inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayReport {
    pub receipt_id: OpaqueId,
    pub verdict: ReplayVerdict,
    pub original_digest: Option<DigestSha256>,
    pub replay_digest: Option<DigestSha256>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{AuthorityScopeId, RealmId};

    fn h(id: &str) -> ObjectHeader {
        ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: ANALYTICS_SCHEMA_VERSION,
            realm_id: RealmId::new("realm"),
            authority_scope_id: AuthorityScopeId::new("scope"),
        }
    }

    fn request(sql: &str, aliases: &[&str]) -> QueryRequest {
        QueryRequest {
            project_id: OpaqueId::new("p"),
            sql: sql.to_owned(),
            bindings: aliases
                .iter()
                .map(|a| ViewBinding {
                    alias: (*a).to_owned(),
                    snapshot_id: OpaqueId::new("s"),
                })
                .collect(),
            max_rows: None,
        }
    }

    fn input(complete: bool) -> PinnedInput {
        PinnedInput {
            alias: "t".to_owned(),
            snapshot_id: OpaqueId::new("s"),
            content_digest: DigestSha256::of(b"s"),
            schema_fingerprint: DigestSha256::of(b"f"),
            row_count: 3,
            complete,
        }
    }

    fn receipt(outcome: QueryOutcome) -> QueryReceipt {
        let sql = "SELECT 1 FROM t".to_owned();
        QueryReceipt {
            header: h("r"),
            project_id: OpaqueId::new("p"),
            origin: QueryOrigin::SqlEditor,
            sql_digest: DigestSha256::of(sql.as_bytes()),
            sql,
            inputs: vec![input(true)],
            reproducibility: InputReproducibility::Exact,
            engine: EngineIdentity {
                engine: "sqlite".to_owned(),
                version: "3".to_owned(),
                max_rows: 10,
                timeout_ms: 10,
            },
            outcome,
            deny_reason: None,
            failure: None,
            result_id: None,
            result_digest: None,
            row_count: 0,
            column_count: 0,
            cohort_id: None,
        }
    }

    #[test]
    fn vocabularies_round_trip_and_are_closed() {
        for v in QueryDenyReason::ALL {
            assert_eq!(QueryDenyReason::parse(v.as_str()), Ok(*v));
        }
        for v in QueryOutcome::ALL {
            assert_eq!(QueryOutcome::parse(v.as_str()), Ok(*v));
        }
        for v in StatisticKind::ALL {
            assert_eq!(StatisticKind::parse(v.as_str()), Ok(*v));
        }
        for v in CohortOp::ALL {
            assert_eq!(CohortOp::parse(v.as_str()), Ok(*v));
        }
        assert!(QueryOutcome::parse("partial").is_err());
        assert!(serde_json::from_str::<CohortOp>("\"like\"").is_err());
    }

    #[test]
    fn aliases_are_plain_identifiers() {
        for ok in ["t", "labs", "cohort_2024"] {
            assert!(is_valid_alias(ok), "{ok}");
        }
        for bad in [
            "",
            "T",
            "1t",
            "t-1",
            "t t",
            "t;drop",
            "\"t\"",
            "main",
            "temp",
            "sqlite_master",
        ] {
            assert!(!is_valid_alias(bad), "{bad}");
        }
        assert!(!is_valid_alias(&"a".repeat(ALIAS_MAX_CHARS + 1)));
    }

    #[test]
    fn requests_validate_shape() {
        assert!(request("SELECT * FROM t", &["t"]).validate().is_ok());
        assert_eq!(
            request(" ", &["t"]).validate(),
            Err(QueryDenyReason::SqlTooLong)
        );
        assert_eq!(
            request(&"x".repeat(SQL_MAX_CHARS + 1), &["t"]).validate(),
            Err(QueryDenyReason::SqlTooLong)
        );
        assert_eq!(
            request("SELECT 1", &[]).validate(),
            Err(QueryDenyReason::BadBinding)
        );
        assert_eq!(
            request("SELECT 1", &["t", "t"]).validate(),
            Err(QueryDenyReason::BadBinding)
        );
        let mut big = request("SELECT 1", &["t"]);
        big.max_rows = Some(RESULT_ROWS_MAX + 1);
        assert_eq!(big.validate(), Err(QueryDenyReason::BadBinding));
        big.max_rows = Some(5);
        assert_eq!(big.effective_max_rows(), 5);
    }

    #[test]
    fn receipts_hold_their_invariants() {
        let mut ok = receipt(QueryOutcome::Completed);
        ok.result_id = Some(OpaqueId::new("res"));
        ok.result_digest = Some(DigestSha256::of(b"x"));
        ok.row_count = 2;
        ok.column_count = 1;
        assert!(ok.validate().is_ok());

        let mut tampered = ok.clone();
        tampered.sql = "SELECT 2 FROM t".to_owned();
        assert!(tampered.validate().is_err(), "SQL digest bound");

        let mut denied = receipt(QueryOutcome::Denied);
        assert!(denied.validate().is_err(), "denied without reason");
        denied.deny_reason = Some(QueryDenyReason::NotReadOnly);
        assert!(denied.validate().is_ok());
        denied.row_count = 1;
        assert!(denied.validate().is_err());

        let mut timed = receipt(QueryOutcome::TimedOut);
        assert!(timed.validate().is_err());
        timed.failure = Some("time limit".to_owned());
        assert!(timed.validate().is_ok());

        let mut partial = ok.clone();
        partial.inputs = vec![input(false)];
        assert!(
            partial.validate().is_err(),
            "partial inputs must be declared"
        );
        partial.reproducibility = InputReproducibility::PartialInputs;
        assert!(partial.validate().is_ok());

        let mut cohort = ok;
        cohort.origin = QueryOrigin::CohortBuilder;
        assert!(cohort.validate().is_err());
        cohort.cohort_id = Some(OpaqueId::new("c"));
        assert!(cohort.validate().is_ok());
    }

    #[test]
    fn cohorts_bind_values_and_validate() {
        let mut c = CohortDefinition {
            header: h("c"),
            project_id: OpaqueId::new("p"),
            label: "adults".to_owned(),
            snapshot_id: OpaqueId::new("s"),
            criteria: vec![CohortCriterion {
                field: "age".to_owned(),
                op: CohortOp::Ge,
                value: Some(CellValue::Integer(18)),
            }],
        };
        assert!(c.validate().is_ok());
        c.criteria[0].value = None;
        assert!(c.validate().is_err(), "comparison needs a value");
        c.criteria[0].op = CohortOp::IsNull;
        assert!(c.validate().is_ok());
        c.criteria[0].value = Some(CellValue::Integer(1));
        assert!(c.validate().is_err(), "is_null takes no value");
        c.criteria[0] = CohortCriterion {
            field: "age".to_owned(),
            op: CohortOp::Eq,
            value: Some(CellValue::Null),
        };
        assert!(c.validate().is_err(), "null comparison is refused");
        c.criteria.clear();
        assert!(c.validate().is_err());
        assert_eq!(CohortOp::Ne.sql(), "<>");
    }

    #[test]
    fn statistic_values_never_hide_missing_data() {
        let json = serde_json::to_string(&StatisticValue::Insufficient {
            needed: 2,
            available: 1,
        })
        .unwrap();
        assert_eq!(json, r#"{"state":"insufficient","needed":2,"available":1}"#);
        assert!(serde_json::from_str::<StatisticValue>(r#"{"state":"zero"}"#).is_err());
    }
}
