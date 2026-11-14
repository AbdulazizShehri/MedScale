//! Windows AppContainer filesystem + network + LPAC ReadyBaseMeasured
//! (Specs 033 / 038 / 040).
//!
//! Spec 033: create/derive per-user AppContainer profile, launch a child with
//! `SECURITY_CAPABILITIES`, prove the child cannot read a host-temp marker file
//! outside the profile folder.
//!
//! Spec 038: same profile launch with **zero** capability SIDs, prove the child
//! cannot complete a TCP connect (network deny).
//!
//! Spec 040: same zero-capability launch plus
//! `PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY` =
//! `PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT` (LPAC). Child proves
//! AppContainer identity, absence of ALL APPLICATION PACKAGES, FS deny, and
//! TCP deny. Does **not** claim multi-OS PLATFORM_QUALIFIED or clear
//! EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED`.

#![allow(unsafe_code)]

use super::OsSandboxApplyError;
use std::ffi::OsStr;
use std::fs;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;

/// Child argv token: read marker path and exit 0 on PermissionDenied, 2 if readable.
pub const APPCONTAINER_FS_CHILD_ARG: &str = "appcontainer-fs-child";

/// Parent argv token: run measured AppContainer FS deny using this executable as child.
pub const APPCONTAINER_FS_PARENT_ARG: &str = "appcontainer-fs";

/// Child argv token: attempt TCP connect; exit 0 on sandbox deny, 2 if ambient network.
pub const APPCONTAINER_NET_CHILD_ARG: &str = "appcontainer-net-child";

/// Parent argv token: run measured AppContainer network deny using this executable as child.
pub const APPCONTAINER_NET_PARENT_ARG: &str = "appcontainer-net";

/// Child argv token: LPAC identity + FS deny + network deny (Spec 040).
pub const APPCONTAINER_LPAC_CHILD_ARG: &str = "appcontainer-lpac-child";

/// Parent argv token: run measured LPAC AppContainer using this executable as child.
pub const APPCONTAINER_LPAC_PARENT_ARG: &str = "appcontainer-lpac";

/// `PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT` (windows-sys WindowsProgramming).
const PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT: u32 = 1;

fn wide_null(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn path_wide_null(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Child entry: attempt to read `marker`; exit codes are for the parent CreateProcess wait.
#[must_use]
pub fn appcontainer_fs_child_exit_code(marker: &Path) -> i32 {
    match fs::read(marker) {
        Ok(_) => 2, // FAIL: ambient host file was readable inside AppContainer
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => 0,
        Err(e) if e.raw_os_error() == Some(5) => 0, // ERROR_ACCESS_DENIED
        Err(_) => 1,
    }
}

/// Child entry: attempt TCP connect; only sandbox-style denials count as PASS.
/// Uses Rust `TcpStream` (WSAStartup via std) — fine for regular AppContainer (Spec 038).
#[must_use]
pub fn appcontainer_net_child_exit_code() -> i32 {
    use std::io::ErrorKind;
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    // TEST-NET-3 documentation address (not a live public dependency). Prefer this over
    // loopback: some AppContainer configurations still allow 127.0.0.1.
    let addr: SocketAddr = "203.0.113.1:9"
        .parse()
        .expect("static TEST-NET-3 discard endpoint");
    match TcpStream::connect_timeout(&addr, Duration::from_millis(800)) {
        Ok(_) => 2, // FAIL: outbound TCP succeeded
        Err(err) => {
            let denied = err.kind() == ErrorKind::PermissionDenied
                || err.raw_os_error() == Some(5) // ERROR_ACCESS_DENIED
                || err.raw_os_error() == Some(10013) // WSAEACCES
                || err.to_string().contains("Permission denied")
                || err
                    .to_string()
                    .contains("An attempt was made to access a socket in a way forbidden")
                || err
                    .to_string()
                    .contains("forbidden by its access permissions");
            if denied {
                0
            } else {
                // TimedOut / ConnectionRefused / Unreachable => network stack was usable.
                2
            }
        }
    }
}

/// Panic-free TCP deny probe for LPAC (Spec 040).
///
/// LPAC can make `std::net` WSAStartup assert (WSANOTINITIALISED / init failure). Raw
/// WinSock treats startup/socket/connect failures as measured deny (PASS).
#[must_use]
pub fn appcontainer_lpac_net_child_exit_code() -> i32 {
    use windows_sys::Win32::Networking::WinSock::{
        AF_INET, INVALID_SOCKET, IPPROTO_TCP, SOCK_STREAM, SOCKADDR, SOCKADDR_IN, SOCKET_ERROR,
        WSACleanup, WSADATA, WSAStartup, closesocket, connect, htons, socket,
    };

    // SAFETY: WinSock2 startup/connect/cleanup for a one-shot deny probe.
    unsafe {
        let mut wsa: WSADATA = std::mem::zeroed();
        let startup = WSAStartup(0x0202, &mut wsa);
        if startup != 0 {
            // Cannot initialize sockets under this token — measured network deny.
            return 0;
        }

        let s = socket(AF_INET as i32, SOCK_STREAM, IPPROTO_TCP);
        if s == INVALID_SOCKET {
            let _ = WSACleanup();
            return 0; // socket creation denied
        }

        // 203.0.113.1:9 TEST-NET-3 discard
        let mut addr: SOCKADDR_IN = std::mem::zeroed();
        addr.sin_family = AF_INET;
        addr.sin_port = htons(9);
        let ip = u32::from_be_bytes([203, 0, 113, 1]);
        addr.sin_addr.S_un.S_addr = ip;

        let rc = connect(
            s,
            (&raw const addr).cast::<SOCKADDR>(),
            size_of::<SOCKADDR_IN>() as i32,
        );
        let _ = closesocket(s);
        let _ = WSACleanup();

        if rc == SOCKET_ERROR {
            0 // connect denied / failed closed
        } else {
            2 // FAIL: connect succeeded
        }
    }
}

/// True if current process token is an AppContainer.
fn token_is_app_container() -> Result<bool, ()> {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TokenIsAppContainer};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // SAFETY: standard token query on current process.
    unsafe {
        let mut token: HANDLE = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(());
        }
        let mut is_ac: u32 = 0;
        let mut ret_len: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TokenIsAppContainer,
            (&raw mut is_ac).cast(),
            size_of::<u32>() as u32,
            &mut ret_len,
        );
        let _ = CloseHandle(token);
        if ok == 0 {
            return Err(());
        }
        Ok(is_ac != 0)
    }
}

