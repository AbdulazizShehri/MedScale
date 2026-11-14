//! Operator doctor report and PRIVACY_PROOF evidence types (Spec 006 / 013).

use serde::{Deserialize, Serialize};

use crate::actions::ControlledActionsDoctorStatus;
use crate::external_artifact::ExternalArtifactDoctorStatus;
use crate::fhir::{FhirInterchangeDoctorStatus, FhirSupportMatrix};
use crate::mobile::MobileDoctorStatus;
use crate::network::NetworkBrokerDoctorStatus;
use crate::online_packs::OnlinePacksDoctorStatus;
use crate::os_sandbox::OsSandboxDoctorStatus;
use crate::packs::{PackSignerDoctorStatus, PacksRuntimeDoctorStatus};
use crate::workflow::WorkflowDoctorStatus;

/// Release-qualification prep posture (Specs 022/027/029/032 / Trusted V1 Q05 remnants).
///
/// Always reports `release_ready = false` until a separate qualification package
/// and external gates close. Lists missing evidence classes without claiming pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseQualificationDoctorStatus {
    pub present: bool,
    /// Spec 022 prep paths/evidence/locked CI documented and wired.
    pub prep_ready_base: bool,
    /// Never true for Specs 022/027/029/032 READY_BASE.
    pub release_ready: bool,
    pub locked_builds: bool,
    pub immutable_ci_action_pins: bool,
    pub cargo_lock_committed: bool,
    pub windows_linux_ci_baseline: bool,
    /// Spec 029: CI rust matrix includes `macos-latest` (baseline build/test only).
    pub macos_ci_present: bool,
    /// Product macOS PLATFORM_QUALIFIED — still false after Spec 029 CI expansion.
    pub macos_qualified: bool,
    /// Deferred mobile readiness axis; false is honest and does not block Trusted V1 Desktop/CLI.
    pub mobile_release_qualified: bool,
    /// Live GitHub ruleset evidence confirms protected main with required checks.
    pub branch_protection_configured: bool,
    /// Spec 027: deterministic perf harness evidence path present (budgets not claimed).
    pub perf_harness_present: bool,
    /// Spec 027: cargo-metadata SBOM scaffold path present (not full release SBOM).
    pub sbom_scaffold_present: bool,
    /// Spec 046: SBOM scaffold binds Cargo.lock digest (still not full release SBOM).
    pub sbom_lock_bound: bool,
    /// Spec 047: release dry-run + manifest↔SBOM↔checksum cross-verifier present.
    pub release_dry_run_verifier_present: bool,
    /// Spec 048: vault-level migration interrupt + backup/restore recovery READY_BASE.
    pub migration_recovery_ready_base: bool,
    /// Spec 049: package upgrade/rollback dry-run scaffold present.
    pub package_upgrade_rollback_scaffold_present: bool,
    /// Spec 058: deterministic unsigned portable release package qualified across required CI OSes.
    #[serde(default)]
    pub portable_release_package_qualified: bool,
    /// Spec 058: real package install/upgrade/rollback lifecycle proof completed.
    #[serde(default)]
    pub package_lifecycle_qualified: bool,
    /// Spec 050: host-bound perf measurement operator path present (budgets not claimed).
    pub host_perf_measurement_path_present: bool,
    /// Spec 057: cold-launch + idle-memory runtime measurement coverage path is wired across CI OSes.
    #[serde(default)]
    pub runtime_perf_measurement_coverage_present: bool,
    /// Spec 051: REQUIRED_CHECKS owner packet synced to live CI job names.
    pub required_checks_packet_synced: bool,
    /// Spec 054: deterministic CycloneDX release SBOM qualified (READY_BASE;
    /// signing/provenance/installer decisions remain external).
    pub release_sbom_qualified: bool,
    /// Spec 032: NOTICE/third-party inventory artifact present (not a license decision).
    pub notice_inventory_present: bool,
    /// Founder-selected public SPDX for MedScale crates; Apache-2.0 decision is recorded.
    pub rights_license_decision: bool,
    /// Spec 059: bounded final audit found no unresolved repository-owned material release finding.
    #[serde(default)]
    pub material_findings_clearance: bool,
    pub missing_evidence_classes: Vec<String>,
}

