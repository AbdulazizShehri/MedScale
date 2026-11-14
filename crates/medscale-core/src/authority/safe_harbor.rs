//! HIPAA Safe Harbor de-identification of FHIR JSON (45 CFR 164.514(b)(2)).
//!
//! Every field is handled by a fixed rule, never by detection, so removal
//! does not depend on a recognizer noticing a value. A field this module has
//! no rule for is refused, not passed through. Free text is removed rather
//! than scanned. After the rules run, the strings that remain are re-scanned
//! with the deterministic recognizers and any finding blocks the output.
//!
//! The output carries no claim of compliance. Safe Harbor also requires that
//! the covered entity has no actual knowledge that the remaining information
//! could identify the person (164.514(b)(2)(ii)); software cannot assess
//! that, and every report says so. The report holds counts and categories
//! only, never a removed value.
//!
//! Resource ids and references are replaced with sequence numbers that are
//! not derived from the originals (164.514(c)), consistent within one run so
//! a Bundle keeps its internal links.

use std::collections::BTreeMap;
use std::fmt;

use medscale_contracts::privacy_gate::SensitiveSpanKind;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::privacy_recognizers::pattern_recognize;

pub const SAFE_HARBOR_METHOD: &str = "hipaa_safe_harbor_45cfr164_514_b_2";
pub const SAFE_HARBOR_ENGINE_VERSION: &str = "1";
pub const SAFE_HARBOR_REPORT_SCHEMA_VERSION: u32 = 1;
pub const SAFE_HARBOR_MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEPTH: usize = 64;
const AGE_LIMIT_YEARS: f64 = 89.0;

/// Three-digit ZIP prefixes that HHS de-identification guidance (2012) lists
/// as covering 20,000 or fewer people, from 2000 Census data. They become
/// `000`. The list has to be re-derived from current Census data before the
/// output is relied on; every report says so.
pub const RESTRICTED_ZIP3: [&str; 17] = [
    "036", "059", "063", "102", "203", "556", "692", "790", "821", "823", "830", "831", "878",
    "879", "884", "890", "893",
];

/// The 18 identifier categories of 164.514(b)(2)(i)(A)-(R), in rule order.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SafeHarborCategory {
    Names,
    Geographic,
    Dates,
    Telephone,
    Fax,
    Email,
    SocialSecurity,
    MedicalRecord,
    HealthPlan,
    Account,
    CertificateLicense,
    Vehicle,
    Device,
    Url,
    IpAddress,
    Biometric,
    Photo,
    #[default]
    OtherUnique,
}

impl SafeHarborCategory {
    pub const ALL: [Self; 18] = [
        Self::Names,
        Self::Geographic,
        Self::Dates,
        Self::Telephone,
        Self::Fax,
        Self::Email,
        Self::SocialSecurity,
        Self::MedicalRecord,
        Self::HealthPlan,
        Self::Account,
        Self::CertificateLicense,
        Self::Vehicle,
        Self::Device,
        Self::Url,
        Self::IpAddress,
        Self::Biometric,
        Self::Photo,
        Self::OtherUnique,
    ];

    /// The paragraph letter in 164.514(b)(2)(i).
    #[must_use]
    pub const fn rule_letter(self) -> char {
        match self {
            Self::Names => 'A',
            Self::Geographic => 'B',
            Self::Dates => 'C',
            Self::Telephone => 'D',
            Self::Fax => 'E',
            Self::Email => 'F',
            Self::SocialSecurity => 'G',
            Self::MedicalRecord => 'H',
            Self::HealthPlan => 'I',
            Self::Account => 'J',
            Self::CertificateLicense => 'K',
            Self::Vehicle => 'L',
            Self::Device => 'M',
            Self::Url => 'N',
            Self::IpAddress => 'O',
            Self::Biometric => 'P',
            Self::Photo => 'Q',
            Self::OtherUnique => 'R',
        }
    }