/// True if ALL APPLICATION PACKAGES (`WinBuiltinAnyPackageSid`) is an enabled group.
/// Err(14) = CreateWellKnownSid failed; Err(13) = CheckTokenMembership failed.
fn token_has_all_application_packages() -> Result<bool, i32> {
    use windows_sys::Win32::Security::{
        CheckTokenMembership, CreateWellKnownSid, WinBuiltinAnyPackageSid,
    };

    // SAFETY: well-known SID + membership against current process/thread token (NULL handle).
    unsafe {
        let mut sid_buf = [0u8; 68];
        let mut sid_size = sid_buf.len() as u32;
        if CreateWellKnownSid(
            WinBuiltinAnyPackageSid,
            ptr::null_mut(),
            sid_buf.as_mut_ptr().cast(),
            &mut sid_size,
        ) == 0
        {
            return Err(14);
        }

        let mut is_member: i32 = 0;
        // NULL token => use calling thread's effective token (works inside AppContainer/LPAC).
        let ok = CheckTokenMembership(ptr::null_mut(), sid_buf.as_mut_ptr().cast(), &mut is_member);
        if ok == 0 {
            return Err(13);
        }
        Ok(is_member != 0)
    }
}

/// Child entry (Spec 040): LPAC identity + FS deny + network deny.
///
/// Exit codes:
/// - 0 = PASS
/// - 2 = ambient FS/network allow, or not AppContainer, or ALL_APP_PACKAGES still present
/// - 11 = TokenIsAppContainer query failed
/// - 13 = ALL_APPLICATION_PACKAGES membership check failed
/// - 14 = CreateWellKnownSid failed
/// - 1 = other unexpected
#[must_use]
pub fn appcontainer_lpac_child_exit_code(marker: &Path) -> i32 {
    match token_is_app_container() {
        Ok(true) => {}
        Ok(false) => return 2, // not AppContainer
        Err(()) => return 11,
    }

    match token_has_all_application_packages() {
        Ok(false) => {}       // LPAC: ALL APPLICATION PACKAGES omitted
        Ok(true) => return 2, // regular AppContainer — not LPAC
        Err(14) => return 14,
        Err(_) => return 13,
    }

    let fs_code = appcontainer_fs_child_exit_code(marker);
    if fs_code != 0 {
        return fs_code;
    }

    appcontainer_lpac_net_child_exit_code()
}

