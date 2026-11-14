//! OS worker sandbox (Specs 008 / 026 / 030 / 031 / 033 / 038 / 040 / 041 / 044 / 052 / 053).
//!
//! EXTERNAL_GATES: `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` remains OPEN until multi-OS
//! measured PLATFORM_QUALIFIED evidence exists (stronger composition than per-OS READY_BASE).
//! Linux Landlock (+ Spec 052 FS/TCP/rlimit composition), Windows Job Object + AppContainer
//! FS/network/LPAC, macOS Seatbelt, and macOS App Sandbox entitlements (artifact/probe) may
//! report ReadyBaseMeasured only — not PlatformQualified. Signed App Sandbox **enforcement**
//! remains external after Spec 041. Seccomp composition ReadyBaseMeasured in Spec 053.

#[cfg(target_os = "linux")]
mod linux_rlimit;
#[cfg(target_os = "linux")]
mod linux_seccomp;
mod macos_app_sandbox;
#[cfg(target_os = "macos")]
mod macos_seatbelt;
#[cfg(windows)]
mod windows_appcontainer;
#[cfg(windows)]
mod windows_job;

#[cfg(windows)]
pub use windows_appcontainer::{
    APPCONTAINER_FS_CHILD_ARG, APPCONTAINER_FS_PARENT_ARG, APPCONTAINER_LPAC_CHILD_ARG,
    APPCONTAINER_LPAC_PARENT_ARG, APPCONTAINER_NET_CHILD_ARG, APPCONTAINER_NET_PARENT_ARG,
    appcontainer_fs_child_exit_code, appcontainer_lpac_child_exit_code,
    appcontainer_net_child_exit_code, measure_appcontainer_fs_deny, measure_appcontainer_lpac_deny,
    measure_appcontainer_net_deny, resolve_appcontainer_child_exe,
};

#[cfg(target_os = "linux")]
pub use linux_seccomp::{
    SECCOMP_CHILD_ARG, SECCOMP_PARENT_ARG, measure_seccomp_composition, resolve_seccomp_probe_exe,
    seccomp_child_exit_code,
};

pub use macos_app_sandbox::{
    ENTITLEMENTS_REL_PATH, app_sandbox_container_active, resolve_entitlements_path,
    validate_entitlements_artifact,
};

use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
use std::path::PathBuf;

/// Qualification claim for an OS confinement backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsSandboxQualification {
    /// Scaffold / research only — Spec 008 exit posture for non-measured backends.
    NotPlatformQualified,
    /// Spec 026/030/031/033/038/040: measured READY_BASE — not full multi-OS PLATFORM_QUALIFIED.
    ReadyBaseMeasured,
    /// Reserved for multi-OS measured evidence + EXTERNAL_GATES close.
    PlatformQualified,
}

/// Target OS for confinement composition (GLM F-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsSandboxTarget {
    LinuxLandlock,
    /// Spec 052: Landlock FS allowlist + TCP network deny + RLIMIT_NOFILE composition.
    LinuxLandlockComposition,
    /// Spec 053: seccomp-bpf strict allowlist composition (child-measured SIGSYS deny).
    LinuxSeccompComposition,
    WindowsAppContainerJobObject,
    /// Spec 033: per-user AppContainer profile + measured host-file deny for child process.
    WindowsAppContainerFs,
    /// Spec 038: per-user AppContainer profile + measured TCP/network deny (zero capabilities).
    WindowsAppContainerNetwork,
    /// Spec 040: LPAC (ALL_APPLICATION_PACKAGES_OPT_OUT) + identity/FS/network deny.
    WindowsAppContainerLpac,
    MacosSeatbeltSandbox,
    /// Spec 041: App Sandbox entitlements artifact + detection probe (not signed enforcement).
    MacosAppSandboxEntitlements,
}

/// Documented plan for one OS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OsSandboxPlan {
    pub target: OsSandboxTarget,
    pub qualification: OsSandboxQualification,
    pub mechanisms: Vec<String>,
    pub evidence_path: String,
    pub limitations: Vec<String>,
    /// Absolute paths allowed for ReadyBaseMeasured Landlock apply (read+write under hierarchy).
    /// Unused for Windows Job Object / macOS Seatbelt network-deny READY_BASE.
    #[serde(default)]
    pub allow_paths: Vec<String>,
}

impl OsSandboxPlan {
    /// Spec 026 Linux READY_BASE measured Landlock plan (allow_paths filled by caller).
    #[must_use]
    pub fn linux_landlock_ready_base(allow_paths: Vec<String>) -> Self {
        Self {
            target: OsSandboxTarget::LinuxLandlock,
            qualification: OsSandboxQualification::ReadyBaseMeasured,
            mechanisms: vec![
                "landlock_abi_v1".to_owned(),
                "path_beneath_allowlist".to_owned(),
            ],
            evidence_path: "evidence/026-pack-signer-os-sandbox/LINUX_LANDLOCK_MEASURED.md"
                .to_owned(),
            limitations: vec![
                "ReadyBaseMeasured on Linux only — not multi-OS PLATFORM_QUALIFIED".to_owned(),
                "EXTERNAL_GATES WORKER_OS_SANDBOX_PLATFORM_QUALIFIED remains OPEN".to_owned(),
                "No seccomp/network broker composition in Spec 026".to_owned(),
            ],
            allow_paths,
        }
    }

