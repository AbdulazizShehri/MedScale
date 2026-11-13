//! Lexical/structural gate and H0-B typed extractors for synthetic FHIR R4 JSON.

mod extractors;
mod lexical;
mod units;

pub use extractors::{
    CONDITION_V1, ConditionExtractor, ExtractContext, Extraction, OBSERVATION_V1,
    ObservationExtractor, PATIENT_V1, PatientExtractor, TypedResourceExtractor, extract_resource,
    unit_conflict_status,
};
pub use lexical::{
    LexicalError, LexicalReport, extract_patient_identifiers, gate_fhir_json, parse_exact_decimals,
};
pub use units::{ADMITTED_UCUM_CODES, compare_units, evaluate_unit, same_quantity, subset_pin};

/// Maximum accepted ingest payload size (bytes): 8 MiB (B2). Each import is
/// stored whole, so this stays well under what a snapshot rewrite can carry.
pub const MAX_INGEST_BYTES: usize = 8 * 1_048_576;

/// Maximum JSON nesting depth.
pub const MAX_JSON_DEPTH: usize = 64;