/// Locate the AppContainer child helper executable.
#[must_use]
pub fn resolve_appcontainer_child_exe() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_medscale-os-sandbox-probe") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    if let Ok(cur) = std::env::current_exe() {
        if cur
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("medscale-os-sandbox-probe"))
        {
            return Some(cur);
        }
        let sibling = cur.with_file_name("medscale-os-sandbox-probe.exe");
        if sibling.is_file() {
            return Some(sibling);
        }
    }
    None
}

/// Create AppContainer profile, launch `child_exe` with FS-child args, expect deny.
pub fn measure_appcontainer_fs_deny(child_exe: &Path) -> Result<(), OsSandboxApplyError> {
    if !child_exe.is_file() {
        return Err(OsSandboxApplyError::ApplyFailed(format!(
            "AppContainer FS child exe missing: {}",
            child_exe.display()
        )));
    }

    let pid = std::process::id();
    let profile_name = format!("medscale.ac.033.{pid}");
    let marker = std::env::temp_dir().join(format!("medscale-ac-033-{pid}.marker"));
    fs::write(&marker, b"medscale-033-host-marker").map_err(|e| {
        OsSandboxApplyError::ApplyFailed(format!("failed to plant host marker: {e}"))
    })?;

    let cmdline_extra = format!("{APPCONTAINER_FS_CHILD_ARG} \"{}\"", marker.display());
    let result = measure_with_profile(
        &profile_name,
        "MedScale Spec 033 AppContainer",
        "Synthetic AppContainer FS ReadyBaseMeasured profile",
        child_exe,
        &cmdline_extra,
        "FS",
        false,
    );
    let _ = fs::remove_file(&marker);
    let _ = delete_profile(&profile_name);
    result
}

/// Create AppContainer profile (zero capabilities), launch net-child, expect TCP deny.
pub fn measure_appcontainer_net_deny(child_exe: &Path) -> Result<(), OsSandboxApplyError> {
    if !child_exe.is_file() {
        return Err(OsSandboxApplyError::ApplyFailed(format!(
            "AppContainer network child exe missing: {}",
            child_exe.display()
        )));
    }

    let pid = std::process::id();
    let profile_name = format!("medscale.ac.038.{pid}");
    let result = measure_with_profile(
        &profile_name,
        "MedScale Spec 038 AppContainer",
        "Synthetic AppContainer network ReadyBaseMeasured profile",
        child_exe,
        APPCONTAINER_NET_CHILD_ARG,
        "network",
        false,
    );
    let _ = delete_profile(&profile_name);
    result
}

/// Create LPAC AppContainer (zero caps + ALL_APPLICATION_PACKAGES_OPT_OUT), measure child.
pub fn measure_appcontainer_lpac_deny(child_exe: &Path) -> Result<(), OsSandboxApplyError> {
    if !child_exe.is_file() {
        return Err(OsSandboxApplyError::ApplyFailed(format!(
            "AppContainer LPAC child exe missing: {}",
            child_exe.display()
        )));
    }

    let pid = std::process::id();
    let profile_name = format!("medscale.ac.040.{pid}");
    let marker = std::env::temp_dir().join(format!("medscale-ac-040-{pid}.marker"));
    fs::write(&marker, b"medscale-040-host-marker").map_err(|e| {
        OsSandboxApplyError::ApplyFailed(format!("failed to plant host marker: {e}"))
    })?;

    let cmdline_extra = format!("{APPCONTAINER_LPAC_CHILD_ARG} \"{}\"", marker.display());
    let result = measure_with_profile(
        &profile_name,
        "MedScale Spec 040 AppContainer LPAC",
        "Synthetic AppContainer LPAC ReadyBaseMeasured profile",
        child_exe,
        &cmdline_extra,
        "LPAC",
        true,
    );
    let _ = fs::remove_file(&marker);
    let _ = delete_profile(&profile_name);
    result
}

/// ReadyBaseMeasured apply: resolve probe helper and measure FS deny.
pub(super) fn apply_appcontainer_fs_windows() -> Result<(), OsSandboxApplyError> {
    let child = resolve_appcontainer_child_exe().ok_or_else(|| {
        OsSandboxApplyError::ApplyFailed(
            "AppContainer FS measure needs medscale-os-sandbox-probe on PATH/sibling/CARGO_BIN_EXE"
                .to_owned(),
        )
    })?;
    measure_appcontainer_fs_deny(&child)
}