impl ReleaseQualificationDoctorStatus {
    /// Specs 022+027+029+032+054+057+058+059 release qualification state:
    /// repository-owned prep/material findings are closed; external release gates remain.
    #[must_use]
    pub fn prep_ready_base() -> Self {
        Self {
            present: true,
            prep_ready_base: true,
            release_ready: false,
            locked_builds: true,
            immutable_ci_action_pins: true,
            cargo_lock_committed: true,
            windows_linux_ci_baseline: true,
            macos_ci_present: true,
            macos_qualified: false,
            mobile_release_qualified: false,
            branch_protection_configured: true,
            perf_harness_present: true,
            sbom_scaffold_present: true,
            sbom_lock_bound: true,
            release_dry_run_verifier_present: true,
            migration_recovery_ready_base: true,
            package_upgrade_rollback_scaffold_present: true,
            portable_release_package_qualified: true,
            package_lifecycle_qualified: true,
            host_perf_measurement_path_present: true,
            runtime_perf_measurement_coverage_present: true,
            required_checks_packet_synced: true,
            release_sbom_qualified: true,
            notice_inventory_present: true,
            rights_license_decision: true,
            material_findings_clearance: true,
            missing_evidence_classes: vec![
                "macos_platform_product_qualification".to_owned(),
                "release_sbom_signing_provenance".to_owned(),
                "perf_budgets_attained_on_qualified_hardware".to_owned(),
                "checksums_provenance_signing_verification".to_owned(),
                "wcag_final_v0_ui_accessibility_qualification".to_owned(),
            ],
        }
    }

    #[must_use]
    pub fn is_honest_prep(&self) -> bool {
        self.present
            && self.prep_ready_base
            && !self.release_ready
            && self.macos_ci_present
            && !self.macos_qualified
            && !self.mobile_release_qualified
            && self.branch_protection_configured
            && self.perf_harness_present
            && self.sbom_scaffold_present
            && self.sbom_lock_bound
            && self.release_dry_run_verifier_present
            && self.migration_recovery_ready_base
            && self.package_upgrade_rollback_scaffold_present
            && self.portable_release_package_qualified
            && self.package_lifecycle_qualified
            && self.host_perf_measurement_path_present
            && self.runtime_perf_measurement_coverage_present
            && self.required_checks_packet_synced
            && self.release_sbom_qualified
            && self.notice_inventory_present
            && self.rights_license_decision
            && self.material_findings_clearance
            && self
                .missing_evidence_classes
                .contains(&"macos_platform_product_qualification".to_owned())
            && !self
                .missing_evidence_classes
                .contains(&"repo_branch_protection_required_checks".to_owned())
            && !self
                .missing_evidence_classes
                .contains(&"public_source_license_choice".to_owned())
            && self
                .missing_evidence_classes
                .contains(&"release_sbom_signing_provenance".to_owned())
            && self
                .missing_evidence_classes
                .contains(&"checksums_provenance_signing_verification".to_owned())
            && !self
                .missing_evidence_classes
                .contains(&"reproducible_release_package_contents".to_owned())
            && !self
                .missing_evidence_classes
                .contains(&"release_package_upgrade_rollback_proof".to_owned())
            && self
                .missing_evidence_classes
                .contains(&"perf_budgets_attained_on_qualified_hardware".to_owned())
            && !self
                .missing_evidence_classes
                .contains(&"release_bar_migration_recovery_proof".to_owned())
            && !self.missing_evidence_classes.is_empty()
    }
}

/// Fixture/CLI accessibility honesty (Spec 029). Not a WCAG audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessibilityDoctorStatus {
    pub present: bool,
    /// Fixture/CLI keyboard-path and disclosure clarity checks exist (READY_BASE).
    pub ready_base: bool,
    /// FixtureUiViewModel surfaces expose stable accessible labels.
    pub fixture_cli_labels_checked: bool,
    /// Spec 056: fixture surfaces expose honest role/keyboard/announcement semantics.
    #[serde(default)]
    pub fixture_state_semantics_checked: bool,
    /// CLI help exposes named subcommands/flags for keyboard/operator navigation.
    pub cli_keyboard_path_documented: bool,
    /// Disclosure / doctor honesty fields remain explicit (synthetic_only, non-claims).
    pub disclosure_clarity_checked: bool,
    /// Never true in Spec 029.
    pub wcag_conformance_claimed: bool,
    /// Final v0 visual UI not present in-repo for this unit.
    pub final_v0_ui_present: bool,
    /// Never true in Spec 029.
    pub release_ready: bool,
    pub limitations: Vec<String>,
}

