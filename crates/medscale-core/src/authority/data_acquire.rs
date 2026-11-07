//! Data acquisition for Spec 075: bounded hostile-input parsing, canonical
//! materialization, external SQLite reads, and brokered remote dataset fetch.
//!
//! All parsers are dependency-free, streaming-bounded, and deterministic: the
//! same bytes always yield the same fields/rows/digests. Malformed input
//! fails as explicit `AcquireFail`, never as half-materialized state.

use medscale_contracts::data_sources::{
    ACQUIRE_WARNINGS_MAX, CELL_TEXT_MAX_BYTES, CellValue, FieldType, FilterExpr, FilterOp,
    SCHEMA_FIELDS_MAX, SNAPSHOT_BYTES_MAX, SNAPSHOT_PART_ROWS, SNAPSHOT_ROWS_MAX, SchemaField,
    SortKey, TransformOp,
};
use medscale_contracts::network::{BrokerReasonCode, EgressDataClass, EgressPurpose};
use medscale_storage::{ExternalCell, ExternalTable};

/// Parsed table ready for schema freeze and materialization.
#[derive(Debug, Clone)]
pub struct ParsedTable {
    pub fields: Vec<SchemaField>,
    pub rows: Vec<Vec<CellValue>>,
    pub warnings: Vec<String>,
    pub rows_skipped: u64,
    pub bytes_hashed: u64,
}

/// Explicit acquisition failure. The caller maps these to typed authority
/// errors or receipt outcomes; content problems never become silent success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcquireFail {
    Rejected(String),
    Quarantined(String),
    Unavailable(String),
    Denied(String),
    Unsupported(String),
    Missing(String),
}

impl AcquireFail {
    /// Returns the human-readable reason (never includes secrets or bytes).
    #[must_use]
    pub fn reason(&self) -> &str {
        match self {
            Self::Rejected(reason)
            | Self::Quarantined(reason)
            | Self::Unavailable(reason)
            | Self::Denied(reason)
            | Self::Unsupported(reason)
            | Self::Missing(reason) => reason,
        }
    }
}

/// Most cells one import may hold. Rows and fields are bounded separately,
/// but their product is what gets allocated, so it is bounded too.
pub const TABLE_CELLS_MAX: usize = 16_000_000;

fn cells_exceeded() -> AcquireFail {
    AcquireFail::Rejected("input exceeds the table cell bound".to_owned())
}

fn push_warning(warnings: &mut Vec<String>, warning: String) {
    if warnings.len() < ACQUIRE_WARNINGS_MAX {
        warnings.push(warning);
    }
}

// ---------- delimited (CSV/TSV) ----------

