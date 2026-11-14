# Contracts — Spec 082 Analytics Gate

`crates/medscale-contracts/src/analytics.rs`, schema
`ANALYTICS_SCHEMA_VERSION = 1`.

```text
QueryRequest     { project_id, sql, bindings[ViewBinding], max_rows? }
ViewBinding      { alias, snapshot_id }
PinnedInput      { alias, snapshot_id, content_digest, schema_fingerprint,
                   row_count, complete }
QueryReceipt     { header, project_id, origin, sql, sql_digest, inputs[],
                   reproducibility, engine, outcome, deny_reason?, failure?,
                   result_id?, result_digest?, row_count, column_count,
                   cohort_id? }
EngineIdentity   { engine, version, max_rows, timeout_ms }
ResultTableDoc   { columns[{name, observed_type}], rows[[CellValue]] }
DerivedTable     { header, project_id, receipt_id, content_digest,
                   row_count, column_count, derived_from[], truncated }
CohortDefinition { header, project_id, label, snapshot_id, criteria[] }
CohortCriterion  { field, op, value? }
StatisticResult  { result_id, result_digest, column, kind, value }
ReplayReport     { receipt_id, verdict, original_digest?, replay_digest? }
```

Closed vocabularies:
- `QueryOutcome`: completed, truncated, denied, timed_out, failed.
- `QueryDenyReason`: not_read_only, multiple_statements,
  forbidden_construct, sql_too_long, bad_binding, unknown_table,
  snapshot_unavailable, syntax_error.
- `QueryOrigin`: sql_editor, cohort_builder.
- `InputReproducibility`: exact, partial_inputs.
- `ReplayVerdict`: reproduced, diverged, input_unavailable, not_replayable.
- `CohortOp`: eq, ne, lt, le, gt, ge, is_null, is_not_null.
- `StatisticKind`: count, missing, mean, std_dev, min, median, max.

`StatisticValue` is one of `value`, `insufficient {needed, available}` or
`not_numeric`.

Receipt invariants (`validate`):
- the SQL digest matches the SQL;
- exactly denied receipts carry a reason, and exactly failed or timed-out
  receipts carry a fixed failure text;
- exactly completed or truncated receipts name a result and digest;
- receipts without a result have no rows;
- exactly cohort receipts name their cohort;
- reproducibility matches the pinned inputs.

Aliases are lowercase identifiers that are not `main`, `temp` or
`sqlite_*`.