    const fn of_span(kind: SensitiveSpanKind) -> Self {
        match kind {
            SensitiveSpanKind::PersonName => Self::Names,
            SensitiveSpanKind::Date => Self::Dates,
            SensitiveSpanKind::Phone => Self::Telephone,
            SensitiveSpanKind::Email => Self::Email,
            SensitiveSpanKind::PostalAddress | SensitiveSpanKind::PostalCode => Self::Geographic,
            SensitiveSpanKind::Url => Self::Url,
            SensitiveSpanKind::IpAddress => Self::IpAddress,
            SensitiveSpanKind::Identifier | SensitiveSpanKind::ModelEntity => Self::OtherUnique,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryOutcome {
    pub category: SafeHarborCategory,
    pub removed: u32,
    pub generalized: u32,
    pub replaced: u32,
}

/// What a report cannot establish. Every report lists all of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafeHarborLimitation {
    /// 164.514(b)(2)(ii): no actual knowledge that the rest could identify
    /// the person. Only the covered entity can establish this.
    ActualKnowledgeNotAssessed,
    /// The restricted ZIP3 list comes from 2000 Census data.
    RestrictedZip3ListNeedsCurrentCensus,
    /// Numbers are kept as they are; an identifier stored as a number is not
    /// recognized.
    NumericValuesNotInspected,
    /// The re-scan of kept text uses deterministic recognizers that can miss
    /// values.
    ResidualScanIsIncomplete,
    /// Ages are judged against the reference year given by the caller.
    AgeJudgedAgainstReferenceYear,
}

impl SafeHarborLimitation {
    pub const ALL: [Self; 5] = [
        Self::ActualKnowledgeNotAssessed,
        Self::RestrictedZip3ListNeedsCurrentCensus,
        Self::NumericValuesNotInspected,
        Self::ResidualScanIsIncomplete,
        Self::AgeJudgedAgainstReferenceYear,
    ];
}

/// Counts and categories for one run. Never carries a removed value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafeHarborReport {
    pub schema_version: u32,
    pub method: String,
    pub engine_version: String,
    pub reference_year: i32,
    pub resources: u32,
    pub resources_reduced_to_id: u32,
    pub categories: Vec<CategoryOutcome>,
    pub free_text_fields_removed: u32,
    pub narratives_removed: u32,
    pub extensions_removed: u32,
    pub attachments_removed: u32,
    pub ages_aggregated_90_or_older: u32,
    pub residual_strings_scanned: u32,
    pub limitations: Vec<SafeHarborLimitation>,
}

impl SafeHarborReport {
    #[must_use]
    pub fn category(&self, category: SafeHarborCategory) -> CategoryOutcome {
        self.categories
            .iter()
            .copied()
            .find(|c| c.category == category)
            .unwrap_or(CategoryOutcome {
                category,
                ..CategoryOutcome::default()
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeHarborOutput {
    pub json: String,
    pub report: SafeHarborReport,
}

/// Why a run was refused. Paths name JSON keys and indices only, never a
/// value; keys that are not plain identifiers are shown as `?`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafeHarborError {
    TooLarge,
    Malformed,
    NotFhir,
    TooDeep,
    BadReferenceYear,
    UnsupportedResource {
        resource_type: String,
    },
    UnknownField {
        path: String,
    },
    ModifierExtension {
        path: String,
    },
    UnreadableReference {
        path: String,
    },
    ResidualFinding {
        category: SafeHarborCategory,
        path: String,
    },
}

impl fmt::Display for SafeHarborError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => write!(f, "the file is larger than the Safe Harbor limit"),
            Self::Malformed => write!(f, "the file is not valid JSON"),
            Self::NotFhir => write!(f, "the file is not a FHIR resource"),
            Self::TooDeep => write!(f, "the file is nested too deeply"),
            Self::BadReferenceYear => write!(f, "the reference year is out of range"),
            Self::UnsupportedResource { resource_type } => {
                write!(f, "no Safe Harbor rules for resource type {resource_type}")
            }
            Self::UnknownField { path } => write!(f, "no Safe Harbor rule for field {path}"),
            Self::ModifierExtension { path } => {
                write!(f, "a modifier extension at {path} cannot be removed safely")
            }
            Self::UnreadableReference { path } => write!(f, "unreadable reference at {path}"),
            Self::ResidualFinding { category, path } => write!(
                f,
                "kept text at {path} still looks like an identifier ({})",
                category.rule_letter()
            ),
        }
    }
}

impl std::error::Error for SafeHarborError {}

/// Resources de-identified field by field.
const FULL_RESOURCES: &[&str] = &[
    "Bundle",
    "Patient",
    "Observation",
    "Condition",
    "Encounter",
    "Medication",
    "MedicationRequest",
    "MedicationStatement",
    "AllergyIntolerance",
    "Immunization",
    "Procedure",
    "DiagnosticReport",
];

/// Resources about people or places other than the patient: only the type
/// and a replaced id are kept, so references to them still resolve.
const REDUCED_RESOURCES: &[&str] = &[
    "Practitioner",
    "PractitionerRole",
    "Organization",
    "Location",
    "RelatedPerson",
];

/// Coded and structural string fields kept as they are (then re-scanned).
const KEEP_STRING_KEYS: &[&str] = &[
    "resourceType",
    "status",
    "gender",
    "system",
    "code",
    "version",
    "unit",
    "comparator",
    "use",
    "language",
    "intent",
    "priority",
    "profile",
    "versionId",
    "valueCode",
    "valueTime",
    "state",
    "country",
    "type",
    "mode",
    "category",
    "criticality",
    "severity",
    "series",
    "doseNumberString",
    "seriesDosesString",
    "periodUnit",
    "durationUnit",
    "dayOfWeek",
    "when",
    "timeOfDay",
    "display",
];

/// Kept strings that are codes or URIs from terminologies, not prose: not
/// re-scanned, because codes such as SNOMED numbers look like identifiers.
const CODE_KEYS: &[&str] = &[
    "resourceType",
    "system",
    "code",
    "version",
    "profile",
    "versionId",
    "valueCode",
];

const FREE_TEXT_KEYS: &[&str] = &[
    "note",
    "comment",
    "description",
    "patientInstruction",
    "valueString",
    "valueMarkdown",
    "title",
    "conclusion",
    "summary",
    "purpose",
];

const ATTACHMENT_KEYS: &[&str] = &["valueAttachment", "content", "presentedForm", "attachment"];

const DEVICE_KEYS: &[&str] = &[
    "udiCarrier",
    "serialNumber",
    "lotNumber",
    "distinctIdentifier",
    "modelNumber",
];

const URL_KEYS: &[&str] = &["url", "valueUri", "valueUrl", "source", "implicitRules"];

const AGE_KEYS: &[&str] = &["onsetAge", "abatementAge", "valueAge", "ageAge"];

/// LOINC codes whose `valueQuantity` is a person's age.
const AGE_OBSERVATION_CODES: &[&str] = &["30525-0", "21612-7", "29553-5"];

/// Path segments for error messages. Keys come from the input, so anything
/// that is not a plain identifier is masked.
struct JsonPath(Vec<String>);

impl JsonPath {
    fn push_key(&mut self, key: &str) {
        let plain = !key.is_empty()
            && key.len() <= 64
            && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
        self.0.push(if plain {
            key.to_owned()
        } else {
            "?".to_owned()
        });
    }

