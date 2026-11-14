//! Spec 052 — Linux Landlock FS + TCP + rlimit composition honesty.

use medscale_contracts::os_sandbox::{
    OsSandboxCompositionInventory, OsSandboxDoctorStatus, OsSandboxPlan, try_apply_os_sandbox,
};

#[test]
fn composition_plan_never_claims_platform_qualified() {
    let plan = OsSandboxPlan::linux_landlock_composition_ready_base(vec!["/tmp".into()]);
    assert!(plan.claims_ready_base_measured());
    assert!(!plan.claims_platform_qualified());
    assert!(plan.mechanisms.iter().any(|m| m.contains("rlimit")));
    assert!(plan.mechanisms.iter().any(|m| m.contains("network")));
}

#[test]
fn doctor_linux_composition_axis_honest() {
    let d = OsSandboxDoctorStatus::ready_base();
    assert!(d.linux_landlock_composition_measured);
    assert!(!d.platform_qualified);
    assert!(d.is_honest_ready_base());
    let inv = OsSandboxCompositionInventory::trusted_v1_ready_base();
    assert!(
        inv.axes_ready_base_measured
            .contains(&"linux_landlock_composition".to_owned())
    );
}

#[cfg(not(target_os = "linux"))]
#[test]
fn composition_not_ready_on_non_linux() {
    use medscale_contracts::os_sandbox::OsSandboxApplyError;

    let plan =
        OsSandboxPlan::linux_landlock_composition_ready_base(vec!["C:\\Windows\\Temp".into()]);
    assert_eq!(
        try_apply_os_sandbox(&plan),
        Err(OsSandboxApplyError::NotReadyOnThisHost)
    );
}

#[cfg(target_os = "linux")]
#[test]
fn landlock_composition_measures_fs_net_rlimit() {
    use std::fs;
    use std::io::ErrorKind;
    use std::net::{SocketAddr, TcpStream};
    use std::sync::mpsc;
    use std::time::Duration;

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let base = std::env::temp_dir().join(format!(
            "medscale-052-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let result = (|| -> Result<(), String> {
            fs::create_dir_all(&base).map_err(|e| e.to_string())?;
            let before = medscale_contracts::os_sandbox::linux_rlimit_nofile_soft();
            let plan = OsSandboxPlan::linux_landlock_composition_ready_base(vec![
                base.to_string_lossy().into_owned(),
            ]);
            try_apply_os_sandbox(&plan).map_err(|e| format!("apply: {e:?}"))?;
            fs::write(base.join("ok.txt"), b"ok").map_err(|e| format!("allow write: {e}"))?;
            let deny =
                std::env::temp_dir().join(format!("medscale-052-deny-{}", std::process::id()));
            if fs::write(&deny, b"x").is_ok() {
                let _ = fs::remove_file(&deny);
                return Err("outside allowlist write succeeded".into());
            }
            let addr: SocketAddr = "127.0.0.1:9".parse().unwrap();
            match TcpStream::connect_timeout(&addr, Duration::from_millis(800)) {
                Ok(_) => return Err("tcp connect allowed".into()),
                Err(err) => {
                    let denied = err.kind() == ErrorKind::PermissionDenied
                        || err.raw_os_error() == Some(1)
                        || err.to_string().contains("Operation not permitted")
                        || err.to_string().contains("Permission denied");
                    if !denied {
                        return Err(format!("unexpected connect err {err:?}"));
                    }
                }
            }
            let after = medscale_contracts::os_sandbox::linux_rlimit_nofile_soft();
            if let (Some(b), Some(a)) = (before, after)
                && b > 8
                && a >= b
            {
                return Err(format!("rlimit not lowered {b}->{a}"));
            }
            let _ = fs::remove_dir_all(&base);
            Ok(())
        })();
        let _ = tx.send(result);
    });
    let outcome = rx.recv().expect("composition thread");
    assert!(outcome.is_ok(), "{outcome:?}");
}
