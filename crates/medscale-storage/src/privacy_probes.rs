//! Spec 032 OS privacy probes + Spec 043 honesty classification.
//!
//! Never claims `PRIVATE_DATA_READY` or measured swap/hibernate/snapshot **protection**.

use std::io::ErrorKind;
use std::path::Path;
#[cfg(any(windows, target_os = "macos"))]
use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::encrypted_vault::EncryptedVault;

/// Best-effort existence result for OS residual privacy surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsFileProbeResult {
    Detected,
    NotFound,
    NotReadable,
    NotApplicable,
}

impl OsFileProbeResult {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Detected => "detected",
            Self::NotFound => "not_found",
            Self::NotReadable => "not_readable",
            Self::NotApplicable => "not_applicable",
        }
    }
}

/// Honesty class for a residual privacy surface (Spec 043).
///
/// `ProtectionMeasured` is defined for the taxonomy but **must not** be returned for
/// swap/pagefile/hibernate/snapshot/core-dump in Spec 043 READY_BASE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeHonestyClass {
    ConfigurationDetected,
    ProtectionMeasured,
    NotMeasurableOnHost,
    OwnerOrOsPolicyRequired,
}

impl ProbeHonestyClass {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConfigurationDetected => "configuration_detected",
            Self::ProtectionMeasured => "protection_measured",
            Self::NotMeasurableOnHost => "not_measurable_on_host",
            Self::OwnerOrOsPolicyRequired => "owner_or_os_policy_required",
        }
    }
}

/// One residual surface: existence probe + honesty classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassifiedResidualSurface {
    pub existence: OsFileProbeResult,
    /// Existence / configuration claim for this surface.
    pub existence_honesty: ProbeHonestyClass,
    /// Protection claim — Spec 043 always leaves protection unmeasured.
    pub protection_honesty: ProbeHonestyClass,
}

impl ClassifiedResidualSurface {
    #[must_use]
    fn from_existence(existence: OsFileProbeResult) -> Self {
        let existence_honesty = match existence {
            OsFileProbeResult::Detected | OsFileProbeResult::NotFound => {
                ProbeHonestyClass::ConfigurationDetected
            }
            OsFileProbeResult::NotReadable => ProbeHonestyClass::NotMeasurableOnHost,
            OsFileProbeResult::NotApplicable => ProbeHonestyClass::NotMeasurableOnHost,
        };
        Self {
            existence,
            existence_honesty,
            protection_honesty: ProbeHonestyClass::OwnerOrOsPolicyRequired,
        }
    }
}

/// Aggregated privacy probe report for doctor honesty (Specs 032 + 043).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyProbeReport {
    pub probes_present: bool,
    /// Spec 043: classified residual surfaces wired.
    pub swap_snapshot_honesty_present: bool,
    pub pagefile: ClassifiedResidualSurface,
    pub hibernate: ClassifiedResidualSurface,
    pub swap: ClassifiedResidualSurface,
    pub snapshot: ClassifiedResidualSurface,
    pub core_dump_config: ClassifiedResidualSurface,
    pub residual_risk_classes_open: Vec<&'static str>,
    pub vault_leftover_scan_available: bool,
    pub crash_sidecar_detect_available: bool,
}

/// Convenience aliases used by Spec 032 call sites.
impl PrivacyProbeReport {
    #[must_use]
    pub fn pagefile_existence(&self) -> OsFileProbeResult {
        self.pagefile.existence
    }

    #[must_use]
    pub fn hibernate_file_existence(&self) -> OsFileProbeResult {
        self.hibernate.existence
    }
}

/// Residual risk classes that remain OPEN after Spec 043 READY_BASE.
#[must_use]
pub fn residual_risk_classes_open() -> Vec<&'static str> {
    vec!["swap", "hibernate", "snapshot", "pagefile", "core_dump"]
}