    fn push_index(&mut self, index: usize) {
        self.0.push(format!("[{index}]"));
    }

    fn pop(&mut self) {
        self.0.pop();
    }

    fn render(&self) -> String {
        let mut out = String::new();
        for seg in &self.0 {
            if !seg.starts_with('[') && !out.is_empty() {
                out.push('.');
            }
            out.push_str(seg);
        }
        if out.is_empty() { "$".to_owned() } else { out }
    }
}

struct Engine {
    reference_year: i32,
    ids: BTreeMap<String, String>,
    counts: BTreeMap<SafeHarborCategory, CategoryOutcome>,
    resources: u32,
    reduced: u32,
    free_text: u32,
    narratives: u32,
    extensions: u32,
    attachments: u32,
    ages: u32,
    residual_scanned: u32,
    path: JsonPath,
}

#[derive(Clone, Copy)]
enum Tally {
    Removed,
    Generalized,
    Replaced,
}

impl Engine {
    fn tally(&mut self, category: SafeHarborCategory, how: Tally) {
        let entry = self.counts.entry(category).or_insert(CategoryOutcome {
            category,
            ..CategoryOutcome::default()
        });
        match how {
            Tally::Removed => entry.removed += 1,
            Tally::Generalized => entry.generalized += 1,
            Tally::Replaced => entry.replaced += 1,
        }
    }

    fn unknown(&self) -> SafeHarborError {
        SafeHarborError::UnknownField {
            path: self.path.render(),
        }
    }