    /// Spec 052 Linux READY_BASE: Landlock FS + TCP network deny + RLIMIT_NOFILE.
    #[must_use]
    pub fn linux_landlock_composition_ready_base(allow_paths: Vec<String>) -> Self {
        Self {
            target: OsSandboxTarget::LinuxLandlockComposition,
            qualification: OsSandboxQualification::ReadyBaseMeasured,
            mechanisms: vec![
                "landlock_abi_fs_allowlist".to_owned(),
                "landlock_tcp_network_deny".to_owned(),
                "rlimit_nofile".to_owned(),
            ],
            evidence_path:
                "evidence/052-linux-landlock-composition/LINUX_LANDLOCK_COMPOSITION_MEASURED.md"
                    .to_owned(),
            limitations: vec![
                "ReadyBaseMeasured Linux composition only — not multi-OS PLATFORM_QUALIFIED"
                    .to_owned(),
                "Requires Landlock TCP (ABI V4+) on the host kernel".to_owned(),
                "No seccomp-bpf composition in Spec 052".to_owned(),
                "EXTERNAL_GATES WORKER_OS_SANDBOX_PLATFORM_QUALIFIED remains OPEN".to_owned(),
            ],
            allow_paths,
        }
    }

    /// Spec 053 Linux READY_BASE: seccomp-bpf strict allowlist composition measured.
    #[must_use]
    pub fn linux_seccomp_composition_ready_base() -> Self {
        Self {
            target: OsSandboxTarget::LinuxSeccompComposition,
            qualification: OsSandboxQualification::ReadyBaseMeasured,
            mechanisms: vec![
                "seccomp_bpf_strict_allowlist".to_owned(),
                "sigsys_deny_child_measured".to_owned(),
                "no_new_privs".to_owned(),
            ],
            evidence_path:
                "evidence/053-linux-seccomp-composition/LINUX_SECCOMP_COMPOSITION_MEASURED.md"
                    .to_owned(),
            limitations: vec![
                "ReadyBaseMeasured Linux seccomp composition only — not multi-OS PLATFORM_QUALIFIED"
                    .to_owned(),
                "x86_64 + aarch64 filter tables only; other arches NotReadyOnThisHost".to_owned(),
                "Parent process is not seccomp-confined; worker spawn confinement is later work"
                    .to_owned(),
                "EXTERNAL_GATES WORKER_OS_SANDBOX_PLATFORM_QUALIFIED remains OPEN".to_owned(),
            ],
            allow_paths: vec![],
        }
    }

    /// Spec 030 Windows READY_BASE: Job Object active-process limit measured.
    #[must_use]
    pub fn windows_job_object_ready_base() -> Self {
        Self {
            target: OsSandboxTarget::WindowsAppContainerJobObject,
            qualification: OsSandboxQualification::ReadyBaseMeasured,
            mechanisms: vec![
                "job_object_active_process_limit".to_owned(),
                "job_object_kill_on_job_close".to_owned(),
                "appcontainer_profile_scaffold".to_owned(),
            ],
            evidence_path:
                "evidence/030-windows-appcontainer-sandbox/WINDOWS_JOB_OBJECT_MEASURED.md"
                    .to_owned(),
            limitations: vec![
                "ReadyBaseMeasured Job Object process-limit only — not multi-OS PLATFORM_QUALIFIED"
                    .to_owned(),
                "AppContainer FS/network/LPAC ReadyBaseMeasured are Specs 033/038/040".to_owned(),
                "Job Objects do not provide Landlock-equivalent path allowlists".to_owned(),
                "EXTERNAL_GATES WORKER_OS_SANDBOX_PLATFORM_QUALIFIED remains OPEN".to_owned(),
            ],
            allow_paths: vec![],
        }
    }

    /// Spec 033 Windows READY_BASE: AppContainer child FS deny measured.
    #[must_use]
    pub fn windows_appcontainer_fs_ready_base() -> Self {
        Self {
            target: OsSandboxTarget::WindowsAppContainerFs,
            qualification: OsSandboxQualification::ReadyBaseMeasured,
            mechanisms: vec![
                "appcontainer_profile_create_derive".to_owned(),
                "proc_thread_security_capabilities".to_owned(),
                "host_temp_marker_fs_deny".to_owned(),
            ],
            evidence_path: "evidence/033-windows-appcontainer-fs/WINDOWS_APPCONTAINER_FS_MEASURED.md"
                .to_owned(),
            limitations: vec![
                "ReadyBaseMeasured AppContainer child FS deny only — not multi-OS PLATFORM_QUALIFIED"
                    .to_owned(),
                "Does not apply Landlock-equivalent allowlists to the parent process".to_owned(),
                "Network ReadyBaseMeasured is Spec 038; LPAC is Spec 040".to_owned(),
                "Requires medscale-os-sandbox-probe helper for CreateProcess child".to_owned(),
                "EXTERNAL_GATES WORKER_OS_SANDBOX_PLATFORM_QUALIFIED remains OPEN".to_owned(),
            ],
            allow_paths: vec![],
        }
    }

