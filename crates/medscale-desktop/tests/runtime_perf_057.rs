//! Spec 057 runtime performance coverage.
//! Measures only; never claims Trusted V1 budget attainment.

use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const WARMUP: usize = 3;
const RUNS_DEFAULT: usize = 30;
const IDLE_MS_DEFAULT: u64 = 1_500;
const RSS_SAMPLES: usize = 5;
const WINDOWS_RSS_PROBE_MIN_IDLE_MS: u64 = 10_000;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_medscale-desktop"))
}

fn timed_runs() -> usize {
    std::env::var("MEDSCALE_057_TIMED_RUNS")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(RUNS_DEFAULT)
        .clamp(3, 30)
}

fn idle_ms() -> u64 {
    std::env::var("MEDSCALE_057_IDLE_MS")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(IDLE_MS_DEFAULT)
        .clamp(500, 10_000)
}

fn rss_probe_idle_ms(requested_ms: u64) -> u64 {
    if cfg!(windows) {
        requested_ms.max(WINDOWS_RSS_PROBE_MIN_IDLE_MS)
    } else {
        requested_ms
    }
}

fn percentile_ms(sorted: &[Duration], pct: usize) -> f64 {
    assert!(!sorted.is_empty());
    let index = ((sorted.len() * pct) / 100).min(sorted.len() - 1);
    sorted[index].as_secs_f64() * 1_000.0
}

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn cargo_lock_sha256() -> String {
    let bytes = fs::read(repo_root().join("Cargo.lock")).expect("read Cargo.lock");
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn rustc_version() -> String {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_else(|| "rustc-version-unavailable".to_owned())
}

fn smoke_once() -> Duration {
    let started = Instant::now();
    let output = Command::new(binary())
        .arg("--smoke")
        .output()
        .expect("run desktop smoke");
    let elapsed = started.elapsed();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("medscale-desktop smoke ok"));
    elapsed
}

#[cfg(unix)]
fn sample_rss_kib(pid: u32) -> u64 {
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .expect("sample RSS with ps");
    assert!(output.status.success(), "ps RSS sample failed");
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .expect("parse ps RSS KiB")
}

fn parse_tasklist_rss_kib(text: &str, pid: u32) -> Option<u64> {
    text.lines().find_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with("INFO:") {
            return None;
        }
        let columns = line.trim_matches('"').split("\",\"").collect::<Vec<_>>();
        if columns.len() < 5 || columns.get(1)?.parse::<u32>().ok()? != pid {
            return None;
        }
        let digits = columns
            .last()?
            .chars()
            .filter(char::is_ascii_digit)
            .collect::<String>();
        if digits.is_empty() {
            return None;
        }
        digits.parse::<u64>().ok()
    })
}