    /// A replacement id that says nothing about the original.
    fn replaced_id(&mut self, original: &str) -> String {
        let next = self.ids.len() + 1;
        let id = self
            .ids
            .entry(original.to_owned())
            .or_insert_with(|| format!("00000000-0000-4000-8000-{next:012x}"))
            .clone();
        self.tally(SafeHarborCategory::OtherUnique, Tally::Replaced);
        id
    }

    fn unreadable(&self) -> SafeHarborError {
        SafeHarborError::UnreadableReference {
            path: self.path.render(),
        }
    }

    fn removed(&mut self, category: SafeHarborCategory) -> Result<Option<Value>, SafeHarborError> {
        self.tally(category, Tally::Removed);
        Ok(None)
    }

    fn reference(&mut self, value: &str) -> Result<String, SafeHarborError> {
        if let Some(local) = value.strip_prefix('#') {
            return Ok(if local.is_empty() {
                "#".to_owned()
            } else {
                format!("#{}", self.replaced_id(local))
            });
        }
        if let Some(uuid) = value.strip_prefix("urn:uuid:") {
            if uuid.is_empty() {
                return Err(self.unreadable());
            }
            return Ok(format!("urn:uuid:{}", self.replaced_id(uuid)));
        }
        let segments: Vec<&str> = value.split('/').filter(|s| !s.is_empty()).collect();
        let history = segments.iter().position(|s| *s == "_history");
        let (resource_type, id) = match history {
            Some(i) if i >= 2 => (segments[i - 2], segments[i - 1]),
            None if segments.len() >= 2 => {
                (segments[segments.len() - 2], segments[segments.len() - 1])
            }
            _ => return Err(self.unreadable()),
        };
        let type_ok = resource_type
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
            && resource_type.chars().all(|c| c.is_ascii_alphabetic());
        if !type_ok {
            return Err(self.unreadable());
        }
        Ok(format!("{resource_type}/{}", self.replaced_id(id)))
    }

    fn full_url(&mut self, value: &str) -> Result<String, SafeHarborError> {
        let replaced = self.reference(value)?;
        Ok(match replaced.rsplit_once('/') {
            Some((_, id)) => format!("urn:uuid:{id}"),
            None => replaced,
        })
    }

    fn resource(
        &mut self,
        map: Map<String, Value>,
        depth: usize,
    ) -> Result<Value, SafeHarborError> {
        let resource_type = map
            .get("resourceType")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        self.resources += 1;
        if REDUCED_RESOURCES.contains(&resource_type.as_str()) {
            self.reduced += 1;
            self.tally(SafeHarborCategory::Names, Tally::Removed);
            let mut out = Map::new();
            out.insert("resourceType".to_owned(), Value::String(resource_type));
            if let Some(Value::String(id)) = map.get("id") {
                let id = self.replaced_id(id);
                out.insert("id".to_owned(), Value::String(id));
            }
            return Ok(Value::Object(out));
        }
        if !FULL_RESOURCES.contains(&resource_type.as_str()) {
            let shown = if !resource_type.is_empty()
                && resource_type.len() <= 64
                && resource_type.chars().all(|c| c.is_ascii_alphabetic())
            {
                resource_type
            } else {
                "?".to_owned()
            };
            return Err(SafeHarborError::UnsupportedResource {
                resource_type: shown,
            });
        }
        let age_observation = resource_type == "Observation" && is_age_observation(&map);
        let is_patient = resource_type == "Patient";
        let mut out = self.object(map, depth, age_observation)?;
        if is_patient {
            self.birth_date(&mut out);
        }
        Ok(Value::Object(out))
    }

    /// Year only, unless the year shows the person is 90 or older: then the
    /// birth date goes entirely (164.514(b)(2)(i)(C)).
    fn birth_date(&mut self, out: &mut Map<String, Value>) {
        let Some(Value::String(year)) = out.get("birthDate") else {
            return;
        };
        let Ok(year) = year.parse::<i32>() else {
            return;
        };
        if self.reference_year - year >= 90 {
            out.remove("birthDate");
            self.ages += 1;
            self.tally(SafeHarborCategory::Dates, Tally::Removed);
        }
    }