impl AccessibilityDoctorStatus {
    #[must_use]
    pub fn ready_base() -> Self {
        Self {
            present: true,
            ready_base: true,
            fixture_cli_labels_checked: true,
            fixture_state_semantics_checked: true,
            cli_keyboard_path_documented: true,
            disclosure_clarity_checked: true,
            wcag_conformance_claimed: false,
            final_v0_ui_present: false,
            release_ready: false,
            limitations: vec![
                "No full WCAG 2.x audit or assistive-technology product qualification".to_owned(),
                "No final v0 UI artifact; FINAL_V0_UI_ARTIFACT remains external".to_owned(),
                "READY_BASE covers fixture/CLI label, keyboard/role/announcement semantics, and disclosure honesty only".to_owned(),
            ],
        }
    }

    /// Current native final-v0 implementation posture (Spec 060).
    ///
    /// Presence of the visual implementation is deliberately separate from
    /// WCAG / assistive-technology qualification and release readiness.
    #[must_use]
    pub fn native_final_v0() -> Self {
        Self {
            present: true,
            ready_base: true,
            fixture_cli_labels_checked: true,
            fixture_state_semantics_checked: true,
            cli_keyboard_path_documented: true,
            disclosure_clarity_checked: true,
            wcag_conformance_claimed: false,
            final_v0_ui_present: true,
            release_ready: false,
            limitations: vec![
                "Native final-v0 implementation is present; WCAG 2.x and assistive-technology product qualification remain pending".to_owned(),
                "Spec 060 establishes Slint accessibility semantics and keyboard paths but does not itself claim conformance".to_owned(),
            ],
        }
    }

    #[must_use]
    pub fn is_honest_ready_base(&self) -> bool {
        self.present
            && self.ready_base
            && self.fixture_cli_labels_checked
            && self.fixture_state_semantics_checked
            && self.cli_keyboard_path_documented
            && self.disclosure_clarity_checked
            && !self.wcag_conformance_claimed
            && !self.release_ready
            && !self.limitations.is_empty()
    }
}

/// Sync / remote filesystem risk assessment for a vault path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncRisk {
    Clear,
    RefusedMarker,
    Unknown,
}

/// Key-store availability for wrapped DEK material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyStoreAvailability {
    MemoryMockAvailable,
    /// Platform credential store probed successfully (Spec 028).
    OsStoreAvailable,
    /// Historical: OS store deferred before Spec 028 READY_BASE.
    OsStoreDeferred,
    Unavailable,
}

/// Privacy evidence freshness relative to repository evidence artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyFreshness {
    Present,
    Missing,
    Stale,
}

/// WebView cache scan applicability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebViewScanStatus {
    NotApplicable,
    Pass,
    Fail,
    Deferred,
}

/// Best-effort OS residual file existence (Spec 032 probes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsResidualFileProbe {
    Detected,
    NotFound,
    NotReadable,
    NotApplicable,
}

/// Honesty class for residual privacy surfaces (Spec 043).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeHonestyClass {
    ConfigurationDetected,
    ProtectionMeasured,
    NotMeasurableOnHost,
    OwnerOrOsPolicyRequired,
}