/// Parses delimited bytes with RFC-4180-style quoting (`""` escapes).
/// Ragged rows are skipped and counted; zero data rows rejects the import.
pub fn parse_delimited(bytes: &[u8], delimiter: u8) -> Result<ParsedTable, AcquireFail> {
    if bytes.len() as u64 > SNAPSHOT_BYTES_MAX {
        return Err(AcquireFail::Rejected(
            "input exceeds snapshot byte bound".to_owned(),
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| AcquireFail::Quarantined("input is not valid UTF-8".to_owned()))?;
    let records = split_records(text, delimiter)?;
    if records.is_empty() {
        return Err(AcquireFail::Rejected("input has no rows".to_owned()));
    }
    let header = &records[0];
    if header.is_empty() {
        return Err(AcquireFail::Rejected("input is empty".to_owned()));
    }
    if header.len() > SCHEMA_FIELDS_MAX {
        return Err(AcquireFail::Rejected(
            "header exceeds field bound".to_owned(),
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for name in header {
        if name.trim().is_empty() || name.contains('\0') {
            return Err(AcquireFail::Rejected(
                "header names must be non-empty".to_owned(),
            ));
        }
        if !seen.insert(name.clone()) {
            return Err(AcquireFail::Rejected(format!(
                "duplicate header name {name}"
            )));
        }
    }
    let width = header.len();
    let mut columns: Vec<Vec<String>> = vec![Vec::new(); width];
    let mut rows_skipped = 0_u64;
    let mut warnings = Vec::new();
    for record in records.iter().skip(1) {
        if record.len() != width {
            rows_skipped += 1;
            continue;
        }
        for (index, cell) in record.iter().enumerate() {
            if cell.len() > CELL_TEXT_MAX_BYTES {
                return Err(AcquireFail::Quarantined(
                    "cell exceeds text bound".to_owned(),
                ));
            }
            columns[index].push(cell.clone());
        }
        if columns[0].len() as u64 > SNAPSHOT_ROWS_MAX {
            return Err(AcquireFail::Rejected(
                "input exceeds snapshot row bound".to_owned(),
            ));
        }
    }
    if columns[0].is_empty() {
        return Err(AcquireFail::Rejected("input has no data rows".to_owned()));
    }
    if rows_skipped > 0 {
        push_warning(&mut warnings, format!("skipped {rows_skipped} ragged rows"));
    }
    let mut fields = Vec::with_capacity(width);
    for (index, name) in header.iter().enumerate() {
        fields.push(SchemaField {
            name: name.clone(),
            field_type: infer_column_type(&columns[index]),
            nullable: true,
            declared_unit: None,
        });
    }
    let rows = columns_to_rows(&columns, &fields);
    Ok(ParsedTable {
        fields,
        rows,
        warnings,
        rows_skipped,
        bytes_hashed: bytes.len() as u64,
    })
}

/// Splits delimited text into records, honoring double-quote grouping.
/// Fails once the fields read exceed `TABLE_CELLS_MAX`, before the header
/// and width checks, so a file of bare delimiters can't exhaust memory.
fn split_records(text: &str, delimiter: u8) -> Result<Vec<Vec<String>>, AcquireFail> {
    let delim = delimiter as char;
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut fields_read = 0_usize;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if fields_read > TABLE_CELLS_MAX {
            return Err(cells_exceeded());
        }
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(ch);
            }
        } else if ch == '"' && field.is_empty() {
            in_quotes = true;
        } else if ch == delim {
            record.push(std::mem::take(&mut field));
            fields_read += 1;
        } else if ch == '\n' {
            record.push(std::mem::take(&mut field));
            fields_read += 1;
            push_record(&mut records, &mut record);
        } else if ch == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            record.push(std::mem::take(&mut field));
            fields_read += 1;
            push_record(&mut records, &mut record);
        } else {
            field.push(ch);
        }
    }
    if in_quotes || !field.is_empty() || !record.is_empty() {
        record.push(field);
        push_record(&mut records, &mut record);
    }
    Ok(records)
}

fn push_record(records: &mut Vec<Vec<String>>, record: &mut Vec<String>) {
    // Skip fully empty lines (a single empty field with no content).
    if record.len() == 1 && record[0].is_empty() {
        record.clear();
        return;
    }
    records.push(std::mem::take(record));
}

/// Infers the narrowest column type covering all non-empty cells.
fn infer_column_type(cells: &[String]) -> FieldType {
    let mut non_empty = 0;
    let mut all_integer = true;
    let mut all_float = true;
    let mut all_boolean = true;
    for cell in cells {
        if cell.is_empty() {
            continue;
        }
        non_empty += 1;
        if cell.parse::<i64>().is_err() {
            all_integer = false;
        }
        if cell.parse::<f64>().is_err() {
            all_float = false;
        }
        let lower = cell.to_ascii_lowercase();
        if lower != "true" && lower != "false" && lower != "1" && lower != "0" {
            all_boolean = false;
        }
    }
    if non_empty == 0 {
        return FieldType::Text;
    }
    if all_integer {
        FieldType::Integer
    } else if all_float {
        FieldType::Float
    } else if all_boolean {
        FieldType::Boolean
    } else {
        FieldType::Text
    }
}

/// Converts column storage to typed row storage.
fn columns_to_rows(columns: &[Vec<String>], fields: &[SchemaField]) -> Vec<Vec<CellValue>> {
    let height = columns.first().map_or(0, Vec::len);
    let mut rows: Vec<Vec<CellValue>> = Vec::with_capacity(height);
    for _ in 0..height {
        rows.push(Vec::with_capacity(columns.len()));
    }
    for (col, field) in columns.iter().zip(fields.iter()) {
        for (row_index, cell) in col.iter().enumerate() {
            rows[row_index].push(coerce_text(cell, field.field_type));
        }
    }
    rows
}

/// Coerces one text cell to its inferred type; empty is always Null.
fn coerce_text(raw: &str, field_type: FieldType) -> CellValue {
    if raw.is_empty() {
        return CellValue::Null;
    }
    match field_type {
        FieldType::Integer => raw
            .parse::<i64>()
            .map_or(CellValue::Text(raw.to_owned()), CellValue::Integer),
        FieldType::Float => raw
            .parse::<f64>()
            .map_or(CellValue::Text(raw.to_owned()), CellValue::Float),
        FieldType::Boolean => {
            let lower = raw.to_ascii_lowercase();
            match lower.as_str() {
                "true" | "1" => CellValue::Boolean(true),
                "false" | "0" => CellValue::Boolean(false),
                _ => CellValue::Text(raw.to_owned()),
            }
        }
        FieldType::Text
        | FieldType::Date
        | FieldType::Time
        | FieldType::DateTime
        | FieldType::Binary => CellValue::Text(raw.to_owned()),
    }
}

// ---------- JSON / JSONL ----------

/// Parses a JSON array of objects or JSONL lines into a table. Nested values
/// become canonical JSON text with a counted warning; mixed-type columns
/// widen deterministically (Integer+Float to Float, anything+Text to Text).
pub fn parse_json_bytes(bytes: &[u8], lines: bool) -> Result<ParsedTable, AcquireFail> {
    if bytes.len() as u64 > SNAPSHOT_BYTES_MAX {
        return Err(AcquireFail::Rejected(
            "input exceeds snapshot byte bound".to_owned(),
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| AcquireFail::Quarantined("input is not valid UTF-8".to_owned()))?;
    let mut objects: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();
    if lines {
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(line).map_err(|_| {
                AcquireFail::Rejected(format!("invalid JSON on line {}", index + 1))
            })?;
            let Some(map) = value.as_object().cloned() else {
                return Err(AcquireFail::Rejected(format!(
                    "JSONL line {} is not an object",
                    index + 1
                )));
            };
            objects.push(map);
        }
    } else {
        let value: serde_json::Value = serde_json::from_str(text)
            .map_err(|e| AcquireFail::Rejected(format!("invalid JSON document: {e}")))?;
        let Some(array) = value.as_array() else {
            return Err(AcquireFail::Rejected(
                "JSON document must be an array of objects".to_owned(),
            ));
        };
        for value in array {
            let Some(map) = value.as_object() else {
                return Err(AcquireFail::Rejected(
                    "JSON array elements must be objects".to_owned(),
                ));
            };
            objects.push(map.clone());
        }
    }
    if objects.is_empty() {
        return Err(AcquireFail::Rejected("input has no data rows".to_owned()));
    }
    if objects.len() as u64 > SNAPSHOT_ROWS_MAX {
        return Err(AcquireFail::Rejected(
            "input exceeds snapshot row bound".to_owned(),
        ));
    }
    let mut names: Vec<String> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for map in &objects {
        for key in map.keys() {
            if seen.insert(key.clone()) {
                names.push(key.clone());
            }
        }
    }
    if names.is_empty() || names.len() > SCHEMA_FIELDS_MAX {
        return Err(AcquireFail::Rejected(
            "JSON fields are empty or exceed bound".to_owned(),
        ));
    }
    for name in &names {
        if name.trim().is_empty() || name.contains('\0') {
            return Err(AcquireFail::Rejected(
                "JSON field names must be non-empty".to_owned(),
            ));
        }
    }
    if objects.len().saturating_mul(names.len()) > TABLE_CELLS_MAX {
        return Err(cells_exceeded());
    }
    let mut warnings = Vec::new();
    let mut nested = 0_u64;
    let mut columns: Vec<Vec<CellValue>> = vec![Vec::new(); names.len()];
    for map in &objects {
        for (index, name) in names.iter().enumerate() {
            let cell = match map.get(name) {
                None | Some(serde_json::Value::Null) => CellValue::Null,
                Some(serde_json::Value::Bool(b)) => CellValue::Boolean(*b),
                Some(serde_json::Value::Number(n)) => {
                    if let Some(v) = n.as_i64() {
                        CellValue::Integer(v)
                    } else if let Some(v) = n.as_f64() {
                        CellValue::Float(v)
                    } else {
                        CellValue::Text(n.to_string())
                    }
                }
                Some(serde_json::Value::String(s)) => {
                    if s.len() > CELL_TEXT_MAX_BYTES {
                        return Err(AcquireFail::Quarantined(
                            "cell exceeds text bound".to_owned(),
                        ));
                    }
                    if s.contains('\0') {
                        return Err(AcquireFail::Quarantined("cell contains NUL".to_owned()));
                    }
                    CellValue::Text(s.clone())
                }
                Some(other) => {
                    nested += 1;
                    CellValue::Text(
                        serde_json::to_string(other).unwrap_or_else(|_| "null".to_owned()),
                    )
                }
            };
            columns[index].push(cell);
        }
    }
    if nested > 0 {
        push_warning(
            &mut warnings,
            format!("encoded {nested} nested values as JSON text"),
        );
    }
    let mut fields = Vec::with_capacity(names.len());
    for (index, name) in names.iter().enumerate() {
        fields.push(SchemaField {
            name: name.clone(),
            field_type: merge_cell_type(&columns[index]),
            nullable: true,
            declared_unit: None,
        });
    }
    // Widen cells that do not fit the merged column type to Text.
    let mut rows: Vec<Vec<CellValue>> = vec![Vec::with_capacity(names.len()); objects.len()];
    for (col_index, field) in fields.iter().enumerate() {
        for (row_index, cell) in columns[col_index].iter().enumerate() {
            rows[row_index].push(widen_cell(cell, field.field_type));
        }
    }
    Ok(ParsedTable {
        fields,
        rows,
        warnings,
        rows_skipped: 0,
        bytes_hashed: bytes.len() as u64,
    })
}

/// Merges observed cell types into one deterministic column type.
fn merge_cell_type(cells: &[CellValue]) -> FieldType {
    let mut has_text = false;
    let mut has_float = false;
    let mut has_integer = false;
    let mut has_boolean = false;
    let mut any = false;
    for cell in cells {
        match cell {
            CellValue::Null => {}
            CellValue::Text(_) => {
                has_text = true;
                any = true;
            }
            CellValue::Float(_) => {
                has_float = true;
                any = true;
            }
            CellValue::Integer(_) => {
                has_integer = true;
                any = true;
            }
            CellValue::Boolean(_) => {
                has_boolean = true;
                any = true;
            }
        }
    }
    if !any {
        return FieldType::Text;
    }
    if has_text {
        return FieldType::Text;
    }
    if has_float && (has_integer || has_boolean) {
        return FieldType::Float;
    }
    if has_float {
        return FieldType::Float;
    }
    if has_integer && !has_boolean {
        return FieldType::Integer;
    }
    if has_boolean && !has_integer {
        return FieldType::Boolean;
    }
    FieldType::Text
}

/// Widens one cell to its column type (numeric mix to Float, rest to Text).
fn widen_cell(cell: &CellValue, field_type: FieldType) -> CellValue {
    match (cell, field_type) {
        (CellValue::Integer(v), FieldType::Float) => CellValue::Float(*v as f64),
        (CellValue::Boolean(b), FieldType::Float) => CellValue::Float(if *b { 1.0 } else { 0.0 }),
        (CellValue::Integer(v), FieldType::Text) => CellValue::Text(v.to_string()),
        (CellValue::Float(v), FieldType::Text) => CellValue::Text(canonical_float(*v)),
        (CellValue::Boolean(b), FieldType::Text) => CellValue::Text(b.to_string()),
        (CellValue::Boolean(b), FieldType::Integer) => CellValue::Integer(i64::from(*b)),
        _ => cell.clone(),
    }
}

/// Canonical float rendering for Text widening.
fn canonical_float(value: f64) -> String {
    serde_json::Number::from_f64(value).map_or("null".to_owned(), |n| n.to_string())
}

// ---------- external SQLite mapping ----------

/// Maps an external SQLite table into a parsed table with merged types.
pub fn map_external_table(table: &ExternalTable) -> Result<ParsedTable, AcquireFail> {
    if table.columns.is_empty() || table.rows.is_empty() {
        return Err(AcquireFail::Rejected(
            "external table has no columns or rows".to_owned(),
        ));
    }
    if table.rows.len() as u64 > SNAPSHOT_ROWS_MAX {
        return Err(AcquireFail::Rejected(
            "external table exceeds snapshot row bound".to_owned(),
        ));
    }
    let width = table.columns.len();
    let mut columns: Vec<Vec<CellValue>> = vec![Vec::new(); width];
    for row in &table.rows {
        if row.len() != width {
            return Err(AcquireFail::Quarantined(
                "external row width mismatch".to_owned(),
            ));
        }
        for (index, cell) in row.iter().enumerate() {
            columns[index].push(match cell {
                ExternalCell::Null => CellValue::Null,
                ExternalCell::Integer(v) => CellValue::Integer(*v),
                ExternalCell::Real(v) => CellValue::Float(*v),
                ExternalCell::Text(s) => CellValue::Text(s.clone()),
            });
        }
    }
    let mut fields = Vec::with_capacity(width);
    for (index, name) in table.columns.iter().enumerate() {
        if name.trim().is_empty() || name.contains('\0') {
            return Err(AcquireFail::Rejected(
                "external column names must be non-empty".to_owned(),
            ));
        }
        fields.push(SchemaField {
            name: name.clone(),
            field_type: merge_cell_type(&columns[index]),
            nullable: !table.not_null.get(index).copied().unwrap_or(false),
            declared_unit: None,
        });
    }
    let height = table.rows.len();
    let mut rows: Vec<Vec<CellValue>> = vec![Vec::with_capacity(width); height];
    for (col_index, field) in fields.iter().enumerate() {
        for (row_index, cell) in columns[col_index].iter().enumerate() {
            rows[row_index].push(widen_cell(cell, field.field_type));
        }
    }
    Ok(ParsedTable {
        fields,
        rows,
        warnings: Vec::new(),
        rows_skipped: 0,
        bytes_hashed: 0,
    })
}

// ---------- materialization ----------

/// Materializes canonical snapshot bytes, content digest, and part digests.
/// Part i covers rows `[i*PART .. min((i+1)*PART, n))`; digests are over the
/// canonical per-part encoding so paging verification needs no extra blobs.
#[must_use]
pub fn materialize_parts(
    fields: &[SchemaField],
    rows: &[Vec<CellValue>],
) -> (
    Vec<u8>,
    medscale_contracts::objects::DigestSha256,
    Vec<(u64, u64, medscale_contracts::objects::DigestSha256)>,
) {
    use medscale_contracts::data_sources::{canonical_part_bytes, canonical_snapshot_bytes};
    let bytes = canonical_snapshot_bytes(fields, rows);
    let digest = medscale_contracts::objects::DigestSha256::of(&bytes);
    let mut parts = Vec::new();
    let mut start = 0_u64;
    let total = rows.len() as u64;
    while start < total {
        let end = (start + SNAPSHOT_PART_ROWS).min(total);
        let slice = &rows[start as usize..end as usize];
        let part_digest =
            medscale_contracts::objects::DigestSha256::of(&canonical_part_bytes(slice));
        parts.push((start, end, part_digest));
        start = end;
    }
    (bytes, digest, parts)
}

/// Verifies canonical bytes against a snapshot digest and decodes the table.
pub fn decode_snapshot_bytes(
    bytes: &[u8],
    expected: &medscale_contracts::objects::DigestSha256,
) -> Result<medscale_contracts::data_sources::SnapshotCanonicalDoc, AcquireFail> {
    if medscale_contracts::objects::DigestSha256::of(bytes) != *expected {
        return Err(AcquireFail::Quarantined(
            "snapshot bytes fail digest verification".to_owned(),
        ));
    }
    serde_json::from_slice(bytes).map_err(|_| {
        AcquireFail::Quarantined("snapshot bytes are not a valid snapshot document".to_owned())
    })
}

// ---------- row query (filter/sort/page) ----------

/// Applies filters then a stable sort, returning surviving row indices.
#[must_use]
pub fn apply_view(
    rows: &[Vec<CellValue>],
    fields: &[SchemaField],
    filters: &[FilterExpr],
    sort: &[SortKey],
) -> Vec<usize> {
    let mut kept: Vec<usize> = (0..rows.len()).collect();
    if !filters.is_empty() {
        kept.retain(|index| row_matches(&rows[*index], fields, filters));
    }
    if !sort.is_empty() {
        kept.sort_by(|a, b| compare_rows(&rows[*a], &rows[*b], fields, sort));
    }
    kept
}

fn column_index(fields: &[SchemaField], column: &str) -> Option<usize> {
    fields.iter().position(|f| f.name == column)
}

fn row_matches(row: &[CellValue], fields: &[SchemaField], filters: &[FilterExpr]) -> bool {
    filters.iter().all(|filter| {
        let Some(index) = column_index(fields, &filter.column) else {
            return false;
        };
        let cell = &row[index];
        if cell.is_null() {
            return false;
        }
        match fields[index].field_type {
            FieldType::Integer => {
                let Ok(want) = filter.value.parse::<i64>() else {
                    return false;
                };
                let CellValue::Integer(got) = cell else {
                    return false;
                };
                compare_int(*got, want, filter.op)
            }
            FieldType::Float => {
                let Ok(want) = filter.value.parse::<f64>() else {
                    return false;
                };
                let got = match cell {
                    CellValue::Float(v) => *v,
                    CellValue::Integer(v) => *v as f64,
                    _ => return false,
                };
                compare_float(got, want, filter.op)
            }
            FieldType::Boolean => {
                let want = match filter.value.to_ascii_lowercase().as_str() {
                    "true" | "1" => true,
                    "false" | "0" => false,
                    _ => return false,
                };
                let CellValue::Boolean(got) = cell else {
                    return false;
                };
                match filter.op {
                    FilterOp::Equals => *got == want,
                    FilterOp::NotEquals => *got != want,
                    _ => false,
                }
            }
            FieldType::Text
            | FieldType::Date
            | FieldType::Time
            | FieldType::DateTime
            | FieldType::Binary => {
                let CellValue::Text(got) = cell else {
                    return false;
                };
                match filter.op {
                    FilterOp::Equals => *got == filter.value,
                    FilterOp::NotEquals => *got != filter.value,
                    FilterOp::Contains => got.contains(filter.value.as_str()),
                    FilterOp::GreaterThan => *got > filter.value,
                    FilterOp::LessThan => *got < filter.value,
                }
            }
        }
    })
}

fn compare_int(got: i64, want: i64, op: FilterOp) -> bool {
    match op {
        FilterOp::Equals => got == want,
        FilterOp::NotEquals => got != want,
        FilterOp::Contains => got.to_string().contains(&want.to_string()),
        FilterOp::GreaterThan => got > want,
        FilterOp::LessThan => got < want,
    }
}

fn compare_float(got: f64, want: f64, op: FilterOp) -> bool {
    // Total bitwise equality: deterministic across processes, no NaN
    // tolerance implied (datasets carry measured values, not estimates).
    match op {
        FilterOp::Equals => got.to_bits() == want.to_bits(),
        FilterOp::NotEquals => got.to_bits() != want.to_bits(),
        FilterOp::Contains => false,
        FilterOp::GreaterThan => got > want,
        FilterOp::LessThan => got < want,
    }
}

fn compare_rows(
    a: &[CellValue],
    b: &[CellValue],
    fields: &[SchemaField],
    sort: &[SortKey],
) -> std::cmp::Ordering {
    for key in sort {
        let Some(index) = column_index(fields, &key.column) else {
            continue;
        };
        let ordering = compare_cells(&a[index], &b[index]);
        if ordering != std::cmp::Ordering::Equal {
            return if key.descending {
                ordering.reverse()
            } else {
                ordering
            };
        }
    }
    std::cmp::Ordering::Equal
}

/// Total cell order with Nulls always last, independent of direction.
fn compare_cells(a: &CellValue, b: &CellValue) -> std::cmp::Ordering {
    match (a, b) {
        (CellValue::Null, CellValue::Null) => std::cmp::Ordering::Equal,
        (CellValue::Null, _) => std::cmp::Ordering::Greater,
        (_, CellValue::Null) => std::cmp::Ordering::Less,
        (CellValue::Integer(x), CellValue::Integer(y)) => x.cmp(y),
        (CellValue::Float(x), CellValue::Float(y)) => {
            x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
        }
        (CellValue::Integer(x), CellValue::Float(y)) => (*x as f64)
            .partial_cmp(y)
            .unwrap_or(std::cmp::Ordering::Equal),
        (CellValue::Float(x), CellValue::Integer(y)) => x
            .partial_cmp(&(*y as f64))
            .unwrap_or(std::cmp::Ordering::Equal),
        (CellValue::Text(x), CellValue::Text(y)) => x.cmp(y),
        (CellValue::Boolean(x), CellValue::Boolean(y)) => x.cmp(y),
        // Mixed-type cells order by a fixed type rank (deterministic).
        _ => type_rank(a).cmp(&type_rank(b)),
    }
}

fn type_rank(cell: &CellValue) -> u8 {
    match cell {
        CellValue::Null => 4,
        CellValue::Text(_) => 3,
        CellValue::Float(_) => 2,
        CellValue::Integer(_) => 1,
        CellValue::Boolean(_) => 0,
    }
}

/// Deterministic transform output: new fields, new rows, counted cast failures.
pub type TransformOutput = (Vec<SchemaField>, Vec<Vec<CellValue>>, u64);

// ---------- deterministic transforms ----------

/// Executes frozen transform ops. Returns new fields, rows, and the counted
/// cast failures (failed non-strict casts become Null and are counted;
/// strict casts abort the whole transform on first failure).
pub fn execute_transform(
    fields: &[SchemaField],
    rows: &[Vec<CellValue>],
    ops: &[TransformOp],
) -> Result<TransformOutput, String> {
    let mut cur_fields = fields.to_vec();
    let mut cur_rows = rows.to_vec();
    let mut cast_failures = 0_u64;
    for op in ops {
        op.validate(&cur_fields)?;
        match op {
            TransformOp::SelectColumns { columns } => {
                let keep: Vec<usize> = columns
                    .iter()
                    .filter_map(|c| column_index(&cur_fields, c))
                    .collect();
                cur_fields = keep.iter().map(|i| cur_fields[*i].clone()).collect();
                cur_rows = cur_rows
                    .iter()
                    .map(|row| keep.iter().map(|i| row[*i].clone()).collect())
                    .collect();
            }
            TransformOp::DropColumns { columns } => {
                let drop: Vec<usize> = columns
                    .iter()
                    .filter_map(|c| column_index(&cur_fields, c))
                    .collect();
                let keep: Vec<usize> = (0..cur_fields.len())
                    .filter(|i| !drop.contains(i))
                    .collect();
                if keep.is_empty() {
                    return Err("transform would drop every column".to_owned());
                }
                cur_fields = keep.iter().map(|i| cur_fields[*i].clone()).collect();
                cur_rows = cur_rows
                    .iter()
                    .map(|row| keep.iter().map(|i| row[*i].clone()).collect())
                    .collect();
            }
            TransformOp::RenameColumn { from, to } => {
                if let Some(index) = column_index(&cur_fields, from) {
                    cur_fields[index].name = to.clone();
                }
            }
            TransformOp::CastType { column, to, strict } => {
                let Some(index) = column_index(&cur_fields, column) else {
                    return Err(format!("cast references unknown column {column}"));
                };
                cur_fields[index].field_type = *to;
                for row in &mut cur_rows {
                    match cast_cell(&row[index], *to) {
                        Ok(cell) => row[index] = cell,
                        Err(()) => {
                            if *strict {
                                return Err(format!("strict cast failed for column {column}"));
                            }
                            cast_failures += 1;
                            row[index] = CellValue::Null;
                        }
                    }
                }
            }
            TransformOp::FilterRows { filters } => {
                let kept: Vec<Vec<CellValue>> = cur_rows
                    .into_iter()
                    .filter(|row| row_matches(row, &cur_fields, filters))
                    .collect();
                cur_rows = kept;
            }
            TransformOp::SortRows { keys } => {
                let mut order: Vec<usize> = (0..cur_rows.len()).collect();
                order.sort_by(|a, b| compare_rows(&cur_rows[*a], &cur_rows[*b], &cur_fields, keys));
                cur_rows = order.into_iter().map(|i| cur_rows[i].clone()).collect();
            }
        }
    }
    Ok((cur_fields, cur_rows, cast_failures))
}

/// Casts one cell to a target type. `Date`/`Time`/`DateTime` validate shape
/// only (ISO text is preserved); `Binary` casts are refused.
fn cast_cell(cell: &CellValue, to: FieldType) -> Result<CellValue, ()> {
    if cell.is_null() {
        return Ok(CellValue::Null);
    }
    match to {
        FieldType::Text => Ok(match cell {
            CellValue::Text(s) => CellValue::Text(s.clone()),
            CellValue::Integer(v) => CellValue::Text(v.to_string()),
            CellValue::Float(v) => CellValue::Text(canonical_float(*v)),
            CellValue::Boolean(b) => CellValue::Text(b.to_string()),
            CellValue::Null => CellValue::Null,
        }),
        FieldType::Integer => match cell {
            CellValue::Integer(v) => Ok(CellValue::Integer(*v)),
            CellValue::Float(v) => {
                if v.fract() == 0.0 && *v >= i64::MIN as f64 && *v <= i64::MAX as f64 {
                    Ok(CellValue::Integer(*v as i64))
                } else {
                    Err(())
                }
            }
            CellValue::Boolean(b) => Ok(CellValue::Integer(i64::from(*b))),
            CellValue::Text(s) => s.parse::<i64>().map(CellValue::Integer).map_err(|_| ()),
            CellValue::Null => Ok(CellValue::Null),
        },
        FieldType::Float => match cell {
            CellValue::Float(v) => Ok(CellValue::Float(*v)),
            CellValue::Integer(v) => Ok(CellValue::Float(*v as f64)),
            CellValue::Boolean(b) => Ok(CellValue::Float(if *b { 1.0 } else { 0.0 })),
            CellValue::Text(s) => s.parse::<f64>().map(CellValue::Float).map_err(|_| ()),
            CellValue::Null => Ok(CellValue::Null),
        },
        FieldType::Boolean => match cell {
            CellValue::Boolean(b) => Ok(CellValue::Boolean(*b)),
            CellValue::Integer(v) => match v {
                0 => Ok(CellValue::Boolean(false)),
                1 => Ok(CellValue::Boolean(true)),
                _ => Err(()),
            },
            CellValue::Text(s) => match s.to_ascii_lowercase().as_str() {
                "true" | "1" => Ok(CellValue::Boolean(true)),
                "false" | "0" => Ok(CellValue::Boolean(false)),
                _ => Err(()),
            },
            _ => Err(()),
        },
        FieldType::Date => match cell {
            CellValue::Text(s) if is_date_shape(s) => Ok(CellValue::Text(s.clone())),
            _ => Err(()),
        },
        FieldType::Time => match cell {
            CellValue::Text(s) if is_time_shape(s) => Ok(CellValue::Text(s.clone())),
            _ => Err(()),
        },
        FieldType::DateTime => match cell {
            CellValue::Text(s) if is_datetime_shape(s) => Ok(CellValue::Text(s.clone())),
            _ => Err(()),
        },
        FieldType::Binary => Err(()),
    }
}

fn is_date_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
}

fn is_time_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 8
        && bytes[2] == b':'
        && bytes[5] == b':'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 2 || i == 5 || b.is_ascii_digit())
}

