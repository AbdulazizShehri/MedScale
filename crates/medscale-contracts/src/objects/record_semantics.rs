//! Record semantics: missingness, amendments, identity reconciliation (Spec 019 Q06).

use serde::{Deserialize, Serialize};

use super::{MedicalTime, ObjectHeader, OpaqueId};

/// Honest missingness taxonomy — never collapse into a single "null".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingnessKind {
    Unknown,
    ExplicitAbsence,
    NotObserved,
    NotApplicable,
    Withheld,
    Conflict,
}

/// Supersession vs retraction lineage (append-only; never overwrite prior assertion bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmendmentKind {
    Supersession,
    Retraction,
}

/// Append-only amendment / supersession / retraction lineage linking OpaqueIds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmendmentRecord {
    pub header: ObjectHeader,
    pub prior_assertion_id: OpaqueId,
    pub superseding_assertion_id: OpaqueId,
    pub kind: AmendmentKind,
    pub rationale: String,
    pub authorized_by: OpaqueId,
    pub recorded_time: MedicalTime,
}

/// Candidate pair for explicit identity reconciliation (never auto-merge).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityReconciliationCandidate {
    pub subject_a: OpaqueId,
    pub subject_b: OpaqueId,
    pub identifier_system: String,
    pub identifier_value: String,
    pub evidence_refs: Vec<OpaqueId>,
}

/// Unresolved identity candidates awaiting explicit `IdentityMergeDecision`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct IdentityUnresolvedSet {
    pub candidates: Vec<IdentityReconciliationCandidate>,
}

/// Build unresolved identity candidates from identity assertions.
///
/// Subjects already unified by an explicit merge decision are excluded.
/// Matching is by identical (system, value) across distinct subject ids.
pub fn find_unresolved_identity_candidates(
    assertions: &[super::IdentityAssertion],
    merges: &[super::IdentityMergeDecision],
) -> IdentityUnresolvedSet {
    use std::collections::{BTreeMap, BTreeSet};

    let mut merged_away: BTreeSet<String> = BTreeSet::new();
    for m in merges {
        for id in &m.merged_subject_ids {
            merged_away.insert(id.as_str().to_owned());
        }
    }

    let mut by_key: BTreeMap<(String, String), Vec<&super::IdentityAssertion>> = BTreeMap::new();
    for a in assertions {
        if merged_away.contains(a.subject_id.as_str()) {
            continue;
        }
        by_key
            .entry((a.identifier_system.clone(), a.identifier_value.clone()))
            .or_default()
            .push(a);
    }

    let mut candidates = Vec::new();
    for ((system, value), group) in by_key {
        let mut subjects: BTreeSet<String> = BTreeSet::new();
        for a in &group {
            subjects.insert(a.subject_id.as_str().to_owned());
        }
        if subjects.len() < 2 {
            continue;
        }
        let subject_list: Vec<String> = subjects.into_iter().collect();
        for i in 0..subject_list.len() {
            for j in (i + 1)..subject_list.len() {
                let evidence_refs: Vec<OpaqueId> = group
                    .iter()
                    .filter(|a| {
                        a.subject_id.as_str() == subject_list[i]
                            || a.subject_id.as_str() == subject_list[j]
                    })
                    .map(|a| a.header.id.clone())
                    .collect();
                candidates.push(IdentityReconciliationCandidate {
                    subject_a: OpaqueId::new(subject_list[i].clone()),
                    subject_b: OpaqueId::new(subject_list[j].clone()),
                    identifier_system: system.clone(),
                    identifier_value: value.clone(),
                    evidence_refs,
                });
            }
        }
    }
    IdentityUnresolvedSet { candidates }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AUTHORITY_SCHEMA_VERSION;
    use crate::objects::{AuthorityScopeId, IdentityAssertion, ObjectHeader, RealmId};

    fn header(id: &str) -> ObjectHeader {
        ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id: RealmId::new("r"),
            authority_scope_id: AuthorityScopeId::new("s"),
        }
    }

    #[test]
    fn missingness_roundtrip() {
        for kind in [
            MissingnessKind::Unknown,
            MissingnessKind::ExplicitAbsence,
            MissingnessKind::NotObserved,
            MissingnessKind::NotApplicable,
            MissingnessKind::Withheld,
            MissingnessKind::Conflict,
        ] {
            let v = serde_json::to_value(kind).unwrap();
            let back: MissingnessKind = serde_json::from_value(v).unwrap();
            assert_eq!(back, kind);
        }
    }

    #[test]
    fn unresolved_candidates_from_shared_identifier() {
        let a = IdentityAssertion {
            header: header("ia-1"),
            subject_id: OpaqueId::new("subj-a"),
            identifier_system: "mrn".to_owned(),
            identifier_value: "42".to_owned(),
            confidence: None,
            evidence_refs: vec![],
        };
        let b = IdentityAssertion {
            header: header("ia-2"),
            subject_id: OpaqueId::new("subj-b"),
            identifier_system: "mrn".to_owned(),
            identifier_value: "42".to_owned(),
            confidence: None,
            evidence_refs: vec![],
        };
        let set = find_unresolved_identity_candidates(&[a, b], &[]);
        assert_eq!(set.candidates.len(), 1);
        assert_eq!(set.candidates[0].identifier_value, "42");
    }
}