/// Probe OS residual privacy surfaces (existence + honesty; fail-soft).
#[must_use]
pub fn probe_os_privacy_surfaces() -> PrivacyProbeReport {
    let (pagefile, hibernate, swap, snapshot, core_dump) = platform_classified_probes();
    PrivacyProbeReport {
        probes_present: true,
        swap_snapshot_honesty_present: true,
        pagefile: ClassifiedResidualSurface::from_existence(pagefile),
        hibernate: ClassifiedResidualSurface::from_existence(hibernate),
        swap: ClassifiedResidualSurface::from_existence(swap),
        snapshot: ClassifiedResidualSurface::from_existence(snapshot),
        core_dump_config: ClassifiedResidualSurface::from_existence(core_dump),
        residual_risk_classes_open: residual_risk_classes_open(),
        vault_leftover_scan_available: true,
        crash_sidecar_detect_available: true,
    }
}

/// Vault temp/work leftover scan helper (Spec 017 crash leftovers).
#[must_use]
pub fn scan_vault_work_leftovers(root: &Path) -> bool {
    EncryptedVault::leftover_work_present(root)
}

/// Crash sidecar detection alias (WAL/SHM/journal beside meta.work).
#[must_use]
pub fn crash_sidecar_leftovers_present(root: &Path) -> bool {
    EncryptedVault::leftover_work_present(root)
}

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
fn probe_path(path: &Path) -> OsFileProbeResult {
    match std::fs::metadata(path) {
        Ok(_) => OsFileProbeResult::Detected,
        Err(e) if e.kind() == ErrorKind::NotFound => OsFileProbeResult::NotFound,
        // Windows often returns PermissionDenied for existing pagefile/hiberfil.
        Err(e) if e.kind() == ErrorKind::PermissionDenied => OsFileProbeResult::Detected,
        Err(_) => OsFileProbeResult::NotReadable,
    }
}

#[cfg(windows)]
fn platform_classified_probes() -> (
    OsFileProbeResult,
    OsFileProbeResult,
    OsFileProbeResult,
    OsFileProbeResult,
    OsFileProbeResult,
) {
    let pagefile = probe_path(Path::new(r"C:\pagefile.sys"));
    let hibernate = probe_path(Path::new(r"C:\hiberfil.sys"));
    let swap = probe_path(Path::new(r"C:\swapfile.sys"));
    let snapshot = windows_snapshot_probe();
    let core_dump = windows_core_dump_config_probe();
    (pagefile, hibernate, swap, snapshot, core_dump)
}

#[cfg(windows)]
fn windows_snapshot_probe() -> OsFileProbeResult {
    // System Volume Information is the VSS/restore surface; PermissionDenied ⇒ present.
    let svi = probe_path(Path::new(r"C:\System Volume Information"));
    if matches!(
        svi,
        OsFileProbeResult::Detected | OsFileProbeResult::NotFound
    ) {
        // Best-effort: try listing shadows without elevating (often Access Denied).
        // This probe runs in-process from desktop startup/Settings/About.
        // A console-subsystem child otherwise flashes a temporary CMD window
        // whenever the GUI refreshes its Doctor report. CREATE_NO_WINDOW is
        // a Win32 CreateProcess flag; the probe still executes and reports
        // the same output/status (or fails soft on access denied).
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut command = Command::new("vssadmin");
        let output = command
            .args(["list", "shadows"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        match output {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout);
                if text.to_ascii_lowercase().contains("shadow copy")
                    || text.to_ascii_lowercase().contains("no items")
                    || text.to_ascii_lowercase().contains("no entries")
                {
                    // Command succeeded: configuration measurable (presence or empty list).
                    if text.to_ascii_lowercase().contains("shadow copy id") {
                        OsFileProbeResult::Detected
                    } else {
                        OsFileProbeResult::NotFound
                    }
                } else {
                    svi
                }
            }
            Ok(_) => {
                // Non-zero: often access denied — fall back to SVI existence signal.
                svi
            }
            Err(_) => svi,
        }
    } else {
        svi
    }
}