    fn object(
        &mut self,
        map: Map<String, Value>,
        depth: usize,
        age_observation: bool,
    ) -> Result<Map<String, Value>, SafeHarborError> {
        if depth > MAX_DEPTH {
            return Err(SafeHarborError::TooDeep);
        }
        let is_coding = !map.contains_key("reference")
            && (map.contains_key("code") || map.contains_key("system"));
        let mut out = Map::new();
        for (key, value) in map {
            self.path.push_key(&key);
            let kept = self.field(&key, value, depth, is_coding, age_observation)?;
            self.path.pop();
            if let Some(kept) = kept {
                out.insert(key, kept);
            }
        }
        Ok(out)
    }

    /// The rule for one field. `None` removes it.
    fn field(
        &mut self,
        key: &str,
        value: Value,
        depth: usize,
        is_coding: bool,
        age_observation: bool,
    ) -> Result<Option<Value>, SafeHarborError> {
        use SafeHarborCategory as C;
        match key {
            "modifierExtension" => Err(SafeHarborError::ModifierExtension {
                path: self.path.render(),
            }),
            "extension" => {
                self.extensions += 1;
                Ok(None)
            }
            "name" | "alias" | "contact" => self.removed(C::Names),
            "display" if !is_coding => self.removed(C::Names),
            "telecom" => {
                for item in as_items(&value) {
                    let system = item.get("system").and_then(Value::as_str).unwrap_or("");
                    let category = match system {
                        "fax" => C::Fax,
                        "email" => C::Email,
                        "url" => C::Url,
                        _ => C::Telephone,
                    };
                    self.tally(category, Tally::Removed);
                }
                Ok(None)
            }
            "identifier" => {
                for item in as_items(&value) {
                    let category = identifier_category(item);
                    self.tally(category, Tally::Removed);
                }
                Ok(None)
            }
            "photo" => self.removed(C::Photo),
            "link" | "signature" | "request" | "response" => self.removed(C::OtherUnique),
            k if DEVICE_KEYS.contains(&k) => self.removed(C::Device),
            k if URL_KEYS.contains(&k) => self.removed(C::Url),
            k if ATTACHMENT_KEYS.contains(&k) => {
                self.attachments += 1;
                Ok(None)
            }
            k if FREE_TEXT_KEYS.contains(&k) => {
                self.free_text += 1;
                Ok(None)
            }
            "text" => {
                if value.is_object() {
                    self.narratives += 1;
                } else {
                    self.free_text += 1;
                }
                Ok(None)
            }
            "address" => Ok(Some(self.addresses(value, depth)?)),
            "id" => match value {
                Value::String(id) => Ok(Some(Value::String(self.replaced_id(&id)))),
                _ => Err(self.unknown()),
            },
            "reference" => match value {
                Value::String(r) => Ok(Some(Value::String(self.reference(&r)?))),
                _ => Err(self.unknown()),
            },
            "fullUrl" => match value {
                Value::String(r) => Ok(Some(Value::String(self.full_url(&r)?))),
                _ => Err(self.unknown()),
            },
            k if AGE_KEYS.contains(&k) || (age_observation && k == "valueQuantity") => {
                let value = self.value(key, value, depth + 1)?;
                Ok(Some(self.age(value)))
            }
            _ => Ok(Some(self.value(key, value, depth + 1)?)),
        }
    }

    fn value(&mut self, key: &str, value: Value, depth: usize) -> Result<Value, SafeHarborError> {
        if depth > MAX_DEPTH {
            return Err(SafeHarborError::TooDeep);
        }
        match value {
            Value::Object(map) => {
                if map.contains_key("resourceType") {
                    self.resource(map, depth)
                } else {
                    Ok(Value::Object(self.object(map, depth, false)?))
                }
            }
            Value::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for (i, item) in items.into_iter().enumerate() {
                    self.path.push_index(i);
                    out.push(self.value(key, item, depth + 1)?);
                    self.path.pop();
                }
                Ok(Value::Array(out))
            }
            Value::String(s) => self.string(key, s).map(Value::String),
            other => Ok(other),
        }
    }

    fn string(&mut self, key: &str, s: String) -> Result<String, SafeHarborError> {
        if !CODE_KEYS.contains(&key)
            && let Some(year) = fhir_date_year(&s)
        {
            if s.len() > 4 {
                self.tally(SafeHarborCategory::Dates, Tally::Generalized);
            }
            return Ok(year.to_owned());
        }
        if !KEEP_STRING_KEYS.contains(&key) {
            return Err(self.unknown());
        }
        if !CODE_KEYS.contains(&key) {
            self.residual_scanned += 1;
            if let Some(span) = pattern_recognize(&s).first() {
                return Err(SafeHarborError::ResidualFinding {
                    category: SafeHarborCategory::of_span(span.kind),
                    path: self.path.render(),
                });
            }
        }
        Ok(s)
    }

