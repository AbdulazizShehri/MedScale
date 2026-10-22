//! Deterministic fixture UI view-models (V0_UI_INTEGRATION_CONTRACT).
//!
//! These bind CLI/Desktop/mobile shells to typed authority surfaces without inventing
//! final visual design. No canonical DB/key handles.
//! Spec 029: accessible_label supports fixture/CLI a11y honesty (not WCAG conformance).

use serde::{Deserialize, Serialize};

use crate::doctor::DoctorReport;
use crate::presentation::{SubjectBriefV1, SubjectCoverageV1, SubjectTimelineV1};

/// Fixture UI surface identity (not a visual theme).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureUiSurface {
    Doctor,
    Timeline,
    Brief,
    Coverage,
    PrivacyProof,
}

/// Spec 056: shell-agnostic role for a fixture surface (data only, not a WCAG claim).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureUiRole {
    Status,
    Document,
    Log,
}

/// Spec 056: lifecycle state a fixture surface can be announced in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureViewState {
    Ready,
    Loading,
    Empty,
    Error,
    Conflict,
    Recovery,
}

impl FixtureViewState {
    /// Every state in the lifecycle, in canonical order.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::Ready,
            Self::Loading,
            Self::Empty,
            Self::Error,
            Self::Conflict,
            Self::Recovery,
        ]
    }

    /// Canonical operator announcement for a state (exact text is the contract).
    #[must_use]
    pub fn canonical_announcement(&self) -> &'static str {
        match self {
            Self::Ready => "Ready.",
            Self::Loading => "Loading. Please wait.",
            Self::Empty => "No data to show.",
            Self::Error => "An error occurred. No action was taken.",
            Self::Conflict => "Conflicting data needs review. No action was taken.",
            Self::Recovery => "Recovered. Please verify before continuing.",
        }
    }
}

/// Spec 056: keyboard/role/announcement semantics for one fixture view model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureA11ySemantics {
    pub role: FixtureUiRole,
    /// Zero-based keyboard focus order across fixture surfaces.
    pub keyboard_focus_index: u32,
    /// Human-readable keyboard path (e.g. "doctor, 1 of 4").
    pub keyboard_path: String,
    pub state: FixtureViewState,
    /// Must equal `state.canonical_announcement()` when honest.
    pub announcement: String,
}

impl FixtureA11ySemantics {
    #[must_use]
    pub fn for_surface(
        role: FixtureUiRole,
        keyboard_focus_index: u32,
        keyboard_path: String,
        state: FixtureViewState,
    ) -> Self {
        let announcement = state.canonical_announcement().to_owned();
        Self {
            role,
            keyboard_focus_index,
            keyboard_path,
            state,
            announcement,
        }
    }

    /// Honest iff path is non-empty and the announcement is the canonical
    /// text for the declared state.
    #[must_use]
    pub fn is_honest(&self) -> bool {
        !self.keyboard_path.trim().is_empty()
            && self.announcement == self.state.canonical_announcement()
    }
}

impl Default for FixtureA11ySemantics {
    /// Back-compat default for documents written before Spec 056; not honest.
    fn default() -> Self {
        Self {
            role: FixtureUiRole::Status,
            keyboard_focus_index: 0,
            keyboard_path: String::new(),
            state: FixtureViewState::Ready,
            announcement: String::new(),
        }
    }
}

/// Shell-agnostic fixture adapter payload for one screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureUiViewModel {
    pub surface: FixtureUiSurface,
    pub synthetic_only: bool,
    pub real_phi_authorized: bool,
    pub title: String,
    /// Operator / assistive-tech oriented label (fixture honesty; not a WCAG claim).
    pub accessible_label: String,
    /// Spec 056 keyboard/role/announcement semantics (not a WCAG claim).
    #[serde(default)]
    pub semantics: FixtureA11ySemantics,
    pub body_json: serde_json::Value,
}