fn is_datetime_shape(value: &str) -> bool {
    value.len() == 19
        && is_date_shape(&value[..10])
        && value.as_bytes()[10] == b'T'
        && is_time_shape(&value[11..])
}

// ---------- brokered remote fetch ----------

/// Exact remote fetch identity bound to a brokered download.
#[derive(Debug, Clone)]
pub struct RemoteFetchSpec {
    pub provider: medscale_contracts::data_sources::RemoteDatasetProvider,
    pub repo: String,
    pub revision: String,
    pub files: Vec<String>,
}

/// One fetched file with its digest (quarantine/admission happens after).
#[derive(Debug, Clone)]
pub struct FetchedFile {
    pub name: String,
    pub bytes: Vec<u8>,
    pub digest: medscale_contracts::objects::DigestSha256,
}

/// Brokered dataset fetch over an injected transport. Every file is an exact
/// broker decision; transport bytes are hostile until digest-checked and
/// parsed. Live hosts without allowlist entries deny before any socket.
pub fn brokered_dataset_fetch(
    allowlist: &[medscale_contracts::network::EgressAllowlistEntry],
    transport: &dyn medscale_network::BrokerTransport,
    spec: &RemoteFetchSpec,
) -> Result<Vec<FetchedFile>, AcquireFail> {
    use medscale_contracts::data_sources::RemoteDatasetProvider;
    let host = match spec.provider {
        RemoteDatasetProvider::HuggingFace => "huggingface.co",
        RemoteDatasetProvider::Kaggle => "kaggle.com",
    };
    let mut out = Vec::new();
    for file in &spec.files {
        if file.len() as u64 > SNAPSHOT_BYTES_MAX {
            return Err(AcquireFail::Rejected(
                "remote file name exceeds bound".to_owned(),
            ));
        }
        let request = medscale_contracts::network::NetworkBrokerRequest {
            destination_host: host.to_owned(),
            destination_path: format!("/{}/{}/{}", spec.repo, spec.revision, file),
            purpose: EgressPurpose::DatasetMirrorRead,
            data_class: EgressDataClass::SyntheticFixture,
            authorization_token_id: None,
            body_digest: None,
            fixture_id: Some(format!(
                "dataset:{}:{}:{}:{}",
                match spec.provider {
                    RemoteDatasetProvider::HuggingFace => "hf",
                    RemoteDatasetProvider::Kaggle => "kaggle",
                },
                spec.repo,
                spec.revision,
                file
            )),
        };
        let outcome = medscale_network::broker_invoke(allowlist, &request, transport);
        match outcome.decision {
            medscale_contracts::network::BrokerDecision::Allow => {}
            medscale_contracts::network::BrokerDecision::Deny => {
                return match outcome.reason {
                    BrokerReasonCode::ExternalGateRequired => Err(AcquireFail::Unavailable(
                        "live dataset host requires an external gate".to_owned(),
                    )),
                    BrokerReasonCode::EmptyAllowlist
                    | BrokerReasonCode::UnknownDestination
                    | BrokerReasonCode::PurposeMismatch
                    | BrokerReasonCode::DataClassRefused
                    | BrokerReasonCode::Unauthorized => Err(AcquireFail::Denied(format!(
                        "broker denied dataset fetch: {:?}",
                        outcome.reason
                    ))),
                    _ => Err(AcquireFail::Unavailable(format!(
                        "broker denied dataset fetch: {:?}",
                        outcome.reason
                    ))),
                };
            }
        }
        let Some(body) = outcome.fixture_body else {
            return Err(AcquireFail::Unavailable(
                "dataset transport returned no bytes".to_owned(),
            ));
        };
        let bytes = body.into_bytes();
        if bytes.len() as u64 > SNAPSHOT_BYTES_MAX {
            return Err(AcquireFail::Quarantined(
                "remote file exceeds snapshot byte bound".to_owned(),
            ));
        }
        let digest = medscale_contracts::objects::DigestSha256::of(&bytes);
        out.push(FetchedFile {
            name: file.clone(),
            bytes,
            digest,
        });
    }
    Ok(out)
}