    /// Spec 038 Windows READY_BASE: AppContainer child network deny measured (zero capabilities).
    #[must_use]
    pub fn windows_appcontainer_network_ready_base() -> Self {
        Self {
            target: OsSandboxTarget::WindowsAppContainerNetwork,
            qualification: OsSandboxQualification::ReadyBaseMeasured,
            mechanisms: vec![
                "appcontainer_profile_create_derive".to_owned(),
                "proc_thread_security_capabilities_zero_caps".to_owned(),
                "tcp_connect_network_deny".to_owned(),
            ],
            evidence_path:
                "evidence/038-windows-appcontainer-network/WINDOWS_APPCONTAINER_NETWORK_MEASURED.md"
                    .to_owned(),
            limitations: vec![
                "ReadyBaseMeasured AppContainer child network deny only — not multi-OS PLATFORM_QUALIFIED"
                    .to_owned(),
                "LPAC ReadyBaseMeasured is Spec 040".to_owned(),
                "Requires medscale-os-sandbox-probe helper for CreateProcess child".to_owned(),
                "EXTERNAL_GATES WORKER_OS_SANDBOX_PLATFORM_QUALIFIED remains OPEN".to_owned(),
            ],
            allow_paths: vec![],
        }
    }

    /// Spec 040 Windows READY_BASE: LPAC AppContainer identity + FS + network deny measured.
    #[must_use]
    pub fn windows_appcontainer_lpac_ready_base() -> Self {
        Self {
            target: OsSandboxTarget::WindowsAppContainerLpac,
            qualification: OsSandboxQualification::ReadyBaseMeasured,
            mechanisms: vec![
                "appcontainer_profile_create_derive".to_owned(),
                "proc_thread_security_capabilities_zero_caps".to_owned(),
                "all_application_packages_opt_out_lpac".to_owned(),
                "lpac_identity_no_all_app_packages".to_owned(),
                "host_temp_marker_fs_deny".to_owned(),
                "tcp_connect_network_deny".to_owned(),
            ],
            evidence_path:
                "evidence/040-windows-appcontainer-lpac/WINDOWS_APPCONTAINER_LPAC_MEASURED.md"
                    .to_owned(),
            limitations: vec![
                "ReadyBaseMeasured LPAC identity/FS/network only — not multi-OS PLATFORM_QUALIFIED"
                    .to_owned(),
                "Does not enumerate full LPAC capability allowlist matrix".to_owned(),
                "Parent process is not AppContainer-confined".to_owned(),
                "Requires medscale-os-sandbox-probe helper for CreateProcess child".to_owned(),
                "EXTERNAL_GATES WORKER_OS_SANDBOX_PLATFORM_QUALIFIED remains OPEN".to_owned(),
            ],
            allow_paths: vec![],
        }
    }

    /// Residual scaffold (NotPlatformQualified): composition / brokered-handles candidates.
    /// LPAC ReadyBaseMeasured is Spec 040 (separate plan).
    #[must_use]
    pub fn windows_appcontainer_scaffold() -> Self {
        Self {
            target: OsSandboxTarget::WindowsAppContainerJobObject,
            qualification: OsSandboxQualification::NotPlatformQualified,
            mechanisms: vec![
                "brokered_handles_candidate".to_owned(),
                "multi_os_composition_candidate".to_owned(),
            ],
            evidence_path: "evidence/040-windows-appcontainer-lpac/LIMITATIONS.md".to_owned(),
            limitations: vec![
                "Not PLATFORM_QUALIFIED".to_owned(),
                "LPAC ReadyBaseMeasured is Spec 040; this scaffold covers remaining composition residuals"
                    .to_owned(),
            ],
            allow_paths: vec![],
        }
    }

    /// Spec 031 macOS READY_BASE: Seatbelt `sandbox_init` network-deny measured.
    /// App Sandbox entitlements ReadyBaseMeasured is Spec 041 (separate).
    #[must_use]
    pub fn macos_seatbelt_ready_base() -> Self {
        Self {
            target: OsSandboxTarget::MacosSeatbeltSandbox,
            qualification: OsSandboxQualification::ReadyBaseMeasured,
            mechanisms: vec![
                "sandbox_init_sbpl".to_owned(),
                "seatbelt_network_deny".to_owned(),
            ],
            evidence_path: "evidence/031-macos-seatbelt-sandbox/MACOS_SEATBELT_MEASURED.md"
                .to_owned(),
            limitations: vec![
                "ReadyBaseMeasured Seatbelt network-deny only — not multi-OS PLATFORM_QUALIFIED"
                    .to_owned(),
                "App Sandbox entitlements ReadyBaseMeasured is Spec 041; signed enforcement still external"
                    .to_owned(),
                "sandbox_init is deprecated in headers but still ships; not entitlement App Sandbox"
                    .to_owned(),
                "EXTERNAL_GATES WORKER_OS_SANDBOX_PLATFORM_QUALIFIED remains OPEN".to_owned(),
            ],
            allow_paths: vec![],
        }
    }

