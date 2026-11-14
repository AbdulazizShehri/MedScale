//! Spec 030/031/033/038/040 probe: apply OS sandbox READY_BASE, then prove a denied ambient capability.
//!
//! Exit codes:
//! - 0: apply succeeded and measured deny observed (PASS)
//! - 1: apply failed
//! - 2: apply succeeded but ambient capability was still allowed (FAIL)
//! - 3: not applicable on this host OS
//!
//! Linux seccomp modes (Spec 053):
//! - `seccomp-child`: child entry: install filter, prove write, attempt socket (dies SIGSYS)
//! - `seccomp-composition`: parent measure via child SIGSYS death
//!
//! Windows AppContainer modes:
//! - `appcontainer-fs-child <marker>`: child entry inside AppContainer (FS)
//! - `appcontainer-fs`: parent measure FS deny
//! - `appcontainer-net-child`: child entry inside AppContainer (network)
//! - `appcontainer-net`: parent measure network deny
//! - `appcontainer-lpac-child <marker>`: child entry inside LPAC (identity+FS+net)
//! - `appcontainer-lpac`: parent measure LPAC

fn main() {
    let args: Vec<String> = std::env::args().collect();

    #[cfg(windows)]
    {
        use medscale_contracts::os_sandbox::{
            APPCONTAINER_FS_CHILD_ARG, APPCONTAINER_FS_PARENT_ARG, APPCONTAINER_LPAC_CHILD_ARG,
            APPCONTAINER_LPAC_PARENT_ARG, APPCONTAINER_NET_CHILD_ARG, APPCONTAINER_NET_PARENT_ARG,
            OsSandboxPlan, appcontainer_fs_child_exit_code, appcontainer_lpac_child_exit_code,
            appcontainer_net_child_exit_code, measure_appcontainer_fs_deny,
            measure_appcontainer_lpac_deny, measure_appcontainer_net_deny, try_apply_os_sandbox,
        };
        use std::path::Path;
        use std::process::Command;

        if args.get(1).map(String::as_str) == Some(APPCONTAINER_FS_CHILD_ARG) {
            let marker = args.get(2).map(Path::new).unwrap_or_else(|| Path::new(""));
            std::process::exit(appcontainer_fs_child_exit_code(marker));
        }

        if args.get(1).map(String::as_str) == Some(APPCONTAINER_NET_CHILD_ARG) {
            std::process::exit(appcontainer_net_child_exit_code());
        }

        if args.get(1).map(String::as_str) == Some(APPCONTAINER_LPAC_CHILD_ARG) {
            let marker = args.get(2).map(Path::new).unwrap_or_else(|| Path::new(""));
            std::process::exit(appcontainer_lpac_child_exit_code(marker));
        }

        if args.get(1).map(String::as_str) == Some(APPCONTAINER_FS_PARENT_ARG) {
            let child = std::env::current_exe().unwrap_or_default();
            match measure_appcontainer_fs_deny(&child) {
                Ok(()) => {
                    eprintln!("OK: AppContainer FS deny measured");
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("apply/measure failed: {e:?}");
                    std::process::exit(1);
                }
            }
        }

        if args.get(1).map(String::as_str) == Some(APPCONTAINER_NET_PARENT_ARG) {
            let child = std::env::current_exe().unwrap_or_default();
            match measure_appcontainer_net_deny(&child) {
                Ok(()) => {
                    eprintln!("OK: AppContainer network deny measured");
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("apply/measure failed: {e:?}");
                    std::process::exit(1);
                }
            }
        }

        if args.get(1).map(String::as_str) == Some(APPCONTAINER_LPAC_PARENT_ARG) {
            let child = std::env::current_exe().unwrap_or_default();
            match measure_appcontainer_lpac_deny(&child) {
                Ok(()) => {
                    eprintln!("OK: AppContainer LPAC deny measured");
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("apply/measure failed: {e:?}");
                    std::process::exit(1);
                }
            }
        }

        let plan = OsSandboxPlan::windows_job_object_ready_base();
        if let Err(e) = try_apply_os_sandbox(&plan) {
            eprintln!("apply failed: {e:?}");
            std::process::exit(1);
        }

        // ACTIVE_PROCESS=1: this process is the sole member; CreateProcess must fail.
        match Command::new("cmd.exe").args(["/C", "exit", "0"]).status() {
            Ok(status) => {
                eprintln!(
                    "FAIL: child process was allowed (exit={status:?}); Job Object did not deny spawn"
                );
                std::process::exit(2);
            }
            Err(err) => {
                eprintln!("OK: child process denied ({err})");
                std::process::exit(0);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        use medscale_contracts::os_sandbox::{OsSandboxPlan, try_apply_os_sandbox};
        use std::io::ErrorKind;
        use std::net::{SocketAddr, TcpStream};
        use std::time::Duration;

        let _ = args;

        let plan = OsSandboxPlan::macos_seatbelt_ready_base();
        if let Err(e) = try_apply_os_sandbox(&plan) {
            eprintln!("apply failed: {e:?}");
            std::process::exit(1);
        }

        // Seatbelt (deny network*): connect must fail as EPERM / PermissionDenied.
        // ConnectionRefused would mean the network stack was still reachable (measured FAIL).
        let addr: SocketAddr = "127.0.0.1:9".parse().expect("static loopback discard port");
        match TcpStream::connect_timeout(&addr, Duration::from_millis(800)) {
            Ok(_) => {
                eprintln!("FAIL: TCP connect succeeded; Seatbelt did not deny network");
                std::process::exit(2);
            }
            Err(err) => {
                let denied = err.kind() == ErrorKind::PermissionDenied
                    || err.raw_os_error() == Some(1) // EPERM
                    || err.to_string().contains("Operation not permitted")
                    || err.to_string().contains("Permission denied");
                if denied {
                    eprintln!("OK: network denied ({err})");
                    std::process::exit(0);
                }
                eprintln!(
                    "FAIL: connect error was not a sandbox deny (got {err:?}); network may still be ambient"
                );
                std::process::exit(2);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        use medscale_contracts::os_sandbox::{
            OsSandboxPlan, SECCOMP_CHILD_ARG, SECCOMP_PARENT_ARG, linux_rlimit_nofile_soft,
            measure_seccomp_composition, seccomp_child_exit_code, try_apply_os_sandbox,
        };
        use std::fs;
        use std::io::ErrorKind;
        use std::net::{SocketAddr, TcpStream};
        use std::time::Duration;

        let mode = args
            .get(1)
            .map(String::as_str)
            .unwrap_or("landlock-composition");
        if mode == SECCOMP_CHILD_ARG {
            std::process::exit(seccomp_child_exit_code());
        }
        if mode == SECCOMP_PARENT_ARG {
            let child = std::env::current_exe().unwrap_or_default();
            match measure_seccomp_composition(&child) {
                Ok(()) => {
                    eprintln!("OK: seccomp-bpf allowlist composition measured (SIGSYS deny)");
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("apply/measure failed: {e:?}");
                    std::process::exit(1);
                }
            }
        }
        if mode != "landlock-composition" {
            eprintln!(
                "usage: medscale-os-sandbox-probe landlock-composition | {SECCOMP_PARENT_ARG}"
            );
            std::process::exit(1);
        }

        let base = std::env::temp_dir().join(format!(
            "medscale-ll-comp-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        if let Err(e) = fs::create_dir_all(&base) {
            eprintln!("temp dir failed: {e}");
            std::process::exit(1);
        }
        let allow = base.to_string_lossy().into_owned();
        let before = linux_rlimit_nofile_soft();
        let plan = OsSandboxPlan::linux_landlock_composition_ready_base(vec![allow]);
        if let Err(e) = try_apply_os_sandbox(&plan) {
            eprintln!("apply failed: {e:?}");
            let _ = fs::remove_dir_all(&base);
            std::process::exit(1);
        }

        let ok_path = base.join("ok.txt");
        if let Err(e) = fs::write(&ok_path, b"ok") {
            eprintln!("FAIL: allowlist write denied ({e})");
            let _ = fs::remove_dir_all(&base);
            std::process::exit(2);
        }

        let deny_path =
            std::env::temp_dir().join(format!("medscale-ll-comp-deny-{}", std::process::id()));
        if fs::write(&deny_path, b"x").is_ok() {
            let _ = fs::remove_file(&deny_path);
            eprintln!("FAIL: outside-allowlist write succeeded");
            let _ = fs::remove_dir_all(&base);
            std::process::exit(2);
        }

        let addr: SocketAddr = "127.0.0.1:9".parse().expect("static loopback discard port");
        match TcpStream::connect_timeout(&addr, Duration::from_millis(800)) {
            Ok(_) => {
                eprintln!("FAIL: TCP connect succeeded; Landlock net did not deny");
                let _ = fs::remove_dir_all(&base);
                std::process::exit(2);
            }
            Err(err) => {
                let denied = err.kind() == ErrorKind::PermissionDenied
                    || err.raw_os_error() == Some(1)
                    || err.to_string().contains("Operation not permitted")
                    || err.to_string().contains("Permission denied");
                if !denied {
                    eprintln!("FAIL: connect error was not a sandbox deny (got {err:?})");
                    let _ = fs::remove_dir_all(&base);
                    std::process::exit(2);
                }
            }
        }

        let after = linux_rlimit_nofile_soft();
        match (before, after) {
            (Some(b), Some(a)) if b > 8 && a >= b => {
                eprintln!("FAIL: RLIMIT_NOFILE not lowered (before={b} after={a})");
                let _ = fs::remove_dir_all(&base);
                std::process::exit(2);
            }
            _ => {}
        }

        eprintln!("OK: Landlock FS+net+rlimit composition measured");
        let _ = fs::remove_dir_all(&base);
        std::process::exit(0);
    }

    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        let _ = args;
        eprintln!("medscale-os-sandbox-probe: not applicable on this host");
        std::process::exit(3);
    }
}