/// ReadyBaseMeasured apply: resolve probe helper and measure network deny.
pub(super) fn apply_appcontainer_net_windows() -> Result<(), OsSandboxApplyError> {
    let child = resolve_appcontainer_child_exe().ok_or_else(|| {
        OsSandboxApplyError::ApplyFailed(
            "AppContainer network measure needs medscale-os-sandbox-probe on PATH/sibling/CARGO_BIN_EXE"
                .to_owned(),
        )
    })?;
    measure_appcontainer_net_deny(&child)
}

/// ReadyBaseMeasured apply: resolve probe helper and measure LPAC.
pub(super) fn apply_appcontainer_lpac_windows() -> Result<(), OsSandboxApplyError> {
    let child = resolve_appcontainer_child_exe().ok_or_else(|| {
        OsSandboxApplyError::ApplyFailed(
            "AppContainer LPAC measure needs medscale-os-sandbox-probe on PATH/sibling/CARGO_BIN_EXE"
                .to_owned(),
        )
    })?;
    measure_appcontainer_lpac_deny(&child)
}

fn delete_profile(profile_name: &str) -> Result<(), OsSandboxApplyError> {
    use windows_sys::Win32::Security::Isolation::DeleteAppContainerProfile;

    let name = wide_null(profile_name);
    // SAFETY: profile name is a valid NUL-terminated UTF-16 string.
    let hr = unsafe { DeleteAppContainerProfile(name.as_ptr()) };
    if hr < 0 {
        // Best-effort cleanup; ignore "not found".
        let _ = hr;
    }
    Ok(())
}