    /// Spec 041 macOS READY_BASE: App Sandbox entitlements artifact + detection probe.
    /// Signed runtime enforcement remains external (`enforcement_measured=false`).
    #[must_use]
    pub fn macos_app_sandbox_entitlements_ready_base() -> Self {
        Self {
            target: OsSandboxTarget::MacosAppSandboxEntitlements,
            qualification: OsSandboxQualification::ReadyBaseMeasured,
            mechanisms: vec![
                "entitlements_plist_artifact".to_owned(),
                "app_sandbox_container_id_probe".to_owned(),
            ],
            evidence_path:
                "evidence/041-macos-app-sandbox-entitlements/MACOS_APP_SANDBOX_ENTITLEMENTS_MEASURED.md"
                    .to_owned(),
            limitations: vec![
                "ReadyBaseMeasured entitlements artifact + detection probe only — not signed enforcement"
                    .to_owned(),
                "Runtime App Sandbox enforcement requires codesign (SIGNING_ACTION external)"
                    .to_owned(),
                "Seatbelt Spec 031 is separate and is not App Sandbox".to_owned(),
                "EXTERNAL_GATES WORKER_OS_SANDBOX_PLATFORM_QUALIFIED remains OPEN".to_owned(),
            ],
            allow_paths: vec![],
        }
    }

    /// Residual scaffold (NotPlatformQualified): XPC / signed-enforcement composition candidates.
    #[must_use]
    pub fn macos_seatbelt_scaffold() -> Self {
        Self {
            target: OsSandboxTarget::MacosSeatbeltSandbox,
            qualification: OsSandboxQualification::NotPlatformQualified,
            mechanisms: vec![
                "xpc_pattern_candidate".to_owned(),
                "signed_app_sandbox_enforcement_candidate".to_owned(),
            ],
            evidence_path: "evidence/041-macos-app-sandbox-entitlements/LIMITATIONS.md".to_owned(),
            limitations: vec![
                "Not PLATFORM_QUALIFIED".to_owned(),
                "App Sandbox entitlements artifact/probe is Spec 041; signed enforcement still external"
                    .to_owned(),
            ],
            allow_paths: vec![],
        }
    }

    /// Scaffold / READY_BASE plans; Linux + Windows + macOS may claim ReadyBaseMeasured.
    #[must_use]
    pub fn all_plans() -> Vec<Self> {
        vec![
            Self::linux_landlock_ready_base(vec![]),
            Self::linux_landlock_composition_ready_base(vec![]),
            Self::windows_job_object_ready_base(),
            Self::windows_appcontainer_fs_ready_base(),
            Self::windows_appcontainer_network_ready_base(),
            Self::windows_appcontainer_lpac_ready_base(),
            Self::windows_appcontainer_scaffold(),
            Self::macos_seatbelt_ready_base(),
            Self::macos_app_sandbox_entitlements_ready_base(),
            Self::macos_seatbelt_scaffold(),
        ]
    }

    #[must_use]
    pub fn claims_platform_qualified(&self) -> bool {
        matches!(
            self.qualification,
            OsSandboxQualification::PlatformQualified
        )
    }

    #[must_use]
    pub fn claims_ready_base_measured(&self) -> bool {
        matches!(
            self.qualification,
            OsSandboxQualification::ReadyBaseMeasured
        )
    }
}

/// OS sandbox doctor axis (Specs 026 + 030 + 031 + 033 + 038 + 040 + 041 + 044).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OsSandboxDoctorStatus {
    pub present: bool,
    pub ready_base: bool,
    pub linux_measured: bool,
    /// Spec 052: Linux Landlock FS + TCP deny + RLIMIT_NOFILE composition measured.
    pub linux_landlock_composition_measured: bool,
    /// Spec 053: Linux seccomp-bpf strict allowlist composition measured.
    pub linux_seccomp_composition_measured: bool,
    /// Spec 030: Windows Job Object ReadyBaseMeasured evidence exists in-tree.
    pub windows_measured: bool,
    /// Spec 033: Windows AppContainer FS ReadyBaseMeasured evidence exists in-tree.
    pub windows_appcontainer_fs_measured: bool,
    /// Spec 038: Windows AppContainer network ReadyBaseMeasured evidence exists in-tree.
    pub windows_appcontainer_network_measured: bool,
    /// Spec 040: Windows AppContainer LPAC ReadyBaseMeasured evidence exists in-tree.
    pub windows_appcontainer_lpac_measured: bool,
    /// Spec 031: macOS Seatbelt ReadyBaseMeasured evidence exists in-tree.
    pub macos_measured: bool,
    /// Spec 041: macOS App Sandbox entitlements artifact/probe ReadyBaseMeasured.
    pub macos_app_sandbox_entitlements_measured: bool,
    /// Spec 041 honesty: signed App Sandbox runtime enforcement not measured.
    pub macos_app_sandbox_enforcement_measured: bool,
    /// Spec 044: multi-OS composition inventory present (does not imply PLATFORM_QUALIFIED).
    pub composition_inventory_present: bool,
    /// Spec 044: residual axes still blocking PLATFORM_QUALIFIED.
    pub composition_residuals_open: Vec<String>,
    pub platform_qualified: bool,
    pub release_ready: bool,
}