#[cfg(windows)]
fn windows_core_dump_config_probe() -> OsFileProbeResult {
    // Crash dump folder / Wer config surface — existence only.
    let dumps = probe_path(Path::new(r"C:\Windows\Minidump"));
    if dumps != OsFileProbeResult::NotFound {
        return dumps;
    }
    probe_path(Path::new(r"C:\Users\Public\Documents\WER"))
}

#[cfg(target_os = "linux")]
fn platform_classified_probes() -> (
    OsFileProbeResult,
    OsFileProbeResult,
    OsFileProbeResult,
    OsFileProbeResult,
    OsFileProbeResult,
) {
    let swap = match std::fs::read_to_string("/proc/swaps") {
        Ok(s) => {
            let data_lines = s.lines().skip(1).filter(|l| !l.trim().is_empty()).count();
            if data_lines > 0 {
                OsFileProbeResult::Detected
            } else {
                OsFileProbeResult::NotFound
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => OsFileProbeResult::NotFound,
        Err(e) if e.kind() == ErrorKind::PermissionDenied => OsFileProbeResult::NotReadable,
        Err(_) => OsFileProbeResult::NotReadable,
    };
    // Linux has no Windows pagefile; pagefile-class maps to swap for Spec 032 compat.
    let pagefile = swap;
    let hibernate = match std::fs::read_to_string("/sys/power/disk") {
        Ok(s) if s.trim().is_empty() => OsFileProbeResult::NotFound,
        Ok(_) => OsFileProbeResult::Detected,
        Err(e) if e.kind() == ErrorKind::NotFound => OsFileProbeResult::NotFound,
        Err(e) if e.kind() == ErrorKind::PermissionDenied => OsFileProbeResult::NotReadable,
        Err(_) => OsFileProbeResult::NotReadable,
    };
    let snapshot = linux_snapshot_probe();
    let core_dump = match std::fs::read_to_string("/proc/sys/kernel/core_pattern") {
        Ok(s) if s.trim().is_empty() => OsFileProbeResult::NotFound,
        Ok(_) => OsFileProbeResult::Detected,
        Err(e) if e.kind() == ErrorKind::NotFound => OsFileProbeResult::NotFound,
        Err(e) if e.kind() == ErrorKind::PermissionDenied => OsFileProbeResult::NotReadable,
        Err(_) => OsFileProbeResult::NotReadable,
    };
    (pagefile, hibernate, swap, snapshot, core_dump)
}

#[cfg(target_os = "linux")]
fn linux_snapshot_probe() -> OsFileProbeResult {
    for candidate in [
        Path::new("/.snapshots"),
        Path::new("/mnt/.snapshots"),
        Path::new("/var/lib/snapd/snapshots"),
    ] {
        match probe_path(candidate) {
            OsFileProbeResult::Detected => return OsFileProbeResult::Detected,
            OsFileProbeResult::NotReadable => return OsFileProbeResult::NotReadable,
            _ => {}
        }
    }
    // btrfs filesystem present is a configuration hint, not a snapshot inventory.
    match std::fs::read_dir("/sys/fs/btrfs") {
        Ok(mut it) => {
            if it.next().is_some() {
                // FS type present; snapshot objects themselves not enumerated → NotFound for
                // concrete snapshot artifacts under common paths above.
                OsFileProbeResult::NotFound
            } else {
                OsFileProbeResult::NotFound
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => OsFileProbeResult::NotFound,
        Err(e) if e.kind() == ErrorKind::PermissionDenied => OsFileProbeResult::NotReadable,
        Err(_) => OsFileProbeResult::NotReadable,
    }
}

#[cfg(target_os = "macos")]
fn platform_classified_probes() -> (
    OsFileProbeResult,
    OsFileProbeResult,
    OsFileProbeResult,
    OsFileProbeResult,
    OsFileProbeResult,
) {
    let vm = Path::new("/private/var/vm");
    let swap = match std::fs::read_dir(vm) {
        Ok(entries) => {
            let any_swap = entries
                .filter_map(Result::ok)
                .any(|e| e.file_name().to_string_lossy().starts_with("swapfile"));
            if any_swap {
                OsFileProbeResult::Detected
            } else {
                OsFileProbeResult::NotFound
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => OsFileProbeResult::NotFound,
        Err(e) if e.kind() == ErrorKind::PermissionDenied => OsFileProbeResult::NotReadable,
        Err(_) => OsFileProbeResult::NotReadable,
    };
    let pagefile = swap;
    let hibernate = probe_path(Path::new("/private/var/vm/sleepimage"));
    let snapshot = macos_snapshot_probe();
    let core_dump = probe_path(Path::new("/cores"));
    (pagefile, hibernate, swap, snapshot, core_dump)
}

#[cfg(target_os = "macos")]
fn macos_snapshot_probe() -> OsFileProbeResult {
    match Command::new("tmutil")
        .args(["listlocalsnapshots", "/"])
        .output()
    {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let lines = text
                .lines()
                .filter(|l| l.contains("com.apple.os.update") || l.contains("Snapshot"))
                .count();
            if lines > 0 || text.to_ascii_lowercase().contains("snapshots for volume") {
                // Header alone with no snapshot lines ⇒ NotFound; any named line ⇒ Detected.
                let named = text
                    .lines()
                    .skip(1)
                    .filter(|l| !l.trim().is_empty())
                    .count();
                if named > 0 {
                    OsFileProbeResult::Detected
                } else {
                    OsFileProbeResult::NotFound
                }
            } else if text.trim().is_empty() {
                OsFileProbeResult::NotFound
            } else {
                OsFileProbeResult::Detected
            }
        }
        Ok(_) => OsFileProbeResult::NotReadable,
        Err(_) => {
            // Fall back to MobileBackups path.
            probe_path(Path::new("/Volumes/MobileBackups"))
        }
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn platform_classified_probes() -> (
    OsFileProbeResult,
    OsFileProbeResult,
    OsFileProbeResult,
    OsFileProbeResult,
    OsFileProbeResult,
) {
    (
        OsFileProbeResult::NotApplicable,
        OsFileProbeResult::NotApplicable,
        OsFileProbeResult::NotApplicable,
        OsFileProbeResult::NotApplicable,
        OsFileProbeResult::NotApplicable,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn probe_report_lists_open_residual_classes_and_never_claims_ready() {
        let r = probe_os_privacy_surfaces();
        assert!(r.probes_present);
        assert!(r.swap_snapshot_honesty_present);
        assert!(r.vault_leftover_scan_available);
        assert!(r.crash_sidecar_detect_available);
        assert_eq!(
            r.residual_risk_classes_open,
            vec!["swap", "hibernate", "snapshot", "pagefile", "core_dump"]
        );
        assert_ne!(
            r.pagefile.protection_honesty,
            ProbeHonestyClass::ProtectionMeasured
        );
        assert_ne!(
            r.swap.protection_honesty,
            ProbeHonestyClass::ProtectionMeasured
        );
        assert_ne!(
            r.snapshot.protection_honesty,
            ProbeHonestyClass::ProtectionMeasured
        );
        assert_eq!(
            r.snapshot.protection_honesty,
            ProbeHonestyClass::OwnerOrOsPolicyRequired
        );
        assert!(matches!(
            r.pagefile.existence,
            OsFileProbeResult::Detected
                | OsFileProbeResult::NotFound
                | OsFileProbeResult::NotReadable
                | OsFileProbeResult::NotApplicable
        ));
    }

    #[test]
    fn leftover_scan_detects_planted_crash_sidecar() {
        let root = std::env::temp_dir().join(format!("medscale-043-probe-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("meta.work.sqlite3"), b"leftover").unwrap();
        assert!(scan_vault_work_leftovers(&root));
        assert!(crash_sidecar_leftovers_present(&root));
        let _ = fs::remove_dir_all(&root);
    }
}