#[cfg(windows)]
fn sample_rss_kib(pid: u32) -> u64 {
    for _ in 0..5 {
        let filter = format!("PID eq {pid}");
        let output = Command::new("tasklist")
            .args(["/FI", filter.as_str(), "/FO", "CSV", "/NH"])
            .output()
            .expect("sample RSS with tasklist");
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(rss_kib) = parse_tasklist_rss_kib(&stdout, pid) {
                return rss_kib;
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("tasklist did not yield a valid RSS sample for PID {pid}");
}

fn measure_idle_rss_kib() -> Vec<u64> {
    let idle_ms = rss_probe_idle_ms(idle_ms());
    let mut child = Command::new(binary())
        .args(["--perf-idle-ms", &idle_ms.to_string()])
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn desktop idle probe");
    let stdout = child.stdout.take().expect("idle probe stdout");
    let mut reader = BufReader::new(stdout);
    let mut ready = String::new();
    reader.read_line(&mut ready).expect("read idle readiness");
    assert_eq!(ready.trim(), "medscale-desktop perf-idle-ready");

    let mut samples = Vec::with_capacity(RSS_SAMPLES);
    for _ in 0..RSS_SAMPLES {
        thread::sleep(Duration::from_millis(100));
        samples.push(sample_rss_kib(child.id()));
    }
    drop(reader);
    let status = child.wait().expect("wait idle probe");
    assert!(status.success());
    samples
}

fn evidence_dir() -> PathBuf {
    repo_root().join("target/medscale-evidence/057-release-qualification-residual-integrity")
}

#[test]
fn rss_probe_window_covers_windows_process_enumeration_latency() {
    let requested = 1_500;
    let effective = rss_probe_idle_ms(requested);
    if cfg!(windows) {
        assert_eq!(effective, WINDOWS_RSS_PROBE_MIN_IDLE_MS);
    } else {
        assert_eq!(effective, requested);
    }
}

#[test]
fn tasklist_rss_parser_tolerates_transient_empty_output() {
    assert_eq!(
        parse_tasklist_rss_kib(
            "INFO: No tasks are running which match the specified criteria.\r\n",
            4242
        ),
        None
    );
    assert_eq!(
        parse_tasklist_rss_kib(
            "\"medscale-desktop.exe\",\"4242\",\"Console\",\"1\",\"10,240 K\"\r\n",
            4242
        ),
        Some(10_240)
    );
    assert_eq!(
        parse_tasklist_rss_kib(
            "\"medscale-desktop.exe\",\"99\",\"Console\",\"1\",\"10,240 K\"\r\n",
            4242
        ),
        None
    );
}

#[test]
fn runtime_perf_057_measures_without_claiming_attainment() {
    for _ in 0..WARMUP {
        let _ = smoke_once();
    }

    let runs = timed_runs();
    let mut launch_samples: Vec<Duration> = (0..runs).map(|_| smoke_once()).collect();
    launch_samples.sort();
    let launch_p50 = percentile_ms(&launch_samples, 50);
    let launch_p95 = percentile_ms(&launch_samples, 95);
    let launch_raw: Vec<f64> = launch_samples
        .iter()
        .map(|sample| sample.as_secs_f64() * 1_000.0)
        .collect();

    let rss_samples = measure_idle_rss_kib();
    let idle_rss_kib = *rss_samples.iter().max().expect("RSS samples");
    assert!(launch_p50.is_finite() && launch_p95.is_finite());
    assert!(launch_p95 >= launch_p50);
    assert!(idle_rss_kib > 0);

    let report = json!({
        "spec_id": "057-release-qualification-residual-integrity",
        "schema_version": 1,
        "budgets_claimed_met": false,
        "release_ready": false,
        "binding": {
            "git_sha": git_output(&["rev-parse", "HEAD"]),
            "git_tree": git_output(&["rev-parse", "HEAD^{tree}"]),
            "cargo_lock_sha256": cargo_lock_sha256(),
            "rustc_version": rustc_version(),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "binary": binary().display().to_string(),
        },
        "methodology": {
            "warmup_runs": WARMUP,
            "timed_runs": runs,
            "idle_probe_ms_requested": idle_ms(),
            "idle_probe_ms_effective": rss_probe_idle_ms(idle_ms()),
            "idle_rss_samples": RSS_SAMPLES,
            "synthetic_only": true,
            "real_phi": false,
        },
        "delivery_plan_targets_not_claimed": {
            "cold_model_free_launch_p95_ms": 2000,
            "ui_interaction_response_ms": 100,
            "model_free_idle_memory_mib": 250,
        },
        "results": {
            "desktop_scaffold_cold_launch_ms": {
                "p50": launch_p50,
                "p95": launch_p95,
                "samples": launch_raw,
            },
            "desktop_scaffold_idle_rss": {
                "max_kib": idle_rss_kib,
                "max_mib": (idle_rss_kib as f64) / 1024.0,
                "samples_kib": rss_samples,
            },
        },
        "honesty": {
            "cold_launch_scope": "desktop scaffold process + model-free doctor init",
            "idle_memory_scope": "desktop scaffold engineering idle probe",
            "final_ui_response": "FINAL_UI_PRESENT_EXTERNAL_INTERACTION_MEASUREMENT_REQUIRED",
            "final_shell_remeasurement_required": true,
            "qualified_hardware_required_for_attainment": true,
        },
    });

    assert_eq!(report["budgets_claimed_met"], false);
    assert_eq!(report["release_ready"], false);
    assert_eq!(
        report["honesty"]["final_ui_response"],
        "FINAL_UI_PRESENT_EXTERNAL_INTERACTION_MEASUREMENT_REQUIRED"
    );

    let out_dir = evidence_dir();
    fs::create_dir_all(&out_dir).expect("create runtime perf evidence dir");
    let out_path = out_dir.join(format!("runtime_perf_{}.json", std::env::consts::OS));
    fs::write(
        &out_path,
        serde_json::to_string_pretty(&report).expect("serialize report"),
    )
    .expect("write runtime perf report");
    assert!(Path::new(&out_path).is_file());
}
