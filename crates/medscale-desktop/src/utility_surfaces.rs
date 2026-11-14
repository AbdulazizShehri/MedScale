//! Audit / Exports / Settings / Integrations presentation boundary for Spec 064.
//!
//! These utility surfaces expose existing doctor, FHIR, disclosure, and broker
//! honesty. They do not open storage/network clients or upgrade READY_BASE into
//! release, privacy, conformance, or partner-integration authority.

use medscale_contracts::doctor::DoctorReport;
use medscale_contracts::fhir::{ResourceSupportStatus, SupportLevel};
use medscale_contracts::objects::{DigestSha256, OpaqueId};
use medscale_contracts::workflow::DisclosureRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtilityRowVm {
    pub label: String,
    pub value: String,
    pub detail: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRowVm {
    pub record_id: String,
    pub action: String,
    pub subject: String,
    pub detail: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtilitySurfacesVm {
    pub synthetic_only: bool,
    pub audit_boundary: String,
    pub export_boundary: String,
    pub settings_boundary: String,
    pub integrations_boundary: String,
    pub audit_rows: Vec<AuditRowVm>,
    pub export_rows: Vec<UtilityRowVm>,
    pub settings_rows: Vec<UtilityRowVm>,
    pub integration_rows: Vec<UtilityRowVm>,
    pub missing_release_evidence: String,
}

impl UtilitySurfacesVm {
    #[must_use]
    pub fn from_doctor(report: &DoctorReport, disclosures: &[DisclosureRecord]) -> Self {
        let audit_rows = disclosures
            .iter()
            .map(|record| AuditRowVm {
                record_id: record.disclosure_id.as_str().to_owned(),
                action: record.purpose.clone(),
                subject: record
                    .subject_ref
                    .as_ref()
                    .map(|id| id.as_str().to_owned())
                    .unwrap_or_else(|| "No subject reference".to_owned()),
                detail: record.note.clone().unwrap_or_else(|| {
                    "Synthetic disclosure record without additional note".to_owned()
                }),
                status: if record.synthetic_only && !record.release_ready_claimed {
                    "Synthetic · no release claim".to_owned()
                } else {
                    "Review required".to_owned()
                },
            })
            .collect();

        let export_rows = report
            .fhir_support_matrix
            .resources
            .iter()
            .map(export_row)
            .collect::<Vec<_>>();

        let settings_rows = vec![
            UtilityRowVm {
                label: "Runtime locality".to_owned(),
                value: if report.local_only { "Local" } else { "Review" }.to_owned(),
                detail: report.product_runtime_egress.clone(),
                status: if report.local_only {
                    "Local-first".to_owned()
                } else {
                    "Review required".to_owned()
                },
            },
            UtilityRowVm {
                label: "Real PHI".to_owned(),
                value: if report.real_phi_authorized {
                    "Authorized"
                } else {
                    "Not authorized"
                }
                .to_owned(),
                detail: "Synthetic/permitted fixtures only until explicit private-data authorization."
                    .to_owned(),
                status: "Fail closed".to_owned(),
            },
            UtilityRowVm {
                label: "Private-data readiness".to_owned(),
                value: if report.vault_privacy.private_data_ready {
                    "Ready"
                } else {
                    "False"
                }
                .to_owned(),
                detail: format!(
                    "SQLCipher={} · OS residual classes open: {}",
                    report.vault_privacy.sqlcipher_enabled,
                    report.vault_privacy.residual_risk_classes_open.join(", ")
                ),
                status: "Evidence gated".to_owned(),
            },
            UtilityRowVm {
                label: "Release readiness".to_owned(),
                value: if report.release_qualification.release_ready {
                    "Ready"
                } else {
                    "False"
                }
                .to_owned(),
                detail: "Desktop+CLI product-launch implementation remains in progress; external release evidence also remains open."
                    .to_owned(),
                status: "External gates remain".to_owned(),
            },
            UtilityRowVm {
                label: "Accessibility".to_owned(),
                value: if report.accessibility.wcag_conformance_claimed {
                    "WCAG claimed"
                } else {
                    "WCAG not claimed"
                }
                .to_owned(),
                detail: "Native labels/keyboard semantics are present; product assistive-technology qualification remains pending."
                    .to_owned(),
                status: "Qualification pending".to_owned(),
            },
        ];

        let integration_rows = vec![
            UtilityRowVm {
                label: "Network Broker".to_owned(),
                value: if report.network_broker.default_deny {
                    "Default deny"
                } else {
                    "Review"
                }
                .to_owned(),
                detail: format!(
                    "{} configured allowlist entries · live partner authorized={}",
                    report.network_broker.allowlist_entries,
                    report.network_broker.live_partner_authorized
                ),
                status: "Broker-only egress".to_owned(),
            },
            UtilityRowVm {
                label: "FHIR R4".to_owned(),
                value: report.fhir_interchange.fhir_version.clone(),
                detail: "Narrow typed support matrix; no full conformance or clinical interpretation claim."
                    .to_owned(),
                status: "READY_BASE subset".to_owned(),
            },
            UtilityRowVm {
                label: "NPHIES".to_owned(),
                value: if report.controlled_actions.nphies_authorized {
                    "Authorized"
                } else {
                    "External gate"
                }
                .to_owned(),
                detail: "Controlled-action path remains gated pending real workflow/partner authority."
                    .to_owned(),
                status: "Not live".to_owned(),
            },
            UtilityRowVm {
                label: "Online packs".to_owned(),
                value: if report.online_packs.online_download_authorized {
                    "Authorized"
                } else {
                    "Denied"
                }
                .to_owned(),
                detail: "Offline-first; online acquisition remains broker/external-gate controlled."
                    .to_owned(),
                status: "Offline-first".to_owned(),
            },
        ];

        Self {
            synthetic_only: true,
            audit_boundary: "Audit Trail shows admitted synthetic disclosure records in this demo. It is not a replacement for the Core Host canonical action-audit store."
                .to_owned(),
            export_boundary: "Exports surface the trusted FHIR support matrix and loss-awareness posture. No full FHIR conformance, live partner send, or clinical interpretation is implied."
                .to_owned(),
            settings_boundary: "Settings expose doctor truth without turning configuration UI into evidence. PRIVATE_DATA_READY, RELEASE_READY, and WCAG claims remain false unless separately qualified."
                .to_owned(),
            integrations_boundary: "Integrations are status/configuration views over the Network Broker and admitted contracts; no direct Desktop egress client or partner credential is introduced."
                .to_owned(),
            audit_rows,
            export_rows,
            settings_rows,
            integration_rows,
            missing_release_evidence: report
                .release_qualification
                .missing_evidence_classes
                .join(" · "),
        }
    }

    #[must_use]
    pub fn synthetic_demo(report: &DoctorReport) -> Self {
        Self::from_doctor(report, &synthetic_disclosures())
    }
}

fn export_row(resource: &ResourceSupportStatus) -> UtilityRowVm {
    UtilityRowVm {
        label: resource.resource_type.clone(),
        value: format!(
            "Lexical {} · Structural {}",
            support_label(resource.lexical),
            support_label(resource.structural)
        ),
        detail: format!(
            "Provenance {} · Clinical interpretation {}",
            support_label(resource.provenance),
            support_label(resource.clinical_interpretation)
        ),
        status: if resource.clinical_interpretation == SupportLevel::Unsupported {
            "No clinical interpretation".to_owned()
        } else {
            "Review".to_owned()
        },
    }
}

fn support_label(level: SupportLevel) -> &'static str {
    match level {
        SupportLevel::Qualified => "Qualified",
        SupportLevel::Partial => "Partial",
        SupportLevel::Unsupported => "Unsupported",
        SupportLevel::EvidenceOnly => "Evidence only",
    }
}

fn synthetic_disclosures() -> Vec<DisclosureRecord> {
    vec![
        DisclosureRecord {
            disclosure_id: OpaqueId::new("disclosure-synthetic-export-001"),
            purpose: "Synthetic FHIR export".to_owned(),
            scope: "patient-summary".to_owned(),
            subject_ref: Some(OpaqueId::new("synthetic-subject-a")),
            artifact_refs: vec![OpaqueId::new("synthetic-export-artifact-001")],
            export_digest: Some(DigestSha256::of(b"synthetic-fhir-export-001")),
            synthetic_only: true,
            release_ready_claimed: false,
            note: Some(
                "Loss-aware synthetic export disclosure; no live partner transmission.".to_owned(),
            ),
        },
        DisclosureRecord {
            disclosure_id: OpaqueId::new("disclosure-synthetic-review-002"),
            purpose: "Synthetic review packet".to_owned(),
            scope: "workflow-review".to_owned(),
            subject_ref: Some(OpaqueId::new("synthetic-subject-b")),
            artifact_refs: vec![OpaqueId::new("synthetic-review-artifact-002")],
            export_digest: Some(DigestSha256::of(b"synthetic-review-packet-002")),
            synthetic_only: true,
            release_ready_claimed: false,
            note: Some("Local disclosure history only; external delivery not claimed.".to_owned()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_core::{build_doctor_report, privacy_proof_artifact_present};

    #[test]
    fn synthetic_utility_surfaces_preserve_non_claims() {
        let report = build_doctor_report(None, false, privacy_proof_artifact_present());
        let vm = UtilitySurfacesVm::synthetic_demo(&report);
        assert!(vm.synthetic_only);
        assert_eq!(vm.audit_rows.len(), 2);
        assert!(
            vm.export_rows
                .iter()
                .all(|row| row.status.contains("No clinical"))
        );
        assert!(
            vm.settings_rows
                .iter()
                .any(|row| { row.label == "Release readiness" && row.value == "False" })
        );
        assert!(
            vm.settings_rows
                .iter()
                .any(|row| { row.label == "Real PHI" && row.value == "Not authorized" })
        );
    }

    #[test]
    fn integrations_keep_default_deny_and_external_gates_visible() {
        let report = build_doctor_report(None, false, privacy_proof_artifact_present());
        let vm = UtilitySurfacesVm::synthetic_demo(&report);
        assert!(
            vm.integration_rows
                .iter()
                .any(|row| { row.label == "Network Broker" && row.value == "Default deny" })
        );
        assert!(
            vm.integration_rows
                .iter()
                .any(|row| { row.label == "NPHIES" && row.value == "External gate" })
        );
        assert!(
            vm.integration_rows.iter().all(|row| row.label != "MESC"),
            "separate MESC project must not appear as a MedScale integration"
        );
    }
}