/// Vault privacy posture (Specs 017/023/028/032/043). Never claims secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultPrivacyDoctorStatus {
    pub present: bool,
    pub private_data_ready: bool,
    pub sealed_at_close: bool,
    pub open_work_plaintext_risk: bool,
    /// Spec 023: EncryptedVault open-work pages encrypted via SQLCipher.
    pub open_work_page_encrypted: bool,
    pub work_wipe_on_close: bool,
    pub sqlcipher_enabled: bool,
    /// Spec 028: OS credential store put/get/delete probe succeeded on this host.
    pub os_keyring_available: bool,
    /// Spec 028: runtime prefers/uses OsKeyStore when available (not forced to Memory).
    pub os_keyring_used: bool,
    /// Spec 032: OS residual privacy probes wired (existence/scan only).
    pub probes_present: bool,
    /// Spec 043: swap/snapshot/core-dump honesty classification wired.
    pub swap_snapshot_honesty_present: bool,
    /// Spec 032/043: residual risk classes still OPEN.
    pub residual_risk_classes_open: Vec<String>,
    /// Spec 032: best-effort pagefile (or OS swap-class) existence probe.
    pub pagefile_existence: OsResidualFileProbe,
    /// Spec 032: best-effort hibernate/sleepimage existence probe.
    pub hibernate_file_existence: OsResidualFileProbe,
    /// Spec 043: distinct swap existence (may equal pagefile-class on some OS).
    pub swap_existence: OsResidualFileProbe,
    /// Spec 043: best-effort filesystem/volume snapshot existence.
    pub snapshot_existence: OsResidualFileProbe,
    /// Spec 043: core/crash-dump configuration surface existence.
    pub core_dump_config_existence: OsResidualFileProbe,
    /// Spec 043: existence honesty for pagefile/swap-class.
    pub pagefile_existence_honesty: ProbeHonestyClass,
    /// Spec 043: protection honesty (never measured for residuals in this unit).
    pub pagefile_protection_honesty: ProbeHonestyClass,
    pub swap_existence_honesty: ProbeHonestyClass,
    pub swap_protection_honesty: ProbeHonestyClass,
    pub snapshot_existence_honesty: ProbeHonestyClass,
    pub snapshot_protection_honesty: ProbeHonestyClass,
    pub core_dump_existence_honesty: ProbeHonestyClass,
    pub core_dump_protection_honesty: ProbeHonestyClass,
    /// Spec 032: vault temp/work leftover scan helper available.
    pub vault_leftover_scan_available: bool,
    /// Spec 032: crash sidecar detection (Spec 017 wipe path) available.
    pub crash_sidecar_detect_available: bool,
    /// Spec 035: EncryptedVault persists/reloads authority graph (SQLCipher meta).
    pub encrypted_authority_sync_qualified: bool,
}

impl VaultPrivacyDoctorStatus {
    fn residual_classes_open() -> Vec<String> {
        vec![
            "swap".to_owned(),
            "hibernate".to_owned(),
            "snapshot".to_owned(),
            "pagefile".to_owned(),
            "core_dump".to_owned(),
        ]
    }

    fn honesty_unmeasured() -> (
        ProbeHonestyClass,
        ProbeHonestyClass,
        ProbeHonestyClass,
        ProbeHonestyClass,
        ProbeHonestyClass,
        ProbeHonestyClass,
        ProbeHonestyClass,
        ProbeHonestyClass,
    ) {
        (
            ProbeHonestyClass::NotMeasurableOnHost,
            ProbeHonestyClass::OwnerOrOsPolicyRequired,
            ProbeHonestyClass::NotMeasurableOnHost,
            ProbeHonestyClass::OwnerOrOsPolicyRequired,
            ProbeHonestyClass::NotMeasurableOnHost,
            ProbeHonestyClass::OwnerOrOsPolicyRequired,
            ProbeHonestyClass::NotMeasurableOnHost,
            ProbeHonestyClass::OwnerOrOsPolicyRequired,
        )
    }

    /// Spec 017 posture before SQLCipher open-work (historical).
    #[must_use]
    pub fn spec_017_honest() -> Self {
        let (
            pagefile_existence_honesty,
            pagefile_protection_honesty,
            swap_existence_honesty,
            swap_protection_honesty,
            snapshot_existence_honesty,
            snapshot_protection_honesty,
            core_dump_existence_honesty,
            core_dump_protection_honesty,
        ) = Self::honesty_unmeasured();
        Self {
            present: true,
            private_data_ready: false,
            sealed_at_close: true,
            open_work_plaintext_risk: true,
            open_work_page_encrypted: false,
            work_wipe_on_close: true,
            sqlcipher_enabled: false,
            os_keyring_available: false,
            os_keyring_used: false,
            probes_present: false,
            swap_snapshot_honesty_present: false,
            residual_risk_classes_open: Self::residual_classes_open(),
            pagefile_existence: OsResidualFileProbe::NotApplicable,
            hibernate_file_existence: OsResidualFileProbe::NotApplicable,
            swap_existence: OsResidualFileProbe::NotApplicable,
            snapshot_existence: OsResidualFileProbe::NotApplicable,
            core_dump_config_existence: OsResidualFileProbe::NotApplicable,
            pagefile_existence_honesty,
            pagefile_protection_honesty,
            swap_existence_honesty,
            swap_protection_honesty,
            snapshot_existence_honesty,
            snapshot_protection_honesty,
            core_dump_existence_honesty,
            core_dump_protection_honesty,
            vault_leftover_scan_available: true,
            crash_sidecar_detect_available: true,
            encrypted_authority_sync_qualified: false,
        }
    }