    /// Keeps state, country and a three-digit ZIP; removes the rest.
    fn addresses(&mut self, value: Value, depth: usize) -> Result<Value, SafeHarborError> {
        match value {
            Value::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for (i, item) in items.into_iter().enumerate() {
                    self.path.push_index(i);
                    out.push(self.addresses(item, depth + 1)?);
                    self.path.pop();
                }
                Ok(Value::Array(out))
            }
            Value::Object(map) => {
                let us = map
                    .get("country")
                    .and_then(Value::as_str)
                    .is_none_or(|c| matches!(c.trim(), "US" | "USA" | "United States"));
                let mut out = Map::new();
                for (key, child) in map {
                    self.path.push_key(&key);
                    match key.as_str() {
                        "use" | "type" | "state" | "country" | "period" => {
                            let kept = self.value(&key, child, depth + 1)?;
                            out.insert(key.clone(), kept);
                        }
                        "postalCode" => {
                            if let Some(zip3) = child.as_str().and_then(|z| zip3(z, us)) {
                                self.tally(SafeHarborCategory::Geographic, Tally::Generalized);
                                out.insert(key.clone(), Value::String(zip3));
                            } else {
                                self.tally(SafeHarborCategory::Geographic, Tally::Removed);
                            }
                        }
                        "extension" => self.extensions += 1,
                        _ => self.tally(SafeHarborCategory::Geographic, Tally::Removed),
                    }
                    self.path.pop();
                }
                Ok(Value::Object(out))
            }
            _ => Err(self.unknown()),
        }
    }

    /// Ages over 89 become "90 or older" (164.514(b)(2)(i)(C)).
    fn age(&mut self, value: Value) -> Value {
        let Value::Object(mut map) = value else {
            return value;
        };
        let Some(number) = map.get("value").and_then(Value::as_f64) else {
            return Value::Object(map);
        };
        let unit = map
            .get("code")
            .or_else(|| map.get("unit"))
            .and_then(Value::as_str)
            .unwrap_or("a");
        let years = match unit {
            "mo" | "month" | "months" => number / 12.0,
            "wk" | "week" | "weeks" => number / 52.0,
            "d" | "day" | "days" => number / 365.0,
            "h" | "min" => 0.0,
            _ => number,
        };
        if years > AGE_LIMIT_YEARS {
            map.insert("value".to_owned(), Value::from(90));
            map.insert("comparator".to_owned(), Value::String(">=".to_owned()));
            map.insert("unit".to_owned(), Value::String("years".to_owned()));
            map.insert(
                "system".to_owned(),
                Value::String("http://unitsofmeasure.org".to_owned()),
            );
            map.insert("code".to_owned(), Value::String("a".to_owned()));
            self.ages += 1;
            self.tally(SafeHarborCategory::Dates, Tally::Generalized);
        }
        Value::Object(map)
    }

    fn report(self) -> SafeHarborReport {
        let categories = SafeHarborCategory::ALL
            .iter()
            .map(|c| {
                self.counts.get(c).copied().unwrap_or(CategoryOutcome {
                    category: *c,
                    ..CategoryOutcome::default()
                })
            })
            .collect();
        SafeHarborReport {
            schema_version: SAFE_HARBOR_REPORT_SCHEMA_VERSION,
            method: SAFE_HARBOR_METHOD.to_owned(),
            engine_version: SAFE_HARBOR_ENGINE_VERSION.to_owned(),
            reference_year: self.reference_year,
            resources: self.resources,
            resources_reduced_to_id: self.reduced,
            categories,
            free_text_fields_removed: self.free_text,
            narratives_removed: self.narratives,
            extensions_removed: self.extensions,
            attachments_removed: self.attachments,
            ages_aggregated_90_or_older: self.ages,
            residual_strings_scanned: self.residual_scanned,
            limitations: SafeHarborLimitation::ALL.to_vec(),
        }
    }
}

fn as_items(value: &Value) -> Vec<&Map<String, Value>> {
    match value {
        Value::Array(items) => items.iter().filter_map(Value::as_object).collect(),
        Value::Object(map) => vec![map],
        _ => Vec::new(),
    }
}