impl OsSandboxDoctorStatus {
    #[must_use]
    pub fn ready_base() -> Self {
        Self {
            present: true,
            ready_base: true,
            linux_measured: true,
            linux_landlock_composition_measured: true,
            linux_seccomp_composition_measured: true,
            windows_measured: true,
            windows_appcontainer_fs_measured: true,
            windows_appcontainer_network_measured: true,
            windows_appcontainer_lpac_measured: true,
            macos_measured: true,
            macos_app_sandbox_entitlements_measured: true,
            macos_app_sandbox_enforcement_measured: false,
            composition_inventory_present: true,
            composition_residuals_open: composition_residuals_open(),
            platform_qualified: false,
            release_ready: false,
        }
    }

    #[must_use]
    pub fn is_honest_ready_base(&self) -> bool {
        self.present
            && self.ready_base
            && self.linux_measured
            && self.linux_landlock_composition_measured
            && self.linux_seccomp_composition_measured
            && self.windows_measured
            && self.windows_appcontainer_fs_measured
            && self.windows_appcontainer_network_measured
            && self.windows_appcontainer_lpac_measured
            && self.macos_measured
            && self.macos_app_sandbox_entitlements_measured
            && !self.macos_app_sandbox_enforcement_measured
            && self.composition_inventory_present
            && !self.composition_residuals_open.is_empty()
            && !self.platform_qualified
            && !self.release_ready
    }
}

/// Residual axes that keep WORKER_OS_SANDBOX_PLATFORM_QUALIFIED OPEN (Spec 044).
#[must_use]
pub fn composition_residuals_open() -> Vec<String> {
    vec![
        "macos_app_sandbox_signed_enforcement".to_owned(),
        "multi_os_platform_qualified_composition".to_owned(),
    ]
}

/// Inventory of ReadyBaseMeasured OS sandbox axes (Spec 044). Never claims PLATFORM_QUALIFIED.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OsSandboxCompositionInventory {
    pub axes_ready_base_measured: Vec<String>,
    pub residuals_open: Vec<String>,
    pub platform_qualified: bool,
}

impl OsSandboxCompositionInventory {
    #[must_use]
    pub fn trusted_v1_ready_base() -> Self {
        Self {
            axes_ready_base_measured: vec![
                "linux_landlock".to_owned(),
                "linux_landlock_composition".to_owned(),
                "linux_seccomp_composition".to_owned(),
                "windows_job_object".to_owned(),
                "windows_appcontainer_fs".to_owned(),
                "windows_appcontainer_network".to_owned(),
                "windows_appcontainer_lpac".to_owned(),
                "macos_seatbelt".to_owned(),
                "macos_app_sandbox_entitlements".to_owned(),
            ],
            residuals_open: composition_residuals_open(),
            platform_qualified: false,
        }
    }

    #[must_use]
    pub fn is_honest(&self) -> bool {
        !self.platform_qualified
            && !self.axes_ready_base_measured.is_empty()
            && !self.residuals_open.is_empty()
    }
}

/// Runtime apply errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OsSandboxApplyError {
    NotPlatformQualified,
    NotReadyOnThisHost,
    EmptyAllowlist,
    ApplyFailed(String),
}

/// Apply OS sandbox according to plan qualification.
///
/// - `ReadyBaseMeasured` + LinuxLandlock on Linux: Landlock allowlist apply.
/// - `ReadyBaseMeasured` + WindowsAppContainerJobObject on Windows: Job Object limits.
/// - `ReadyBaseMeasured` + WindowsAppContainerFs on Windows: AppContainer child FS deny measure.
/// - `ReadyBaseMeasured` + WindowsAppContainerNetwork on Windows: AppContainer child network deny.
/// - `ReadyBaseMeasured` + WindowsAppContainerLpac on Windows: LPAC identity/FS/network deny.
/// - `ReadyBaseMeasured` + LinuxSeccompComposition on Linux: seccomp child SIGSYS measure.
/// - `ReadyBaseMeasured` + MacosSeatbeltSandbox on macOS: Seatbelt network-deny.
/// - `PlatformQualified`: still refused while EXTERNAL_GATES remains OPEN.
/// - Composition / App Sandbox scaffolds: NotPlatformQualified.
pub fn try_apply_os_sandbox(plan: &OsSandboxPlan) -> Result<(), OsSandboxApplyError> {
    if plan.claims_platform_qualified() {
        // Gate OPEN — never honor PlatformQualified without EXTERNAL_GATES close.
        return Err(OsSandboxApplyError::NotPlatformQualified);
    }

    if !plan.claims_ready_base_measured() {
        return Err(OsSandboxApplyError::NotPlatformQualified);
    }

    match plan.target {
        OsSandboxTarget::LinuxLandlock => apply_linux_ready_base(plan),
        OsSandboxTarget::LinuxLandlockComposition => apply_linux_composition_ready_base(plan),
        OsSandboxTarget::LinuxSeccompComposition => apply_linux_seccomp_ready_base(plan),
        OsSandboxTarget::WindowsAppContainerJobObject => apply_windows_job_ready_base(plan),
        OsSandboxTarget::WindowsAppContainerFs => apply_windows_appcontainer_fs_ready_base(plan),
        OsSandboxTarget::WindowsAppContainerNetwork => {
            apply_windows_appcontainer_net_ready_base(plan)
        }
        OsSandboxTarget::WindowsAppContainerLpac => {
            apply_windows_appcontainer_lpac_ready_base(plan)
        }
        OsSandboxTarget::MacosSeatbeltSandbox => apply_macos_ready_base(plan),
        OsSandboxTarget::MacosAppSandboxEntitlements => {
            apply_macos_app_sandbox_entitlements_ready_base(plan)
        }
    }
}