    /// Spec 023 READY_BASE: page-encrypted open work; PRIVATE_DATA_READY still false.
    #[must_use]
    pub fn spec_023_honest() -> Self {
        let (
            pagefile_existence_honesty,
            pagefile_protection_honesty,
            swap_existence_honesty,
            swap_protection_honesty,
            snapshot_existence_honesty,
            snapshot_protection_honesty,
            core_dump_existence_honesty,
            core_dump_protection_honesty,
        ) = Self::honesty_unmeasured();
        Self {
            present: true,
            private_data_ready: false,
            sealed_at_close: true,
            // Residual OS swap/hibernate/snapshot risk; work file itself is page-encrypted.
            open_work_plaintext_risk: true,
            open_work_page_encrypted: true,
            work_wipe_on_close: true,
            sqlcipher_enabled: true,
            os_keyring_available: false,
            os_keyring_used: false,
            probes_present: false,
            swap_snapshot_honesty_present: false,
            residual_risk_classes_open: Self::residual_classes_open(),
            pagefile_existence: OsResidualFileProbe::NotApplicable,
            hibernate_file_existence: OsResidualFileProbe::NotApplicable,
            swap_existence: OsResidualFileProbe::NotApplicable,
            snapshot_existence: OsResidualFileProbe::NotApplicable,
            core_dump_config_existence: OsResidualFileProbe::NotApplicable,
            pagefile_existence_honesty,
            pagefile_protection_honesty,
            swap_existence_honesty,
            swap_protection_honesty,
            snapshot_existence_honesty,
            snapshot_protection_honesty,
            core_dump_existence_honesty,
            core_dump_protection_honesty,
            vault_leftover_scan_available: true,
            crash_sidecar_detect_available: true,
            encrypted_authority_sync_qualified: false,
        }
    }

    /// Spec 028 READY_BASE: OS keyring custody fields; PRIVATE_DATA_READY still false
    /// (swap/hibernate/snapshots unqualified; REAL_PHI unauthorized).
    #[must_use]
    pub fn spec_028_honest(os_keyring_available: bool, os_keyring_used: bool) -> Self {
        let (
            pagefile_existence_honesty,
            pagefile_protection_honesty,
            swap_existence_honesty,
            swap_protection_honesty,
            snapshot_existence_honesty,
            snapshot_protection_honesty,
            core_dump_existence_honesty,
            core_dump_protection_honesty,
        ) = Self::honesty_unmeasured();
        Self {
            present: true,
            private_data_ready: false,
            sealed_at_close: true,
            open_work_plaintext_risk: true,
            open_work_page_encrypted: true,
            work_wipe_on_close: true,
            sqlcipher_enabled: true,
            os_keyring_available,
            os_keyring_used,
            probes_present: false,
            swap_snapshot_honesty_present: false,
            residual_risk_classes_open: Self::residual_classes_open(),
            pagefile_existence: OsResidualFileProbe::NotApplicable,
            hibernate_file_existence: OsResidualFileProbe::NotApplicable,
            swap_existence: OsResidualFileProbe::NotApplicable,
            snapshot_existence: OsResidualFileProbe::NotApplicable,
            core_dump_config_existence: OsResidualFileProbe::NotApplicable,
            pagefile_existence_honesty,
            pagefile_protection_honesty,
            swap_existence_honesty,
            swap_protection_honesty,
            snapshot_existence_honesty,
            snapshot_protection_honesty,
            core_dump_existence_honesty,
            core_dump_protection_honesty,
            vault_leftover_scan_available: true,
            crash_sidecar_detect_available: true,
            encrypted_authority_sync_qualified: false,
        }
    }