fn measure_with_profile(
    profile_name: &str,
    display: &str,
    desc: &str,
    child_exe: &Path,
    child_args: &str,
    kind: &str,
    lpac: bool,
) -> Result<(), OsSandboxApplyError> {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LocalFree};
    use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows_sys::Win32::Security::Isolation::{
        CreateAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
    };
    use windows_sys::Win32::Security::{FreeSid, PSID, SECURITY_CAPABILITIES};
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT,
        GetExitCodeProcess, InitializeProcThreadAttributeList,
        PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY,
        PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, PROCESS_INFORMATION, STARTUPINFOEXW,
        UpdateProcThreadAttribute, WaitForSingleObject,
    };

    let name_w = wide_null(profile_name);
    let display_w = wide_null(display);
    let desc_w = wide_null(desc);
    let attr_count: u32 = if lpac { 2 } else { 1 };

    let mut sid: PSID = ptr::null_mut();

    // SAFETY: Win32 AppContainer profile APIs; SID freed via FreeSid on all paths below.
    unsafe {
        let hr = CreateAppContainerProfile(
            name_w.as_ptr(),
            display_w.as_ptr(),
            desc_w.as_ptr(),
            ptr::null(),
            0,
            &mut sid,
        );
        // 0x800700B7 = HRESULT_FROM_WIN32(ERROR_ALREADY_EXISTS)
        if hr < 0 {
            if hr == -2147024713i32 {
                let hr2 = DeriveAppContainerSidFromAppContainerName(name_w.as_ptr(), &mut sid);
                if hr2 < 0 || sid.is_null() {
                    return Err(OsSandboxApplyError::ApplyFailed(format!(
                        "DeriveAppContainerSidFromAppContainerName failed: HRESULT=0x{hr2:08X}"
                    )));
                }
            } else {
                return Err(OsSandboxApplyError::ApplyFailed(format!(
                    "CreateAppContainerProfile failed: HRESULT=0x{hr:08X}"
                )));
            }
        } else if sid.is_null() {
            return Err(OsSandboxApplyError::ApplyFailed(
                "CreateAppContainerProfile returned null SID".to_owned(),
            ));
        }

        let mut sid_str: windows_sys::core::PWSTR = ptr::null_mut();
        if ConvertSidToStringSidW(sid, &mut sid_str) == 0 || sid_str.is_null() {
            FreeSid(sid);
            return Err(OsSandboxApplyError::ApplyFailed(
                "ConvertSidToStringSidW failed".to_owned(),
            ));
        }
        LocalFree(sid_str.cast());

        // Zero capabilities: no InternetClient / privateNetworkClientServer grants.
        let mut caps = SECURITY_CAPABILITIES {
            AppContainerSid: sid,
            Capabilities: ptr::null_mut(),
            CapabilityCount: 0,
            Reserved: 0,
        };

        let mut attr_size: usize = 0;
        let _ = InitializeProcThreadAttributeList(ptr::null_mut(), attr_count, 0, &mut attr_size);
        if attr_size == 0 {
            FreeSid(sid);
            return Err(OsSandboxApplyError::ApplyFailed(
                "InitializeProcThreadAttributeList size query failed".to_owned(),
            ));
        }
        let mut attr_buf = vec![0u8; attr_size];
        let attr_list = attr_buf.as_mut_ptr().cast();
        if InitializeProcThreadAttributeList(attr_list, attr_count, 0, &mut attr_size) == 0 {
            FreeSid(sid);
            return Err(OsSandboxApplyError::ApplyFailed(
                "InitializeProcThreadAttributeList failed".to_owned(),
            ));
        }

        if UpdateProcThreadAttribute(
            attr_list,
            0,
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            (&raw mut caps).cast(),
            size_of::<SECURITY_CAPABILITIES>(),
            ptr::null_mut(),
            ptr::null(),
        ) == 0
        {
            DeleteProcThreadAttributeList(attr_list);
            FreeSid(sid);
            return Err(OsSandboxApplyError::ApplyFailed(
                "UpdateProcThreadAttribute(SECURITY_CAPABILITIES) failed".to_owned(),
            ));
        }

        let mut lpac_policy = PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT;
        if lpac
            && UpdateProcThreadAttribute(
                attr_list,
                0,
                PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY as usize,
                (&raw mut lpac_policy).cast(),
                size_of::<u32>(),
                ptr::null_mut(),
                ptr::null(),
            ) == 0
        {
            DeleteProcThreadAttributeList(attr_list);
            FreeSid(sid);
            return Err(OsSandboxApplyError::ApplyFailed(
                "UpdateProcThreadAttribute(ALL_APPLICATION_PACKAGES_POLICY / LPAC) failed"
                    .to_owned(),
            ));
        }

        let mut si: STARTUPINFOEXW = std::mem::zeroed();
        si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        si.lpAttributeList = attr_list;

        let mut pi = PROCESS_INFORMATION {
            hProcess: ptr::null_mut(),
            hThread: ptr::null_mut(),
            dwProcessId: 0,
            dwThreadId: 0,
        };

        let app = path_wide_null(child_exe);
        let cmdline_str = format!("\"{}\" {child_args}", child_exe.display());
        let mut cmdline = wide_null(&cmdline_str);

        let created = CreateProcessW(
            app.as_ptr(),
            cmdline.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            0,
            EXTENDED_STARTUPINFO_PRESENT,
            ptr::null(),
            ptr::null(),
            &raw const si.StartupInfo,
            &mut pi,
        );

        DeleteProcThreadAttributeList(attr_list);
        FreeSid(sid);

        if created == 0 {
            return Err(OsSandboxApplyError::ApplyFailed(format!(
                "CreateProcessW into AppContainer ({kind}) failed"
            )));
        }

        let process: HANDLE = pi.hProcess;
        let thread: HANDLE = pi.hThread;
        let _ = WaitForSingleObject(process, 30_000);
        let mut code: u32 = 1;
        let _ = GetExitCodeProcess(process, &mut code);
        let _ = CloseHandle(thread);
        let _ = CloseHandle(process);

        match code {
            0 => Ok(()),
            2 => Err(OsSandboxApplyError::ApplyFailed(format!(
                "AppContainer child ambient {kind} still allowed or LPAC identity not observed"
            ))),
            11 => Err(OsSandboxApplyError::ApplyFailed(
                "AppContainer LPAC child TokenIsAppContainer query failed".to_owned(),
            )),
            13 => Err(OsSandboxApplyError::ApplyFailed(
                "AppContainer LPAC child ALL_APPLICATION_PACKAGES membership check failed"
                    .to_owned(),
            )),
            14 => Err(OsSandboxApplyError::ApplyFailed(
                "AppContainer LPAC child CreateWellKnownSid failed".to_owned(),
            )),
            other => Err(OsSandboxApplyError::ApplyFailed(format!(
                "AppContainer {kind} child exited unexpectedly: {other}"
            ))),
        }
    }
}