fn apply_linux_ready_base(plan: &OsSandboxPlan) -> Result<(), OsSandboxApplyError> {
    if plan.allow_paths.is_empty() {
        return Err(OsSandboxApplyError::EmptyAllowlist);
    }

    #[cfg(target_os = "linux")]
    {
        apply_landlock_linux(plan)
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = plan;
        Err(OsSandboxApplyError::NotReadyOnThisHost)
    }
}

fn apply_linux_composition_ready_base(plan: &OsSandboxPlan) -> Result<(), OsSandboxApplyError> {
    if plan.allow_paths.is_empty() {
        return Err(OsSandboxApplyError::EmptyAllowlist);
    }

    #[cfg(target_os = "linux")]
    {
        apply_landlock_composition_linux(plan)
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = plan;
        Err(OsSandboxApplyError::NotReadyOnThisHost)
    }
}

fn apply_linux_seccomp_ready_base(_plan: &OsSandboxPlan) -> Result<(), OsSandboxApplyError> {
    #[cfg(target_os = "linux")]
    {
        match resolve_seccomp_probe_exe() {
            Ok(exe) => measure_seccomp_composition(&exe),
            Err(e) => Err(e),
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(OsSandboxApplyError::NotReadyOnThisHost)
    }
}

fn apply_windows_job_ready_base(_plan: &OsSandboxPlan) -> Result<(), OsSandboxApplyError> {
    #[cfg(windows)]
    {
        windows_job::apply_job_object_windows()
    }

    #[cfg(not(windows))]
    {
        Err(OsSandboxApplyError::NotReadyOnThisHost)
    }
}

fn apply_windows_appcontainer_fs_ready_base(
    _plan: &OsSandboxPlan,
) -> Result<(), OsSandboxApplyError> {
    #[cfg(windows)]
    {
        windows_appcontainer::apply_appcontainer_fs_windows()
    }

    #[cfg(not(windows))]
    {
        Err(OsSandboxApplyError::NotReadyOnThisHost)
    }
}

fn apply_windows_appcontainer_net_ready_base(
    _plan: &OsSandboxPlan,
) -> Result<(), OsSandboxApplyError> {
    #[cfg(windows)]
    {
        windows_appcontainer::apply_appcontainer_net_windows()
    }

    #[cfg(not(windows))]
    {
        Err(OsSandboxApplyError::NotReadyOnThisHost)
    }
}

fn apply_windows_appcontainer_lpac_ready_base(
    _plan: &OsSandboxPlan,
) -> Result<(), OsSandboxApplyError> {
    #[cfg(windows)]
    {
        windows_appcontainer::apply_appcontainer_lpac_windows()
    }

    #[cfg(not(windows))]
    {
        Err(OsSandboxApplyError::NotReadyOnThisHost)
    }
}

fn apply_macos_ready_base(_plan: &OsSandboxPlan) -> Result<(), OsSandboxApplyError> {
    #[cfg(target_os = "macos")]
    {
        macos_seatbelt::apply_seatbelt_macos()
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(OsSandboxApplyError::NotReadyOnThisHost)
    }
}

fn apply_macos_app_sandbox_entitlements_ready_base(
    _plan: &OsSandboxPlan,
) -> Result<(), OsSandboxApplyError> {
    // Artifact + env probe are host-portable; signed enforcement remains external.
    let _ = _plan;
    macos_app_sandbox::apply_macos_app_sandbox_entitlements()
}

#[cfg(target_os = "linux")]
fn apply_landlock_linux(plan: &OsSandboxPlan) -> Result<(), OsSandboxApplyError> {
    use landlock::{
        ABI, Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetStatus,
        path_beneath_rules,
    };

    let paths: Vec<PathBuf> = plan.allow_paths.iter().map(PathBuf::from).collect();
    for p in &paths {
        if !p.is_absolute() {
            return Err(OsSandboxApplyError::ApplyFailed(
                "allow_paths must be absolute".to_owned(),
            ));
        }
    }

    let abi = ABI::V1;
    let access = AccessFs::from_all(abi);
    let status = Ruleset::default()
        .handle_access(access)
        .map_err(|e| OsSandboxApplyError::ApplyFailed(e.to_string()))?
        .create()
        .map_err(|e| OsSandboxApplyError::ApplyFailed(e.to_string()))?
        .add_rules(path_beneath_rules(
            paths.iter().map(|p| p.as_path()),
            access,
        ))
        .map_err(|e| OsSandboxApplyError::ApplyFailed(e.to_string()))?
        .restrict_self()
        .map_err(|e| OsSandboxApplyError::ApplyFailed(e.to_string()))?;

    match status.ruleset {
        RulesetStatus::NotEnforced => Err(OsSandboxApplyError::ApplyFailed(
            "landlock not enforced (kernel/ABI)".to_owned(),
        )),
        RulesetStatus::PartiallyEnforced | RulesetStatus::FullyEnforced => Ok(()),
    }
}

/// Spec 052: Landlock FS allowlist + TCP net deny (no NetPort allows) + RLIMIT_NOFILE.
#[cfg(target_os = "linux")]
fn apply_landlock_composition_linux(plan: &OsSandboxPlan) -> Result<(), OsSandboxApplyError> {
    use landlock::{
        ABI, Access, AccessFs, AccessNet, Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetStatus,
        path_beneath_rules,
    };

    let paths: Vec<PathBuf> = plan.allow_paths.iter().map(PathBuf::from).collect();
    for p in &paths {
        if !p.is_absolute() {
            return Err(OsSandboxApplyError::ApplyFailed(
                "allow_paths must be absolute".to_owned(),
            ));
        }
    }

    // Prefer ABI V4 so AccessNet is available; fail closed if net rights cannot
    // be handled (Spec 052 requires the network axis — no silent FS-only apply).
    let abi = ABI::V4;
    let access_fs = AccessFs::from_all(abi);
    let access_net = AccessNet::BindTcp | AccessNet::ConnectTcp;

    let status = Ruleset::default()
        .handle_access(access_fs)
        .map_err(|e| OsSandboxApplyError::ApplyFailed(e.to_string()))?
        .handle_access(access_net)
        .map_err(|e| {
            OsSandboxApplyError::ApplyFailed(format!(
                "landlock TCP network deny unavailable (need ABI V4+): {e}"
            ))
        })?
        .create()
        .map_err(|e| OsSandboxApplyError::ApplyFailed(e.to_string()))?
        .add_rules(path_beneath_rules(
            paths.iter().map(|p| p.as_path()),
            access_fs,
        ))
        .map_err(|e| OsSandboxApplyError::ApplyFailed(e.to_string()))?
        // No NetPort allow rules => TCP bind/connect denied when handled.
        .restrict_self()
        .map_err(|e| OsSandboxApplyError::ApplyFailed(e.to_string()))?;

    match status.ruleset {
        RulesetStatus::NotEnforced => {
            return Err(OsSandboxApplyError::ApplyFailed(
                "landlock composition not enforced (kernel/ABI)".to_owned(),
            ));
        }
        RulesetStatus::PartiallyEnforced | RulesetStatus::FullyEnforced => {}
    }

    apply_rlimit_nofile_linux()?;
    Ok(())
}

/// Public helper for probe/tests: read current RLIMIT_NOFILE soft limit (Linux).
#[cfg(target_os = "linux")]
#[must_use]
pub fn linux_rlimit_nofile_soft() -> Option<u64> {
    linux_rlimit::soft_nofile()
}

#[cfg(target_os = "linux")]
fn apply_rlimit_nofile_linux() -> Result<(), OsSandboxApplyError> {
    linux_rlimit::apply_rlimit_nofile()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffolds_and_ready_base_claims() {
        assert!(!OsSandboxPlan::windows_appcontainer_scaffold().claims_platform_qualified());
        assert!(!OsSandboxPlan::windows_appcontainer_scaffold().claims_ready_base_measured());
        assert!(!OsSandboxPlan::macos_seatbelt_scaffold().claims_platform_qualified());
        assert!(!OsSandboxPlan::macos_seatbelt_scaffold().claims_ready_base_measured());
        assert!(!OsSandboxPlan::linux_landlock_ready_base(vec![]).claims_platform_qualified());
        assert!(OsSandboxPlan::linux_landlock_ready_base(vec![]).claims_ready_base_measured());
        assert!(
            !OsSandboxPlan::linux_landlock_composition_ready_base(vec![])
                .claims_platform_qualified()
        );
        assert!(
            OsSandboxPlan::linux_landlock_composition_ready_base(vec![])
                .claims_ready_base_measured()
        );
        assert!(OsSandboxPlan::windows_job_object_ready_base().claims_ready_base_measured());
        assert!(!OsSandboxPlan::windows_job_object_ready_base().claims_platform_qualified());
        assert!(OsSandboxPlan::windows_appcontainer_fs_ready_base().claims_ready_base_measured());
        assert!(!OsSandboxPlan::windows_appcontainer_fs_ready_base().claims_platform_qualified());
        assert!(
            OsSandboxPlan::windows_appcontainer_network_ready_base().claims_ready_base_measured()
        );
        assert!(
            !OsSandboxPlan::windows_appcontainer_network_ready_base().claims_platform_qualified()
        );
        assert!(OsSandboxPlan::windows_appcontainer_lpac_ready_base().claims_ready_base_measured());
        assert!(!OsSandboxPlan::windows_appcontainer_lpac_ready_base().claims_platform_qualified());
        assert!(OsSandboxPlan::macos_seatbelt_ready_base().claims_ready_base_measured());
        assert!(!OsSandboxPlan::macos_seatbelt_ready_base().claims_platform_qualified());
        assert!(
            OsSandboxPlan::macos_app_sandbox_entitlements_ready_base().claims_ready_base_measured()
        );
        assert!(
            !OsSandboxPlan::macos_app_sandbox_entitlements_ready_base().claims_platform_qualified()
        );
    }

    #[test]
    fn platform_qualified_still_refused() {
        let mut plan = OsSandboxPlan::linux_landlock_ready_base(vec!["/tmp".to_owned()]);
        plan.qualification = OsSandboxQualification::PlatformQualified;
        assert_eq!(
            try_apply_os_sandbox(&plan),
            Err(OsSandboxApplyError::NotPlatformQualified)
        );
        let mut win = OsSandboxPlan::windows_job_object_ready_base();
        win.qualification = OsSandboxQualification::PlatformQualified;
        assert_eq!(
            try_apply_os_sandbox(&win),
            Err(OsSandboxApplyError::NotPlatformQualified)
        );
        let mut win_fs = OsSandboxPlan::windows_appcontainer_fs_ready_base();
        win_fs.qualification = OsSandboxQualification::PlatformQualified;
        assert_eq!(
            try_apply_os_sandbox(&win_fs),
            Err(OsSandboxApplyError::NotPlatformQualified)
        );
        let mut win_lpac = OsSandboxPlan::windows_appcontainer_lpac_ready_base();
        win_lpac.qualification = OsSandboxQualification::PlatformQualified;
        assert_eq!(
            try_apply_os_sandbox(&win_lpac),
            Err(OsSandboxApplyError::NotPlatformQualified)
        );
        let mut mac = OsSandboxPlan::macos_seatbelt_ready_base();
        mac.qualification = OsSandboxQualification::PlatformQualified;
        assert_eq!(
            try_apply_os_sandbox(&mac),
            Err(OsSandboxApplyError::NotPlatformQualified)
        );
    }

    #[test]
    fn ready_base_linux_without_allowlist_fails() {
        let plan = OsSandboxPlan::linux_landlock_ready_base(vec![]);
        assert_eq!(
            try_apply_os_sandbox(&plan),
            Err(OsSandboxApplyError::EmptyAllowlist)
        );
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn linux_ready_base_on_non_linux_is_not_ready() {
        let plan = OsSandboxPlan::linux_landlock_ready_base(vec!["C:\\tmp".to_owned()]);
        assert_eq!(
            try_apply_os_sandbox(&plan),
            Err(OsSandboxApplyError::NotReadyOnThisHost)
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn windows_ready_base_on_non_windows_is_not_ready() {
        let plan = OsSandboxPlan::windows_job_object_ready_base();
        assert_eq!(
            try_apply_os_sandbox(&plan),
            Err(OsSandboxApplyError::NotReadyOnThisHost)
        );
        let plan_fs = OsSandboxPlan::windows_appcontainer_fs_ready_base();
        assert_eq!(
            try_apply_os_sandbox(&plan_fs),
            Err(OsSandboxApplyError::NotReadyOnThisHost)
        );
        let plan_net = OsSandboxPlan::windows_appcontainer_network_ready_base();
        assert_eq!(
            try_apply_os_sandbox(&plan_net),
            Err(OsSandboxApplyError::NotReadyOnThisHost)
        );
        let plan_lpac = OsSandboxPlan::windows_appcontainer_lpac_ready_base();
        assert_eq!(
            try_apply_os_sandbox(&plan_lpac),
            Err(OsSandboxApplyError::NotReadyOnThisHost)
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn macos_ready_base_on_non_macos_is_not_ready() {
        let plan = OsSandboxPlan::macos_seatbelt_ready_base();
        assert_eq!(
            try_apply_os_sandbox(&plan),
            Err(OsSandboxApplyError::NotReadyOnThisHost)
        );
    }

    #[test]
    fn doctor_axis_honest() {
        let d = OsSandboxDoctorStatus::ready_base();
        assert!(d.is_honest_ready_base());
        assert!(d.windows_measured);
        assert!(d.windows_appcontainer_fs_measured);
        assert!(d.windows_appcontainer_network_measured);
        assert!(d.windows_appcontainer_lpac_measured);
        assert!(d.linux_measured);
        assert!(d.linux_landlock_composition_measured);
        assert!(d.linux_seccomp_composition_measured);
        assert!(d.macos_measured);
        assert!(d.macos_app_sandbox_entitlements_measured);
        assert!(!d.macos_app_sandbox_enforcement_measured);
        assert!(d.composition_inventory_present);
        assert!(!d.composition_residuals_open.is_empty());
        assert!(!d.platform_qualified);
        assert!(!d.release_ready);
        let inv = OsSandboxCompositionInventory::trusted_v1_ready_base();
        assert!(inv.is_honest());
        assert_eq!(inv.axes_ready_base_measured.len(), 9);
        assert!(
            inv.axes_ready_base_measured
                .contains(&"linux_landlock_composition".to_owned())
        );
        assert!(
            inv.axes_ready_base_measured
                .contains(&"linux_seccomp_composition".to_owned())
        );
    }
}