    /// Spec 032 READY_BASE: privacy probes; PRIVATE_DATA_READY still false.
    #[must_use]
    pub fn spec_032_honest(
        os_keyring_available: bool,
        os_keyring_used: bool,
        pagefile_existence: OsResidualFileProbe,
        hibernate_file_existence: OsResidualFileProbe,
    ) -> Self {
        let (
            pagefile_existence_honesty,
            pagefile_protection_honesty,
            swap_existence_honesty,
            swap_protection_honesty,
            snapshot_existence_honesty,
            snapshot_protection_honesty,
            core_dump_existence_honesty,
            core_dump_protection_honesty,
        ) = Self::honesty_unmeasured();
        Self {
            present: true,
            private_data_ready: false,
            sealed_at_close: true,
            open_work_plaintext_risk: true,
            open_work_page_encrypted: true,
            work_wipe_on_close: true,
            sqlcipher_enabled: true,
            os_keyring_available,
            os_keyring_used,
            probes_present: true,
            swap_snapshot_honesty_present: false,
            residual_risk_classes_open: Self::residual_classes_open(),
            pagefile_existence,
            hibernate_file_existence,
            swap_existence: OsResidualFileProbe::NotApplicable,
            snapshot_existence: OsResidualFileProbe::NotApplicable,
            core_dump_config_existence: OsResidualFileProbe::NotApplicable,
            pagefile_existence_honesty,
            pagefile_protection_honesty,
            swap_existence_honesty,
            swap_protection_honesty,
            snapshot_existence_honesty,
            snapshot_protection_honesty,
            core_dump_existence_honesty,
            core_dump_protection_honesty,
            vault_leftover_scan_available: true,
            crash_sidecar_detect_available: true,
            encrypted_authority_sync_qualified: true,
        }
    }

    /// Spec 043 READY_BASE: classified swap/snapshot/core-dump honesty; PRIVATE_DATA_READY still false.
    #[must_use]
    pub fn spec_043_honest(input: Spec043VaultPrivacyInput) -> Self {
        Self {
            present: true,
            private_data_ready: false,
            sealed_at_close: true,
            open_work_plaintext_risk: true,
            open_work_page_encrypted: true,
            work_wipe_on_close: true,
            sqlcipher_enabled: true,
            os_keyring_available: input.os_keyring_available,
            os_keyring_used: input.os_keyring_used,
            probes_present: true,
            swap_snapshot_honesty_present: true,
            residual_risk_classes_open: Self::residual_classes_open(),
            pagefile_existence: input.pagefile_existence,
            hibernate_file_existence: input.hibernate_file_existence,
            swap_existence: input.swap_existence,
            snapshot_existence: input.snapshot_existence,
            core_dump_config_existence: input.core_dump_config_existence,
            pagefile_existence_honesty: input.pagefile_existence_honesty,
            pagefile_protection_honesty: input.pagefile_protection_honesty,
            swap_existence_honesty: input.swap_existence_honesty,
            swap_protection_honesty: input.swap_protection_honesty,
            snapshot_existence_honesty: input.snapshot_existence_honesty,
            snapshot_protection_honesty: input.snapshot_protection_honesty,
            core_dump_existence_honesty: input.core_dump_existence_honesty,
            core_dump_protection_honesty: input.core_dump_protection_honesty,
            vault_leftover_scan_available: true,
            crash_sidecar_detect_available: true,
            encrypted_authority_sync_qualified: true,
        }
    }
}

/// Inputs for Spec 043 vault privacy doctor construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spec043VaultPrivacyInput {
    pub os_keyring_available: bool,
    pub os_keyring_used: bool,
    pub pagefile_existence: OsResidualFileProbe,
    pub hibernate_file_existence: OsResidualFileProbe,
    pub swap_existence: OsResidualFileProbe,
    pub snapshot_existence: OsResidualFileProbe,
    pub core_dump_config_existence: OsResidualFileProbe,
    pub pagefile_existence_honesty: ProbeHonestyClass,
    pub pagefile_protection_honesty: ProbeHonestyClass,
    pub swap_existence_honesty: ProbeHonestyClass,
    pub swap_protection_honesty: ProbeHonestyClass,
    pub snapshot_existence_honesty: ProbeHonestyClass,
    pub snapshot_protection_honesty: ProbeHonestyClass,
    pub core_dump_existence_honesty: ProbeHonestyClass,
    pub core_dump_protection_honesty: ProbeHonestyClass,
}

