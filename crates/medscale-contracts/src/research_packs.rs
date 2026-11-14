//! Research Pack contracts (Spec 089, first domain: Clinical Research).
//!
//! A Research Pack is declarative domain semantics: artifact schemas,
//! workflow state machines and evidence rules. It carries no code, grants
//! no capability and installs no Extension. The first Pack ships inside
//! MedScale (first-party, versioned, canonical JSON); installing it in a
//! Project lets Core create and validate domain artifacts, move them
//! through the Pack's workflow, and record evidence assessments whose every
//! axis may be `unknown`. Upgrades apply declared, non-destructive
//! migrations; uninstalling disables the Pack and keeps its data.
//!
//! ```text
//! Research Pack (semantics) != Extension Pack (integration)
//!   != model/runtime Pack (weights)
//! citation exists != citation supports claim != claim is applicable
//! ```

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::objects::{DigestSha256, ObjectHeader, OpaqueId};

pub const RESEARCH_PACK_SCHEMA_VERSION: u32 = 1;
pub const FIELD_TEXT_MAX_CHARS: usize = 4_000;
pub const ARTIFACT_FIELDS_MAX: usize = 64;
pub const WORKFLOW_STATES_MAX: usize = 32;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchDomain {
    ClinicalResearch,
    AiResearch,
    SystematicReview,
    Imaging,
    Omics,
    WetLab,
}

closed_vocabulary!(ResearchDomain, "research domain", {
    ClinicalResearch => "clinical_research",
    AiResearch => "ai_research",
    SystematicReview => "systematic_review",
    Imaging => "imaging",
    Omics => "omics",
    WetLab => "wet_lab",
});

/// A field's type. Closed: no expressions, no code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", deny_unknown_fields)]
pub enum FieldKind {
    Text,
    Integer,
    Boolean,
    /// ISO 8601 calendar date `YYYY-MM-DD`.
    Date,
    Choice {
        values: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldSpec {
    pub name: String,
    pub kind: FieldKind,
    pub required: bool,
}

/// A field value. `Unknown` is an explicit value, never a missing one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum FieldValue {
    Text(String),
    Integer(i64),
    Boolean(bool),
    Date(String),
    Choice(String),
    Unknown,
}

fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let num = |r: std::ops::Range<usize>| s[r].parse::<u32>().ok();
    match (num(0..4), num(5..7), num(8..10)) {
        (Some(_), Some(m), Some(d)) => (1..=12).contains(&m) && (1..=31).contains(&d),
        _ => false,
    }
}

