//! FHIR interchange support matrix and loss-aware export (Spec 020 / Trusted V1 Q08).
//!
//! External validator output remains evidence, never authority.
//! Full FHIR conformance is never claimed from the closed extractor subset.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::FHIR_R4_VERSION;
use crate::objects::LossClass;

/// Per-axis support honesty for FHIR interchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportLevel {
    /// Actively qualified by MedScale tests for this axis.
    Qualified,
    /// Narrow subset only; not full axis coverage.
    Partial,
    /// Not supported; must not be assumed present.
    Unsupported,
    /// External tool output may be attached as evidence only.
    EvidenceOnly,
}

/// Named support axes required by Trusted V1 Q08.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportAxis {
    Lexical,
    Structural,
    Profile,
    Terminology,
    References,
    Provenance,
    ClinicalInterpretation,
}

/// Axis levels for one resource type (or default row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceSupportStatus {
    pub resource_type: String,
    pub lexical: SupportLevel,
    pub structural: SupportLevel,
    pub profile: SupportLevel,
    pub terminology: SupportLevel,
    pub references: SupportLevel,
    pub provenance: SupportLevel,
    pub clinical_interpretation: SupportLevel,
    pub notes: Vec<String>,
}

impl ResourceSupportStatus {
    #[must_use]
    pub fn level(&self, axis: SupportAxis) -> SupportLevel {
        match axis {
            SupportAxis::Lexical => self.lexical,
            SupportAxis::Structural => self.structural,
            SupportAxis::Profile => self.profile,
            SupportAxis::Terminology => self.terminology,
            SupportAxis::References => self.references,
            SupportAxis::Provenance => self.provenance,
            SupportAxis::ClinicalInterpretation => self.clinical_interpretation,
        }
    }
}

/// Machine-readable FHIR support matrix (not a live CapabilityStatement server).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FhirSupportMatrix {
    pub fhir_version: String,
    pub synthetic_only: bool,
    pub full_conformance_claimed: bool,
    pub release_ready: bool,
    pub validator_is_authority: bool,
    pub resources: Vec<ResourceSupportStatus>,
    pub default_unsupported: ResourceSupportStatus,
    pub limitations: Vec<String>,
}

impl FhirSupportMatrix {
    /// Canonical Trusted V1 Q08 READY_BASE matrix — honest narrow subset only.
    #[must_use]
    pub fn trusted_v1_ready_base() -> Self {
        let patient = ResourceSupportStatus {
            resource_type: "Patient".to_owned(),
            lexical: SupportLevel::Qualified,
            structural: SupportLevel::Partial,
            profile: SupportLevel::Unsupported,
            terminology: SupportLevel::Unsupported,
            references: SupportLevel::Unsupported,
            provenance: SupportLevel::Partial,
            clinical_interpretation: SupportLevel::Unsupported,
            notes: vec![
                "Closed extractor patient.v1: id, birthDate, name.family/given only".to_owned(),
            ],
        };
        let observation = ResourceSupportStatus {
            resource_type: "Observation".to_owned(),
            lexical: SupportLevel::Qualified,
            structural: SupportLevel::Partial,
            profile: SupportLevel::Unsupported,
            terminology: SupportLevel::Partial,
            references: SupportLevel::Unsupported,
            provenance: SupportLevel::Partial,
            clinical_interpretation: SupportLevel::Unsupported,
            notes: vec![
                "Closed extractor observation.v1; UCUM subset pin only — not full terminology"
                    .to_owned(),
            ],
        };
        let condition = ResourceSupportStatus {
            resource_type: "Condition".to_owned(),
            lexical: SupportLevel::Qualified,
            structural: SupportLevel::Partial,
            profile: SupportLevel::Unsupported,
            terminology: SupportLevel::Unsupported,
            references: SupportLevel::Unsupported,
            provenance: SupportLevel::Partial,
            clinical_interpretation: SupportLevel::Unsupported,
            notes: vec!["Closed extractor condition.v1: code + clinicalStatus subset".to_owned()],
        };
        let default_unsupported = ResourceSupportStatus {
            resource_type: "*".to_owned(),
            lexical: SupportLevel::Partial,
            structural: SupportLevel::Unsupported,
            profile: SupportLevel::Unsupported,
            terminology: SupportLevel::Unsupported,
            references: SupportLevel::Unsupported,
            provenance: SupportLevel::EvidenceOnly,
            clinical_interpretation: SupportLevel::Unsupported,
            notes: vec![
                "Non-extracted resource types may pass lexical JSON gate only; no structural claim"
                    .to_owned(),
                "External validator outcomes attach as EvaluationRecord evidence only".to_owned(),
            ],
        };
        Self {
            fhir_version: FHIR_R4_VERSION.to_owned(),
            synthetic_only: true,
            full_conformance_claimed: false,
            release_ready: false,
            validator_is_authority: false,
            resources: vec![patient, observation, condition],
            default_unsupported,
            limitations: vec![
                "Narrow typed subset is not FHIR R4 full conformance".to_owned(),
                "Profile validation not qualified".to_owned(),
                "Reference resolution not qualified".to_owned(),
                "Clinical interpretation never invented by interchange layer".to_owned(),
                "RELEASE_READY remains false".to_owned(),
            ],
        }
    }

    #[must_use]
    pub fn status_for(&self, resource_type: &str) -> &ResourceSupportStatus {
        self.resources
            .iter()
            .find(|r| r.resource_type == resource_type)
            .unwrap_or(&self.default_unsupported)
    }