/// Host / client session authority posture (Spec 018).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostAuthorityDoctorStatus {
    pub present: bool,
    pub ready_base: bool,
    pub multi_client_release_ready: bool,
    pub os_ipc_qualified: bool,
}

impl HostAuthorityDoctorStatus {
    /// Spec 024 READY_BASE: localhost OS IPC + strict sessions; multi-client release not claimed.
    #[must_use]
    pub fn ready_base() -> Self {
        Self {
            present: true,
            ready_base: true,
            multi_client_release_ready: false,
            os_ipc_qualified: true,
        }
    }
}

/// Record semantics posture (Spec 019 Q06). READY_BASE; not release/privacy claims.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordSemanticsDoctorStatus {
    pub present: bool,
    pub ready_base: bool,
    pub precision_aware_time: bool,
    pub append_only_amendments: bool,
    pub explicit_identity_reconciliation: bool,
    pub release_ready: bool,
}

impl RecordSemanticsDoctorStatus {
    #[must_use]
    pub fn ready_base() -> Self {
        Self {
            present: true,
            ready_base: true,
            precision_aware_time: true,
            append_only_amendments: true,
            explicit_identity_reconciliation: true,
            release_ready: false,
        }
    }
}

/// `medscale doctor` structured report (no secrets).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctorReport {
    pub product_name: String,
    pub version: String,
    pub local_only: bool,
    pub product_runtime_egress: String,
    pub synthetic_only: bool,
    pub real_phi_authorized: bool,
    pub vault_root: Option<String>,
    pub vault_open: bool,
    pub sync_risk: SyncRisk,
    pub filesystem_claim_ok: bool,
    pub key_store: KeyStoreAvailability,
    pub privacy_proof_freshness: PrivacyFreshness,
    pub desktop_shell: String,
    pub tauri_admitted: bool,
    pub network_broker: NetworkBrokerDoctorStatus,
    pub packs_runtime: PacksRuntimeDoctorStatus,
    pub mobile: MobileDoctorStatus,
    pub controlled_actions: ControlledActionsDoctorStatus,
    pub online_packs: OnlinePacksDoctorStatus,
    pub mesc_artifact: ExternalArtifactDoctorStatus,
    pub vault_privacy: VaultPrivacyDoctorStatus,
    pub host_authority: HostAuthorityDoctorStatus,
    pub record_semantics: RecordSemanticsDoctorStatus,
    pub fhir_interchange: FhirInterchangeDoctorStatus,
    pub fhir_support_matrix: FhirSupportMatrix,
    pub workflow: WorkflowDoctorStatus,
    pub release_qualification: ReleaseQualificationDoctorStatus,
    pub accessibility: AccessibilityDoctorStatus,
    pub evidence_corpus: crate::evidence::EvidenceCorpusDoctorStatus,
    pub pack_signer: PackSignerDoctorStatus,
    pub os_sandbox: OsSandboxDoctorStatus,
    pub notes: Vec<String>,
}

/// Typed PRIVACY_PROOF evidence artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacyProof {
    pub schema_version: u32,
    pub spec_id: String,
    pub claim_scope: String,
    pub synthetic_only: bool,
    pub real_phi_authorized: bool,
    pub attributable_egress: String,
    pub network_capability_audit: String,
    pub log_crash_marker_scan: String,
    pub webview_cache_scan: WebViewScanStatus,
    pub vault_sync_root_check: String,
    pub tauri_status: String,
    pub limitations: Vec<String>,
}