impl FieldSpec {
    /// Whether `value` is acceptable for this field.
    pub fn check(&self, value: &FieldValue) -> Result<(), String> {
        let ok = match (&self.kind, value) {
            (_, FieldValue::Unknown) => true,
            (FieldKind::Text, FieldValue::Text(t)) => {
                t.chars().count() <= FIELD_TEXT_MAX_CHARS && !t.contains('\0')
            }
            (FieldKind::Integer, FieldValue::Integer(_))
            | (FieldKind::Boolean, FieldValue::Boolean(_)) => true,
            (FieldKind::Date, FieldValue::Date(d)) => valid_date(d),
            (FieldKind::Choice { values }, FieldValue::Choice(c)) => values.contains(c),
            _ => false,
        };
        if ok {
            Ok(())
        } else {
            Err(format!("field {} has a value of the wrong kind", self.name))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchWorkflowDescriptor {
    pub workflow_id: String,
    pub states: Vec<String>,
    pub initial: String,
    /// Allowed `(from, to)` moves.
    pub transitions: Vec<(String, String)>,
}

impl ResearchWorkflowDescriptor {
    #[must_use]
    pub fn allows(&self, from: &str, to: &str) -> bool {
        self.transitions.iter().any(|(f, t)| f == from && t == to)
    }

    pub fn validate(&self) -> Result<(), String> {
        let states: BTreeSet<&str> = self.states.iter().map(String::as_str).collect();
        if states.len() != self.states.len()
            || states.is_empty()
            || states.len() > WORKFLOW_STATES_MAX
        {
            return Err(format!("workflow {} has invalid states", self.workflow_id));
        }
        if !states.contains(self.initial.as_str())
            || !self
                .transitions
                .iter()
                .all(|(f, t)| states.contains(f.as_str()) && states.contains(t.as_str()))
        {
            return Err(format!(
                "workflow {} names unknown states",
                self.workflow_id
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchArtifactSchema {
    pub type_id: String,
    pub title: String,
    pub fields: Vec<FieldSpec>,
    pub workflow_id: String,
}

/// A declared, non-destructive migration step between Pack versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "step", deny_unknown_fields)]
pub enum ResearchPackMigration {
    /// Adds an optional field; existing artifacts get `default`.
    AddField {
        type_id: String,
        field: FieldSpec,
        default: FieldValue,
    },
    /// Adds a workflow state and transitions; existing states are kept.
    AddWorkflowState {
        workflow_id: String,
        state: String,
        transitions: Vec<(String, String)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchPackManifest {
    pub pack_id: String,
    pub version: u32,
    pub title: String,
    pub domain: ResearchDomain,
    pub schemas: Vec<ResearchArtifactSchema>,
    pub workflows: Vec<ResearchWorkflowDescriptor>,
    /// Steps from `version - 1` to `version` (empty for version 1).
    pub migrations: Vec<ResearchPackMigration>,
}

impl ResearchPackManifest {
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    #[must_use]
    pub fn digest(&self) -> DigestSha256 {
        DigestSha256::of(&self.canonical_bytes())
    }

    #[must_use]
    pub fn schema(&self, type_id: &str) -> Option<&ResearchArtifactSchema> {
        self.schemas.iter().find(|s| s.type_id == type_id)
    }

    #[must_use]
    pub fn workflow(&self, id: &str) -> Option<&ResearchWorkflowDescriptor> {
        self.workflows.iter().find(|w| w.workflow_id == id)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version == 0 {
            return Err("pack versions start at 1".to_owned());
        }
        for w in &self.workflows {
            w.validate()?;
        }
        let mut types = BTreeSet::new();
        for s in &self.schemas {
            if !types.insert(s.type_id.as_str()) {
                return Err(format!("duplicate artifact type {}", s.type_id));
            }
            if self.workflow(&s.workflow_id).is_none() {
                return Err(format!("{} names an unknown workflow", s.type_id));
            }
            let names: BTreeSet<&str> = s.fields.iter().map(|f| f.name.as_str()).collect();
            if names.len() != s.fields.len() || s.fields.len() > ARTIFACT_FIELDS_MAX {
                return Err(format!("{} has duplicate or too many fields", s.type_id));
            }
        }
        Ok(())
    }

    /// Validates an artifact's fields against its schema.
    pub fn check_fields(
        &self,
        type_id: &str,
        fields: &BTreeMap<String, FieldValue>,
    ) -> Result<(), String> {
        let schema = self
            .schema(type_id)
            .ok_or_else(|| format!("unknown artifact type {type_id}"))?;
        for (name, value) in fields {
            let spec = schema
                .fields
                .iter()
                .find(|f| &f.name == name)
                .ok_or_else(|| format!("unknown field {name}"))?;
            spec.check(value)?;
        }
        for spec in &schema.fields {
            if spec.required && !fields.contains_key(&spec.name) {
                return Err(format!("missing required field {}", spec.name));
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------
// Evidence assessment: every axis explicit, `unknown` allowed.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tri {
    Yes,
    No,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportRelation {
    Supports,
    Contradicts,
    Neutral,
    Unknown,
}

/// GRADE-style certainty, or unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceQuality {
    High,
    Moderate,
    Low,
    VeryLow,
    Unknown,
}

/// How one citation bears on one claim. Nothing is inferred: each axis is
/// what a reviewer recorded, and `unknown` stays `unknown`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceAssessment {
    pub claim: String,
    /// A citation identifier as given (DOI, PMID, guideline id).
    pub citation: String,
    pub citation_exists: Tri,
    pub relation: SupportRelation,
    pub applicable_to_population: Tri,
    /// Publication or guideline year, when known.
    pub year: Option<u16>,
    pub jurisdiction: Option<String>,
    pub guideline_version: Option<String>,
    pub retracted: Tri,
    pub quality: EvidenceQuality,
    pub trial_criteria_met: Tri,
}

impl EvidenceAssessment {
    pub fn validate(&self) -> Result<(), String> {
        if self.claim.trim().is_empty() || self.citation.trim().is_empty() {
            return Err("an assessment names its claim and citation".to_owned());
        }
        if self.citation_exists == Tri::No && self.relation != SupportRelation::Unknown {
            return Err("a citation that does not exist cannot support or contradict".to_owned());
        }
        if self.retracted == Tri::Yes && self.relation == SupportRelation::Supports {
            return Err("a retracted citation is not recorded as support".to_owned());
        }
        Ok(())
    }

    /// Conservative summary: `supported` only when the citation exists, is
    /// not retracted, supports the claim and applies; `unknown` otherwise
    /// unless it is contradicted.
    #[must_use]
    pub fn verdict(&self) -> &'static str {
        match (
            self.citation_exists,
            self.retracted,
            self.relation,
            self.applicable_to_population,
        ) {
            (Tri::Yes, Tri::No, SupportRelation::Supports, Tri::Yes) => "supported",
            (Tri::Yes, _, SupportRelation::Contradicts, _) => "contradicted",
            _ => "unknown",
        }
    }
}

// ---------------------------------------------------------------------
// Durable objects.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackInstallState {
    Enabled,
    /// Uninstalled: nothing new is created or moved; data is kept.
    Disabled,
}

closed_vocabulary!(PackInstallState, "pack install state", {
    Enabled => "enabled",
    Disabled => "disabled",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchPackInstall {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub pack_id: String,
    pub version: u32,
    pub manifest_digest: DigestSha256,
    pub state: PackInstallState,
    pub revision: u64,
}

/// One domain artifact of a Pack type in a Project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchArtifact {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub pack_id: String,
    pub pack_version: u32,
    pub type_id: String,
    pub fields: BTreeMap<String, FieldValue>,
    pub workflow_state: String,
    pub assessments: Vec<EvidenceAssessment>,
    pub revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackAction {
    Install,
    Upgrade,
    Uninstall,
    Create,
    Update,
    Transition,
    Assess,
    Migrate,
}

closed_vocabulary!(PackAction, "pack action", {
    Install => "install",
    Upgrade => "upgrade",
    Uninstall => "uninstall",
    Create => "create",
    Update => "update",
    Transition => "transition",
    Assess => "assess",
    Migrate => "migrate",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackReceipt {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub pack_id: String,
    pub action: PackAction,
    pub targets: Vec<OpaqueId>,
    pub from_version: Option<u32>,
    pub to_version: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(name: &str, required: bool) -> FieldSpec {
        FieldSpec {
            name: name.to_owned(),
            kind: FieldKind::Text,
            required,
        }
    }

    #[test]
    fn fields_check_their_kind_and_allow_unknown() {
        let date = FieldSpec {
            name: "start".to_owned(),
            kind: FieldKind::Date,
            required: true,
        };
        date.check(&FieldValue::Date("2026-09-26".to_owned()))
            .unwrap();
        assert!(
            date.check(&FieldValue::Date("2026-13-01".to_owned()))
                .is_err()
        );
        assert!(
            date.check(&FieldValue::Text("tomorrow".to_owned()))
                .is_err()
        );
        date.check(&FieldValue::Unknown).unwrap();
        let choice = FieldSpec {
            name: "phase".to_owned(),
            kind: FieldKind::Choice {
                values: vec!["1".to_owned(), "2".to_owned()],
            },
            required: false,
        };
        assert!(choice.check(&FieldValue::Choice("4".to_owned())).is_err());
        assert!(
            text("x", false)
                .check(&FieldValue::Text("a\0b".to_owned()))
                .is_err()
        );
    }

    #[test]
    fn evidence_axes_stay_explicit() {
        let mut a = EvidenceAssessment {
            claim: "Statin lowers LDL".to_owned(),
            citation: "PMID:0000000".to_owned(),
            citation_exists: Tri::Yes,
            relation: SupportRelation::Supports,
            applicable_to_population: Tri::Unknown,
            year: Some(2020),
            jurisdiction: None,
            guideline_version: None,
            retracted: Tri::No,
            quality: EvidenceQuality::Moderate,
            trial_criteria_met: Tri::Unknown,
        };
        a.validate().unwrap();
        assert_eq!(
            a.verdict(),
            "unknown",
            "applicability unknown is not support"
        );
        a.applicable_to_population = Tri::Yes;
        assert_eq!(a.verdict(), "supported");
        a.retracted = Tri::Yes;
        assert!(a.validate().is_err());
        a.retracted = Tri::Unknown;
        assert_eq!(a.verdict(), "unknown");
        a.citation_exists = Tri::No;
        assert!(a.validate().is_err(), "a missing citation supports nothing");
        a.relation = SupportRelation::Unknown;
        a.validate().unwrap();
    }

    #[test]
    fn workflows_name_known_states() {
        let mut w = ResearchWorkflowDescriptor {
            workflow_id: "protocol".to_owned(),
            states: vec!["draft".to_owned(), "approved".to_owned()],
            initial: "draft".to_owned(),
            transitions: vec![("draft".to_owned(), "approved".to_owned())],
        };
        w.validate().unwrap();
        assert!(w.allows("draft", "approved") && !w.allows("approved", "draft"));
        w.transitions
            .push(("draft".to_owned(), "deleted".to_owned()));
        assert!(w.validate().is_err());
    }
}

/// One Research Pack act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "act", deny_unknown_fields)]
pub enum PackActRequest {
    Install {
        project_id: OpaqueId,
        pack_id: String,
    },
    Upgrade {
        project_id: OpaqueId,
        pack_id: String,
    },
    Uninstall {
        project_id: OpaqueId,
        pack_id: String,
    },
    Create {
        project_id: OpaqueId,
        pack_id: String,
        type_id: String,
        fields: BTreeMap<String, FieldValue>,
    },
    Update {
        artifact_id: OpaqueId,
        expected_revision: u64,
        fields: BTreeMap<String, FieldValue>,
    },
    Transition {
        artifact_id: OpaqueId,
        expected_revision: u64,
        to: String,
    },
    Assess {
        artifact_id: OpaqueId,
        expected_revision: u64,
        assessment: EvidenceAssessment,
    },
}

/// What an act produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackActResult {
    pub install: Option<ResearchPackInstall>,
    pub artifact: Option<ResearchArtifact>,
}

// ---------------------------------------------------------------------
// Built-in first-party Pack: Clinical Research.
// ---------------------------------------------------------------------

/// Identifier of the first-party Clinical Research Pack.
pub const CLINICAL_RESEARCH_PACK_ID: &str = "medscale.clinical-research";
/// Highest version of it this build ships.
pub const CLINICAL_RESEARCH_PACK_LATEST: u32 = 2;

fn field(name: &str, kind: FieldKind, required: bool) -> FieldSpec {
    FieldSpec {
        name: name.to_owned(),
        kind,
        required,
    }
}

fn states(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| (*s).to_owned()).collect()
}

fn moves(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect()
}

/// The first-party Clinical Research Pack at `version` (1 or 2).
#[must_use]
pub fn clinical_research_pack(version: u32) -> Option<ResearchPackManifest> {
    if version == 0 || version > CLINICAL_RESEARCH_PACK_LATEST {
        return None;
    }
    let choice = |values: &[&str]| FieldKind::Choice {
        values: values.iter().map(|v| (*v).to_owned()).collect(),
    };
    let mut protocol_fields = vec![
        field("title", FieldKind::Text, true),
        field(
            "phase",
            choice(&["1", "2", "3", "4", "observational"]),
            true,
        ),
        field("primary_outcome", FieldKind::Text, true),
        field("start_date", FieldKind::Date, false),
    ];
    let mut protocol_states = states(&["draft", "submitted", "approved", "closed"]);
    let mut protocol_moves = moves(&[
        ("draft", "submitted"),
        ("submitted", "draft"),
        ("submitted", "approved"),
        ("approved", "closed"),
    ]);
    let mut migrations = Vec::new();
    if version >= 2 {
        let registry = field("registry_id", FieldKind::Text, false);
        protocol_fields.push(registry.clone());
        protocol_states.push("suspended".to_owned());
        protocol_moves.extend(moves(&[
            ("approved", "suspended"),
            ("suspended", "approved"),
        ]));
        migrations = vec![
            ResearchPackMigration::AddField {
                type_id: "study_protocol".to_owned(),
                field: registry,
                default: FieldValue::Unknown,
            },
            ResearchPackMigration::AddWorkflowState {
                workflow_id: "protocol_review".to_owned(),
                state: "suspended".to_owned(),
                transitions: moves(&[("approved", "suspended"), ("suspended", "approved")]),
            },
        ];
    }
    Some(ResearchPackManifest {
        pack_id: CLINICAL_RESEARCH_PACK_ID.to_owned(),
        version,
        title: "Clinical Research".to_owned(),
        domain: ResearchDomain::ClinicalResearch,
        schemas: vec![
            ResearchArtifactSchema {
                type_id: "study_protocol".to_owned(),
                title: "Study protocol".to_owned(),
                fields: protocol_fields,
                workflow_id: "protocol_review".to_owned(),
            },
            ResearchArtifactSchema {
                type_id: "adverse_event".to_owned(),
                title: "Adverse event report".to_owned(),
                fields: vec![
                    field("description", FieldKind::Text, true),
                    field("onset_date", FieldKind::Date, true),
                    field("seriousness", choice(&["non_serious", "serious"]), true),
                    field(
                        "causality",
                        choice(&["unrelated", "possible", "probable", "definite"]),
                        false,
                    ),
                    field("expected", FieldKind::Boolean, false),
                ],
                workflow_id: "event_review".to_owned(),
            },
            ResearchArtifactSchema {
                type_id: "evidence_claim".to_owned(),
                title: "Evidence claim".to_owned(),
                fields: vec![field("claim", FieldKind::Text, true)],
                workflow_id: "claim_review".to_owned(),
            },
        ],
        workflows: vec![
            ResearchWorkflowDescriptor {
                workflow_id: "protocol_review".to_owned(),
                states: protocol_states,
                initial: "draft".to_owned(),
                transitions: protocol_moves,
            },
            ResearchWorkflowDescriptor {
                workflow_id: "event_review".to_owned(),
                states: states(&["reported", "assessed", "closed"]),
                initial: "reported".to_owned(),
                transitions: moves(&[("reported", "assessed"), ("assessed", "closed")]),
            },
            ResearchWorkflowDescriptor {
                workflow_id: "claim_review".to_owned(),
                states: states(&["proposed", "reviewed"]),
                initial: "proposed".to_owned(),
                transitions: moves(&[("proposed", "reviewed")]),
            },
        ],
        migrations,
    })
}

#[cfg(test)]
mod pack_tests {
    use super::*;

    #[test]
    fn built_in_pack_versions_validate_and_migrate_forward_only() {
        let v1 = clinical_research_pack(1).unwrap();
        let v2 = clinical_research_pack(2).unwrap();
        v1.validate().unwrap();
        v2.validate().unwrap();
        assert!(clinical_research_pack(3).is_none());
        assert!(v1.migrations.is_empty());
        assert_ne!(v1.digest(), v2.digest());
        // Every v1 field and state survives in v2 (non-destructive).
        for s in &v1.schemas {
            let s2 = v2.schema(&s.type_id).unwrap();
            assert!(s.fields.iter().all(|f| s2.fields.contains(f)));
        }
        for w in &v1.workflows {
            let w2 = v2.workflow(&w.workflow_id).unwrap();
            assert!(w.states.iter().all(|st| w2.states.contains(st)));
        }
        let mut fields = BTreeMap::new();
        fields.insert("title".to_owned(), FieldValue::Text("LDL".to_owned()));
        fields.insert("phase".to_owned(), FieldValue::Choice("2".to_owned()));
        fields.insert("primary_outcome".to_owned(), FieldValue::Unknown);
        v1.check_fields("study_protocol", &fields).unwrap();
        fields.insert("registry_id".to_owned(), FieldValue::Unknown);
        assert!(v1.check_fields("study_protocol", &fields).is_err());
        v2.check_fields("study_protocol", &fields).unwrap();
        fields.remove("title");
        assert!(v2.check_fields("study_protocol", &fields).is_err());
    }
}