impl FixtureUiViewModel {
    /// Stable surface labels required for Spec 029 fixture accessibility honesty.
    #[must_use]
    pub fn required_surface_labels() -> &'static [&'static str] {
        &["doctor", "timeline", "brief", "coverage", "privacy_proof"]
    }

    #[must_use]
    pub fn from_doctor(report: &DoctorReport) -> Self {
        Self {
            surface: FixtureUiSurface::Doctor,
            synthetic_only: report.synthetic_only,
            real_phi_authorized: report.real_phi_authorized,
            title: "doctor".to_owned(),
            accessible_label: "MedScale doctor status".to_owned(),
            semantics: FixtureA11ySemantics::for_surface(
                FixtureUiRole::Status,
                0,
                "doctor, 1 of 4".to_owned(),
                FixtureViewState::Ready,
            ),
            body_json: serde_json::to_value(report).unwrap_or(serde_json::Value::Null),
        }
    }

    #[must_use]
    pub fn from_timeline(timeline: &SubjectTimelineV1) -> Self {
        Self {
            surface: FixtureUiSurface::Timeline,
            synthetic_only: true,
            real_phi_authorized: false,
            title: "timeline".to_owned(),
            accessible_label: "MedScale subject timeline".to_owned(),
            semantics: FixtureA11ySemantics::for_surface(
                FixtureUiRole::Document,
                1,
                "timeline, 2 of 4".to_owned(),
                FixtureViewState::Ready,
            ),
            body_json: serde_json::to_value(timeline).unwrap_or(serde_json::Value::Null),
        }
    }

    #[must_use]
    pub fn from_brief(brief: &SubjectBriefV1) -> Self {
        Self {
            surface: FixtureUiSurface::Brief,
            synthetic_only: true,
            real_phi_authorized: false,
            title: "brief".to_owned(),
            accessible_label: "MedScale subject brief".to_owned(),
            semantics: FixtureA11ySemantics::for_surface(
                FixtureUiRole::Document,
                2,
                "brief, 3 of 4".to_owned(),
                FixtureViewState::Ready,
            ),
            body_json: serde_json::to_value(brief).unwrap_or(serde_json::Value::Null),
        }
    }

    #[must_use]
    pub fn from_coverage(coverage: &SubjectCoverageV1) -> Self {
        Self {
            surface: FixtureUiSurface::Coverage,
            synthetic_only: true,
            real_phi_authorized: false,
            title: "coverage".to_owned(),
            accessible_label: "MedScale subject coverage".to_owned(),
            semantics: FixtureA11ySemantics::for_surface(
                FixtureUiRole::Document,
                3,
                "coverage, 4 of 4".to_owned(),
                FixtureViewState::Ready,
            ),
            body_json: serde_json::to_value(coverage).unwrap_or(serde_json::Value::Null),
        }
    }

    /// UI must never claim live PHI while REAL_PHI unauthorized.
    #[must_use]
    pub fn respects_phi_boundary(&self) -> bool {
        !(self.real_phi_authorized && !self.synthetic_only)
            && (self.synthetic_only || !self.real_phi_authorized)
    }

    /// Spec 029: title + accessible_label present and non-empty (honesty, not WCAG).
    #[must_use]
    pub fn has_required_a11y_labels(&self) -> bool {
        !self.title.trim().is_empty() && !self.accessible_label.trim().is_empty()
    }

    /// Spec 056: labels plus honest keyboard/role/announcement semantics.
    #[must_use]
    pub fn has_required_a11y_semantics(&self) -> bool {
        self.has_required_a11y_labels() && self.semantics.is_honest()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::ControlledActionsDoctorStatus;
    use crate::doctor::{
        AccessibilityDoctorStatus, HostAuthorityDoctorStatus, KeyStoreAvailability,
        PrivacyFreshness, RecordSemanticsDoctorStatus, SyncRisk, VaultPrivacyDoctorStatus,
    };
    use crate::fhir::{FhirInterchangeDoctorStatus, FhirSupportMatrix};
    use crate::mobile::MobileDoctorStatus;
    use crate::network::NetworkBrokerDoctorStatus;
    use crate::online_packs::OnlinePacksDoctorStatus;
    use crate::packs::PacksRuntimeDoctorStatus;

    #[test]
    fn doctor_view_model_is_synthetic() {
        let report = DoctorReport {
            product_name: "MedScale".to_owned(),
            version: "0.1.0".to_owned(),
            local_only: true,
            product_runtime_egress: "DEFAULT_DENY".to_owned(),
            synthetic_only: true,
            real_phi_authorized: false,
            vault_root: None,
            vault_open: false,
            sync_risk: SyncRisk::Unknown,
            filesystem_claim_ok: false,
            key_store: KeyStoreAvailability::MemoryMockAvailable,
            privacy_proof_freshness: PrivacyFreshness::Missing,
            desktop_shell: "thin_scaffold_non_webview".to_owned(),
            tauri_admitted: false,
            network_broker: NetworkBrokerDoctorStatus {
                present: true,
                default_deny: true,
                allowlist_entries: 0,
                live_partner_authorized: false,
                http_client: "fixture".to_owned(),
            },
            packs_runtime: PacksRuntimeDoctorStatus {
                present: true,
                offline_only: true,
                admitted_count: 0,
                current_pack_id: None,
                confinement_claim: "policy".to_owned(),
                online_download_authorized: false,
            },
            mobile: MobileDoctorStatus::ready_base(),
            controlled_actions: ControlledActionsDoctorStatus::ready_base(),
            online_packs: OnlinePacksDoctorStatus::ready_base(),
            mesc_artifact: crate::external_artifact::ExternalArtifactDoctorStatus::separate_project(
            ),
            vault_privacy: VaultPrivacyDoctorStatus::spec_023_honest(),
            host_authority: HostAuthorityDoctorStatus::ready_base(),
            record_semantics: RecordSemanticsDoctorStatus::ready_base(),
            fhir_interchange: FhirInterchangeDoctorStatus::ready_base(),
            fhir_support_matrix: FhirSupportMatrix::trusted_v1_ready_base(),
            workflow: crate::workflow::WorkflowDoctorStatus::ready_base(),
            release_qualification: crate::doctor::ReleaseQualificationDoctorStatus::prep_ready_base(
            ),
            accessibility: AccessibilityDoctorStatus::ready_base(),
            evidence_corpus: crate::evidence::EvidenceCorpusDoctorStatus::ready_base(
                "synthetic-lexical",
                "1.0.0",
            ),
            pack_signer: crate::packs::PackSignerDoctorStatus::ready_base(),
            os_sandbox: crate::os_sandbox::OsSandboxDoctorStatus::ready_base(),
            notes: vec![],
        };
        let vm = FixtureUiViewModel::from_doctor(&report);
        assert!(vm.respects_phi_boundary());
        assert!(vm.has_required_a11y_labels());
        assert_eq!(vm.surface, FixtureUiSurface::Doctor);
        assert_eq!(vm.title, "doctor");
        assert!(!vm.accessible_label.is_empty());
    }

    #[test]
    fn required_surface_labels_cover_fixture_surfaces() {
        let labels = FixtureUiViewModel::required_surface_labels();
        assert!(labels.contains(&"doctor"));
        assert!(labels.contains(&"timeline"));
        assert!(labels.contains(&"brief"));
        assert!(labels.contains(&"coverage"));
        assert!(labels.contains(&"privacy_proof"));
    }
}