/// Category of a FHIR Identifier from its HL7 v2-0203 type code or system.
fn identifier_category(identifier: &Map<String, Value>) -> SafeHarborCategory {
    use SafeHarborCategory as C;
    let system = identifier
        .get("system")
        .and_then(Value::as_str)
        .unwrap_or("");
    if system.contains("us-ssn") || system.ends_with("2.16.840.1.113883.4.1") {
        return C::SocialSecurity;
    }
    let codes = identifier
        .get("type")
        .and_then(|t| t.get("coding"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| c.get("code").and_then(Value::as_str));
    for code in codes {
        match code {
            "SS" | "SB" => return C::SocialSecurity,
            "MR" | "MRN" | "PI" => return C::MedicalRecord,
            "MB" | "MA" | "MC" | "SN" | "NIIP" => return C::HealthPlan,
            "AN" | "ANON" | "BA" | "BC" => return C::Account,
            "DL" | "PPN" | "LN" | "MD" | "DEA" | "NPI" => return C::CertificateLicense,
            _ => {}
        }
    }
    C::OtherUnique
}

fn is_age_observation(map: &Map<String, Value>) -> bool {
    map.get("code")
        .and_then(|c| c.get("coding"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| c.get("code").and_then(Value::as_str))
        .any(|code| AGE_OBSERVATION_CODES.contains(&code))
}

/// The year of a FHIR `date`, `dateTime` or `instant`, if `s` is one.
fn fhir_date_year(s: &str) -> Option<&str> {
    let b = s.as_bytes();
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    if b.len() < 4 || !digits(0..4) {
        return None;
    }
    let ok = match b.len() {
        4 => true,
        7 => b[4] == b'-' && digits(5..7),
        n if n >= 10 => {
            b[4] == b'-'
                && digits(5..7)
                && b[7] == b'-'
                && digits(8..10)
                && (n == 10 || b[10] == b'T')
        }
        _ => false,
    };
    ok.then(|| &s[..4])
}

/// The first three digits of a US ZIP code, or `000` for a restricted prefix.
/// `None` (remove the field) for anything that is not a US ZIP.
fn zip3(zip: &str, us: bool) -> Option<String> {
    let digits: String = zip.chars().filter(char::is_ascii_digit).collect();
    let shape_ok = matches!(digits.len(), 5 | 9)
        && zip
            .chars()
            .all(|c| c.is_ascii_digit() || c == '-' || c == ' ');
    if !us || !shape_ok {
        return None;
    }
    let prefix = &digits[..3];
    Some(if RESTRICTED_ZIP3.contains(&prefix) {
        "000".to_owned()
    } else {
        prefix.to_owned()
    })
}

/// De-identifies one FHIR JSON resource or Bundle under Safe Harbor rules.
/// `reference_year` is the year ages are judged against (normally the
/// current year).
pub fn deidentify_fhir_json(
    text: &str,
    reference_year: i32,
) -> Result<SafeHarborOutput, SafeHarborError> {
    if text.len() > SAFE_HARBOR_MAX_INPUT_BYTES {
        return Err(SafeHarborError::TooLarge);
    }
    if !(1900..=9999).contains(&reference_year) {
        return Err(SafeHarborError::BadReferenceYear);
    }
    let parsed: Value = serde_json::from_str(text).map_err(|_| SafeHarborError::Malformed)?;
    let Value::Object(map) = parsed else {
        return Err(SafeHarborError::NotFhir);
    };
    if !map.get("resourceType").is_some_and(Value::is_string) {
        return Err(SafeHarborError::NotFhir);
    }
    let mut engine = Engine {
        reference_year,
        ids: BTreeMap::new(),
        counts: BTreeMap::new(),
        resources: 0,
        reduced: 0,
        free_text: 0,
        narratives: 0,
        extensions: 0,
        attachments: 0,
        ages: 0,
        residual_scanned: 0,
        path: JsonPath(Vec::new()),
    };
    let out = engine.resource(map, 0)?;
    let json = serde_json::to_string(&out).map_err(|_| SafeHarborError::Malformed)?;
    Ok(SafeHarborOutput {
        json,
        report: engine.report(),
    })
}

#[cfg(test)]
#[path = "safe_harbor_tests.rs"]
mod tests;