/// Parses one fetched file by extension and merges multi-file datasets.
/// All files must share one schema; divergence rejects the acquisition.
pub fn parse_fetched_files(files: &[FetchedFile]) -> Result<ParsedTable, AcquireFail> {
    if files.is_empty() {
        return Err(AcquireFail::Rejected(
            "remote dataset has no files".to_owned(),
        ));
    }
    let mut merged: Option<ParsedTable> = None;
    for file in files {
        let lower = file.name.to_ascii_lowercase();
        let parsed = if lower.ends_with(".csv") {
            parse_delimited(&file.bytes, b',')
        } else if lower.ends_with(".tsv") {
            parse_delimited(&file.bytes, b'\t')
        } else if lower.ends_with(".jsonl") || lower.ends_with(".ndjson") {
            parse_json_bytes(&file.bytes, true)
        } else if lower.ends_with(".json") {
            parse_json_bytes(&file.bytes, false)
        } else {
            return Err(AcquireFail::Unsupported(format!(
                "remote file {} has an unsupported extension",
                file.name
            )));
        }
        .map_err(|fail| match fail {
            AcquireFail::Rejected(reason) => AcquireFail::Quarantined(reason),
            other => other,
        })?;
        match &mut merged {
            None => merged = Some(parsed),
            Some(first) => {
                if parsed.fields != first.fields {
                    return Err(AcquireFail::Rejected(
                        "remote files disagree on schema".to_owned(),
                    ));
                }
                if (first.rows.len() + parsed.rows.len()) as u64 > SNAPSHOT_ROWS_MAX {
                    return Err(AcquireFail::Rejected(
                        "merged remote rows exceed snapshot row bound".to_owned(),
                    ));
                }
                first.rows.extend(parsed.rows);
                first.rows_skipped += parsed.rows_skipped;
                first.bytes_hashed += parsed.bytes_hashed;
                for warning in parsed.warnings {
                    push_warning(&mut first.warnings, warning);
                }
            }
        }
    }
    merged.ok_or_else(|| AcquireFail::Rejected("remote dataset is empty".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::data_sources::RemoteDatasetProvider;

    #[test]
    fn csv_quotes_and_ragged_rows() {
        let bytes = b"name,note,age\n\"doe, j\",\"a \"\"quoted\"\" note\",3\nbad,row\namy,,30\n";
        let table = parse_delimited(bytes, b',').expect("parse");
        assert_eq!(table.fields.len(), 3);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows_skipped, 1);
        assert_eq!(table.warnings.len(), 1);
        assert_eq!(table.rows[0][0], CellValue::Text("doe, j".to_owned()));
        assert_eq!(
            table.rows[0][1],
            CellValue::Text("a \"quoted\" note".to_owned())
        );
        assert_eq!(table.rows[0][2], CellValue::Integer(3));
        assert!(table.rows[1][1].is_null());
        // age column inferred integer.
        assert_eq!(table.fields[2].field_type, FieldType::Integer);
    }

    #[test]
    fn sparse_or_delimiter_only_input_hits_the_cell_bound() {
        // 900,000 empty objects plus 1,024 distinct keys pass the row and
        // field bounds separately but would allocate about 10^9 cells.
        let mut jsonl = "{}\n".repeat(900_000);
        for k in 0..1_024 {
            jsonl.push_str(&format!("{{\"k{k}\":1}}\n"));
        }
        assert_eq!(
            parse_json_bytes(jsonl.as_bytes(), true).unwrap_err(),
            cells_exceeded()
        );
        let commas = ",".repeat(TABLE_CELLS_MAX + 10);
        assert_eq!(
            parse_delimited(commas.as_bytes(), b',').unwrap_err(),
            cells_exceeded()
        );
    }

    #[test]
    fn csv_rejects_missing_header_and_empty_body() {
        assert!(parse_delimited(b"", b',').is_err());
        assert!(parse_delimited(b"a,b\n", b',').is_err());
        assert!(parse_delimited(b"a,a\n1,2\n", b',').is_err());
        assert!(parse_delimited(b"\xff\xfe", b',').is_err());
    }

    #[test]
    fn json_merges_types_and_counts_nested() {
        let bytes = br#"[{"a": 1, "b": "x"}, {"a": 2.5, "b": null, "c": {"k": 1}}]"#;
        let table = parse_json_bytes(bytes, false).expect("parse");
        assert_eq!(table.fields.len(), 3);
        // Integer + Float widens to Float.
        assert_eq!(table.fields[0].field_type, FieldType::Float);
        assert_eq!(table.rows[0][0], CellValue::Float(1.0));
        assert!(table.rows[1][1].is_null());
        assert_eq!(table.warnings.len(), 1);
        let lines = b"{\"a\": 1}\n{\"a\": 2}\n";
        let table = parse_json_bytes(lines, true).expect("jsonl");
        assert_eq!(table.rows.len(), 2);
        assert!(parse_json_bytes(b"[1, 2]", false).is_err());
    }

    #[test]
    fn filter_sort_page_deterministically() {
        let fields = vec![
            SchemaField {
                name: "city".to_owned(),
                field_type: FieldType::Text,
                nullable: true,
                declared_unit: None,
            },
            SchemaField {
                name: "dose".to_owned(),
                field_type: FieldType::Integer,
                nullable: true,
                declared_unit: None,
            },
        ];
        let rows = vec![
            vec![CellValue::Text("b".to_owned()), CellValue::Integer(5)],
            vec![CellValue::Text("a".to_owned()), CellValue::Integer(9)],
            vec![CellValue::Text("c".to_owned()), CellValue::Null],
        ];
        let filters = vec![FilterExpr {
            column: "dose".to_owned(),
            op: FilterOp::GreaterThan,
            value: "4".to_owned(),
        }];
        let sort = vec![SortKey {
            column: "city".to_owned(),
            descending: false,
        }];
        let order = apply_view(&rows, &fields, &filters, &sort);
        // Null dose never matches; survivors sorted by city.
        assert_eq!(order, vec![1, 0]);
    }

    #[test]
    fn transform_select_cast_and_strict_failure() {
        let fields = vec![
            SchemaField {
                name: "city".to_owned(),
                field_type: FieldType::Text,
                nullable: true,
                declared_unit: None,
            },
            SchemaField {
                name: "dose".to_owned(),
                field_type: FieldType::Text,
                nullable: true,
                declared_unit: None,
            },
        ];
        let rows = vec![
            vec![
                CellValue::Text("a".to_owned()),
                CellValue::Text("5".to_owned()),
            ],
            vec![
                CellValue::Text("b".to_owned()),
                CellValue::Text("oops".to_owned()),
            ],
        ];
        let ops = vec![
            TransformOp::SelectColumns {
                columns: vec!["dose".to_owned()],
            },
            TransformOp::CastType {
                column: "dose".to_owned(),
                to: FieldType::Integer,
                strict: false,
            },
        ];
        let (fields, rows, failures) = execute_transform(&fields, &rows, &ops).expect("transform");
        assert_eq!(fields.len(), 1);
        assert_eq!(rows[0][0], CellValue::Integer(5));
        assert!(rows[1][0].is_null());
        assert_eq!(failures, 1);
        let strict = vec![TransformOp::CastType {
            column: "dose".to_owned(),
            to: FieldType::Integer,
            strict: true,
        }];
        let before_fields = vec![SchemaField {
            name: "dose".to_owned(),
            field_type: FieldType::Text,
            nullable: true,
            declared_unit: None,
        }];
        let before_rows = vec![vec![CellValue::Text("oops".to_owned())]];
        assert!(execute_transform(&before_fields, &before_rows, &strict).is_err());
    }

    struct MapTransport {
        bodies: std::collections::HashMap<String, String>,
    }

    impl medscale_network::BrokerTransport for MapTransport {
        fn send(
            &self,
            req: &medscale_network::TransportRequest,
        ) -> Result<String, medscale_network::TransportError> {
            self.bodies
                .get(&req.path)
                .cloned()
                .ok_or(medscale_network::TransportError::Failed(
                    "no fixture".to_owned(),
                ))
        }
    }

    fn allow_entry() -> medscale_contracts::network::EgressAllowlistEntry {
        medscale_contracts::network::EgressAllowlistEntry {
            host: "huggingface.co".to_owned(),
            path_prefix: "/org/ds/".to_owned(),
            purposes: vec![EgressPurpose::DatasetMirrorRead],
            data_classes: vec![EgressDataClass::SyntheticFixture],
            enabled: true,
        }
    }

    #[test]
    fn brokered_fetch_parses_and_denies() {
        let mut bodies = std::collections::HashMap::new();
        bodies.insert("/org/ds/v1/data.csv".to_owned(), "a,b\n1,2\n".to_owned());
        let transport = MapTransport { bodies };
        let spec = RemoteFetchSpec {
            provider: RemoteDatasetProvider::HuggingFace,
            repo: "org/ds".to_owned(),
            revision: "v1".to_owned(),
            files: vec!["data.csv".to_owned()],
        };
        let fetched =
            brokered_dataset_fetch(std::slice::from_ref(&allow_entry()), &transport, &spec)
                .expect("fetch");
        assert_eq!(fetched.len(), 1);
        assert_eq!(
            fetched[0].digest,
            medscale_contracts::objects::DigestSha256::of(b"a,b\n1,2\n")
        );
        let table = parse_fetched_files(&fetched).expect("parse");
        assert_eq!(table.rows.len(), 1);
        // No allowlist entry: deny before any socket.
        let denied = brokered_dataset_fetch(&[], &transport, &spec);
        assert!(matches!(denied, Err(AcquireFail::Denied(_))));
        // Unknown extension: explicit unsupported.
        let bad = vec![FetchedFile {
            name: "data.parquet".to_owned(),
            bytes: b"raw".to_vec(),
            digest: medscale_contracts::objects::DigestSha256::of(b"raw"),
        }];
        assert!(matches!(
            parse_fetched_files(&bad),
            Err(AcquireFail::Unsupported(_))
        ));
    }
}