impl PrivacyProof {
    /// Spec 006 baseline proof (CLI + non-WebView desktop scaffold).
    #[must_use]
    pub fn spec_006_baseline() -> Self {
        Self {
            schema_version: 1,
            spec_id: "006-cli-desktop-foundation".to_owned(),
            claim_scope: "MedScale CLI + in-process Core Host on synthetic fixtures; Desktop thin scaffold without WebView".to_owned(),
            synthetic_only: true,
            real_phi_authorized: false,
            attributable_egress: "DEFAULT_DENY; no product Network Broker calls in Spec 006 wedge".to_owned(),
            network_capability_audit: "workspace crates have no runtime HTTP client for product egress".to_owned(),
            log_crash_marker_scan: "doctor/CLI output secret-marker scan required in tests".to_owned(),
            webview_cache_scan: WebViewScanStatus::NotApplicable,
            vault_sync_root_check: "assert_claim_path refuse markers (Spec 003/005)".to_owned(),
            tauri_status: "DEFERRED — not admitted; TAURI_WEBVIEW_PRIVACY_QUALIFICATION external gate".to_owned(),
            limitations: vec![
                "Does not claim zero packets system-wide outside MedScale process boundary".to_owned(),
                "Tauri/WebView privacy qualification deferred; Desktop is non-WebView scaffold only".to_owned(),
                "REAL_PHI remains unauthorized; synthetic-only".to_owned(),
                "OS keyring READY_BASE in Spec 028; PRIVATE_DATA_READY still false (swap/snapshot)".to_owned(),
            ],
        }
    }
}

#[cfg(test)]
mod release_qualification_tests {
    use super::{AccessibilityDoctorStatus, ReleaseQualificationDoctorStatus};

    #[test]
    fn prep_ready_base_never_claims_release_ready() {
        let s = ReleaseQualificationDoctorStatus::prep_ready_base();
        assert!(s.is_honest_prep());
        assert!(!s.release_ready);
        assert!(s.macos_ci_present);
        assert!(!s.macos_qualified);
        assert!(s.perf_harness_present);
        assert!(s.sbom_scaffold_present);
        assert!(s.sbom_lock_bound);
        assert!(s.release_dry_run_verifier_present);
        assert!(s.migration_recovery_ready_base);
        assert!(s.package_upgrade_rollback_scaffold_present);
        assert!(s.portable_release_package_qualified);
        assert!(s.package_lifecycle_qualified);
        assert!(s.host_perf_measurement_path_present);
        assert!(s.runtime_perf_measurement_coverage_present);
        assert!(s.required_checks_packet_synced);
        assert!(s.release_sbom_qualified);
        assert!(s.notice_inventory_present);
        assert!(s.rights_license_decision);
        assert!(s.material_findings_clearance);
        assert!(
            s.missing_evidence_classes
                .contains(&"macos_platform_product_qualification".to_owned())
        );
        assert!(
            !s.missing_evidence_classes
                .contains(&"repo_branch_protection_required_checks".to_owned())
        );
        assert!(
            !s.missing_evidence_classes
                .contains(&"public_source_license_choice".to_owned())
        );
        assert!(
            s.missing_evidence_classes
                .contains(&"perf_budgets_attained_on_qualified_hardware".to_owned())
        );
        assert!(
            s.missing_evidence_classes
                .contains(&"release_sbom_signing_provenance".to_owned())
        );
        assert!(
            s.missing_evidence_classes
                .contains(&"checksums_provenance_signing_verification".to_owned())
        );
        assert!(
            !s.missing_evidence_classes
                .contains(&"release_package_upgrade_rollback_proof".to_owned())
        );
        assert!(
            !s.missing_evidence_classes
                .contains(&"reproducible_release_package_contents".to_owned())
        );
        assert!(
            !s.missing_evidence_classes
                .contains(&"release_bar_migration_recovery_proof".to_owned())
        );
        assert!(
            !s.missing_evidence_classes
                .contains(&"unresolved_material_findings_clearance".to_owned())
        );
        assert!(
            s.missing_evidence_classes
                .contains(&"wcag_final_v0_ui_accessibility_qualification".to_owned())
        );
    }

    #[test]
    fn accessibility_ready_base_never_claims_wcag_or_release() {
        let s = AccessibilityDoctorStatus::ready_base();
        assert!(s.is_honest_ready_base());
        assert!(!s.wcag_conformance_claimed);
        assert!(!s.final_v0_ui_present);
        assert!(!s.release_ready);
    }
}