    /// Honesty invariant for READY_BASE.
    #[must_use]
    pub fn is_honest_ready_base(&self) -> bool {
        !self.full_conformance_claimed
            && !self.release_ready
            && !self.validator_is_authority
            && self.synthetic_only
            && self.fhir_version == FHIR_R4_VERSION
            && self.resources.iter().all(|r| {
                r.profile == SupportLevel::Unsupported
                    && r.clinical_interpretation == SupportLevel::Unsupported
                    && r.references == SupportLevel::Unsupported
            })
    }
}

/// Doctor axis for FHIR interchange (Spec 020).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FhirInterchangeDoctorStatus {
    pub present: bool,
    pub ready_base: bool,
    pub fhir_version: String,
    pub full_conformance_claimed: bool,
    pub validator_is_authority: bool,
    pub release_ready: bool,
}

impl FhirInterchangeDoctorStatus {
    #[must_use]
    pub fn ready_base() -> Self {
        Self {
            present: true,
            ready_base: true,
            fhir_version: FHIR_R4_VERSION.to_owned(),
            full_conformance_claimed: false,
            validator_is_authority: false,
            release_ready: false,
        }
    }
}

/// One field-level loss recorded during export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FhirFieldLoss {
    pub fhir_path: String,
    pub loss_class: LossClass,
    pub note: String,
}

/// Synthetic loss-aware FHIR export result (Spec 020).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FhirLossAwareExport {
    pub fhir_version: String,
    pub resource_type: String,
    pub exported_json: Value,
    pub field_losses: Vec<FhirFieldLoss>,
    /// Unsupported field paths preserved for provenance / re-export honesty.
    pub unsupported_fields_preserved: Vec<String>,
    pub full_conformance_claimed: bool,
}

/// Paths the closed extractors actually map for READY_BASE structural support.
fn supported_paths_for(resource_type: &str) -> &'static [&'static str] {
    match resource_type {
        "Patient" => &["resourceType", "id", "birthDate", "name"],
        "Observation" => &[
            "resourceType",
            "id",
            "code",
            "valueQuantity",
            "effectiveDateTime",
            "status",
        ],
        "Condition" => &["resourceType", "id", "code", "clinicalStatus"],
        _ => &["resourceType"],
    }
}

/// Export a synthetic FHIR JSON resource with explicit loss documentation.
///
/// Supported top-level keys are copied into `exported_json`. All other object
/// keys are recorded as `LossClass::Lossy` losses and listed in
/// `unsupported_fields_preserved` so information is not silently discarded.
#[must_use]
pub fn loss_aware_export(resource: &Value) -> FhirLossAwareExport {
    let resource_type = resource
        .get("resourceType")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_owned();
    let supported = supported_paths_for(&resource_type);
    let mut exported = Map::new();
    let mut field_losses = Vec::new();
    let mut unsupported_fields_preserved = Vec::new();

    if let Some(obj) = resource.as_object() {
        for (key, value) in obj {
            if supported.contains(&key.as_str()) {
                exported.insert(key.clone(), value.clone());
            } else {
                let path = format!("{resource_type}.{key}");
                unsupported_fields_preserved.push(path.clone());
                field_losses.push(FhirFieldLoss {
                    fhir_path: path,
                    loss_class: LossClass::Lossy,
                    note: "field outside closed READY_BASE extractor subset; preserved in loss inventory"
                        .to_owned(),
                });
            }
        }
    } else {
        field_losses.push(FhirFieldLoss {
            fhir_path: resource_type.clone(),
            loss_class: LossClass::Unknown,
            note: "non-object FHIR payload; export refused structural claim".to_owned(),
        });
    }

    // Always record that clinical interpretation is unsupported on export.
    field_losses.push(FhirFieldLoss {
        fhir_path: format!("{resource_type}.clinical_interpretation"),
        loss_class: LossClass::Unknown,
        note: "clinical interpretation axis unsupported; export does not invent meaning".to_owned(),
    });

    FhirLossAwareExport {
        fhir_version: FHIR_R4_VERSION.to_owned(),
        resource_type,
        exported_json: Value::Object(exported),
        field_losses,
        unsupported_fields_preserved,
        full_conformance_claimed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn matrix_is_honest_ready_base() {
        let m = FhirSupportMatrix::trusted_v1_ready_base();
        assert!(m.is_honest_ready_base());
        assert_eq!(m.status_for("Patient").structural, SupportLevel::Partial);
        assert_eq!(
            m.status_for("MedicationRequest").structural,
            SupportLevel::Unsupported
        );
        assert_eq!(
            m.status_for("Patient").clinical_interpretation,
            SupportLevel::Unsupported
        );
        assert_eq!(
            m.status_for("Observation").terminology,
            SupportLevel::Partial
        );
        assert_eq!(m.status_for("Patient").profile, SupportLevel::Unsupported);
    }

    #[test]
    fn loss_aware_export_preserves_unsupported() {
        let resource = json!({
            "resourceType": "Patient",
            "id": "p1",
            "birthDate": "1990-01-01",
            "gender": "unknown",
            "telecom": [{"system": "phone", "value": "000"}]
        });
        let export = loss_aware_export(&resource);
        assert!(!export.full_conformance_claimed);
        assert!(export.exported_json.get("birthDate").is_some());
        assert!(export.exported_json.get("gender").is_none());
        assert!(
            export
                .unsupported_fields_preserved
                .iter()
                .any(|p| p == "Patient.gender")
        );
        assert!(
            export
                .field_losses
                .iter()
                .any(|l| l.fhir_path == "Patient.gender" && l.loss_class == LossClass::Lossy)
        );
        assert!(!export.unsupported_fields_preserved.is_empty());
    }

    #[test]
    fn matrix_serde_roundtrip() {
        let m = FhirSupportMatrix::trusted_v1_ready_base();
        let v = serde_json::to_value(&m).unwrap();
        let back: FhirSupportMatrix = serde_json::from_value(v).unwrap();
        assert_eq!(back, m);
    }
}
