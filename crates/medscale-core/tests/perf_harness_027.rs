//! Spec 027 performance harness — measure, do not claim budgets met.
//!
//! Methodology (Trusted V1 delivery plan): explicit warmup, then N=30 timed runs;
//! report p50/p95 in milliseconds. CI asserts the harness runs and emits numbers only.

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::evidence::LexicalRetrieveRequest;
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::CoreFacade;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const WARMUP: usize = 3;
const RUNS_DEFAULT: usize = 30;

fn timed_run_count() -> usize {
    let default = if delivery_plan_scale_requested() {
        5
    } else {
        RUNS_DEFAULT
    };
    std::env::var("MEDSCALE_027_TIMED_RUNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
        .clamp(3, 30)
}
/// READY_BASE timeline event count (delivery-plan target is 10_000; Spec 042 env override).
const TIMELINE_EVENTS_DEFAULT: usize = 64;
/// Default bounded FHIR ingest size for CI (under MAX_INGEST_BYTES). Override with
/// `MEDSCALE_027_FHIR_BYTES` up to 1_000_000 for local near-budget measurements.
const FHIR_INGEST_DEFAULT_BYTES: usize = 128 * 1024;

fn timeline_event_count() -> usize {
    std::env::var("MEDSCALE_027_TIMELINE_EVENTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(TIMELINE_EVENTS_DEFAULT)
        .clamp(1, 10_000)
}

fn delivery_plan_scale_requested() -> bool {
    matches!(
        std::env::var("MEDSCALE_027_DELIVERY_PLAN_SCALE").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}

fn fhir_ingest_target_bytes() -> usize {
    let default = if delivery_plan_scale_requested() {
        1_000_000
    } else {
        FHIR_INGEST_DEFAULT_BYTES
    };
    std::env::var("MEDSCALE_027_FHIR_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
        .clamp(4_096, 1_000_000)
}

/// Spec 045: when set to 10000 (or under delivery-plan scale), use procedural scale corpus.
fn lexical_scale_requested() -> bool {
    match std::env::var("MEDSCALE_027_LEXICAL_SCALE").as_deref() {
        Ok("10000") | Ok("10k") | Ok("10K") => true,
        _ => delivery_plan_scale_requested(),
    }
}

fn lexical_corpus_id() -> &'static str {
    if lexical_scale_requested() {
        medscale_core::SCALE_CORPUS_ID
    } else {
        "synthetic-lexical"
    }
}

fn realm() -> RealmId {
    RealmId::new("realm-027")
}
fn scope() -> AuthorityScopeId {
    AuthorityScopeId::new("scope-027")
}
fn vault() -> VaultId {
    VaultId::new("vault-027")
}

fn req(cap: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("req-027-{}", uuid_like())),
        vault(),
        realm(),
        scope(),
        cap,
        body,
    )
}

fn uuid_like() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static C: AtomicU64 = AtomicU64::new(1);
    C.fetch_add(1, Ordering::SeqCst)
}

fn work_root(name: &str) -> PathBuf {
    let base = PathBuf::from(r"D:\medscale-tmp");
    let root = if base.exists() {
        base
    } else {
        std::env::temp_dir()
    };
    root.join(format!("medscale-027-{name}-{}", std::process::id()))
}

fn repo_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p
}

fn evidence_dir() -> PathBuf {
    if lexical_scale_requested() && delivery_plan_scale_requested() {
        repo_root().join("evidence/045-lexical-10k-corpus")
    } else if delivery_plan_scale_requested() {
        repo_root().join("evidence/042-perf-delivery-plan-scale")
    } else {
        repo_root().join("evidence/027-perf-sbom-release-evidence")
    }
}

fn open_vault(facade: &CoreFacade) -> PathBuf {
    let root = work_root("vault");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    facade
        .dispatch(req(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("cli-027"),
                holder_id_hint: None,
            },
        ))
        .result
        .unwrap();
    facade
        .dispatch(req(
            Capability::OpenSyntheticVault,
            RequestBody::OpenSyntheticVault {
                vault_root: root.to_string_lossy().into_owned(),
            },
        ))
        .result
        .unwrap();
    root
}

fn percentile_ms(sorted: &[Duration], pct: usize) -> f64 {
    assert!(!sorted.is_empty());
    let idx = ((sorted.len() * pct) / 100).min(sorted.len() - 1);
    sorted[idx].as_secs_f64() * 1000.0
}

fn measure_ms<F>(warmup: usize, runs: usize, mut f: F) -> (f64, f64, Vec<f64>)
where
    F: FnMut(),
{
    for _ in 0..warmup {
        f();
    }
    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let t0 = Instant::now();
        f();
        samples.push(t0.elapsed());
    }
    samples.sort();
    let p50 = percentile_ms(&samples, 50);
    let p95 = percentile_ms(&samples, 95);
    let raw: Vec<f64> = samples.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
    (p50, p95, raw)
}

fn seed_timeline(facade: &CoreFacade, subject: &OpaqueId, n: usize) {
    let base = json!({
        "resourceType": "Observation",
        "status": "final",
        "code": { "text": "synthetic-027" },
        "subject": { "reference": "Patient/synthetic-027" },
        "valueQuantity": { "value": 1, "unit": "kg" }
    });
    let bytes = serde_json::to_vec(&base).unwrap();
    let ingested = facade
        .dispatch(req(
            Capability::IngestFhirSynthetic,
            RequestBody::IngestFhirSynthetic {
                media_type: "application/fhir+json".to_owned(),
                bytes: bytes.clone(),
                fhir_version_hint: Some("4.0.1".to_owned()),
                attach_validator_fixture_id: None,
            },
        ))
        .result
        .unwrap();
    let ResponseBody::Ingested { receipt } = ingested else {
        panic!("ingest seed");
    };
    let source_id = receipt.source_id.expect("source");
    let resource: Value = serde_json::from_slice(&bytes).unwrap();

    for i in 0..n {
        let payload = json!({
            "resource": resource,
            "effective_hint": format!("2020-01-{:02}T09:00:00Z", (i % 28) + 1),
            "harness_seq": i,
        });
        let created = facade
            .dispatch(req(
                Capability::CreateProposal,
                RequestBody::CreateProposal {
                    subject_ref: Some(subject.clone()),
                    claim_kind: "observation".to_owned(),
                    payload,
                    evidence_refs: vec![source_id.clone()],
                },
            ))
            .result
            .unwrap();
        let ResponseBody::Created {
            object_id: proposal_id,
        } = created
        else {
            panic!("proposal");
        };
        facade
            .dispatch(req(
                Capability::PromoteProposal,
                RequestBody::PromoteProposal {
                    proposal_id,
                    authorized_by: OpaqueId::new("clinician-027"),
                    subject_ref: subject.clone(),
                },
            ))
            .result
            .unwrap();
    }
}

fn padded_fhir_bytes(target: usize, salt: u64) -> Vec<u8> {
    // Conservative overhead for JSON wrapper around valueString.
    let overhead = 220 + salt.to_string().len();
    let pad_len = target.saturating_sub(overhead);
    let pad = format!("{salt}:{}", "x".repeat(pad_len));
    let v = json!({
        "resourceType": "Patient",
        "id": format!("synthetic-027-padded-{salt}"),
        "active": true,
        "name": [{ "family": "Harness", "given": ["Synthetic"] }],
        "extension": [{
            "url": "https://medscale.local/synthetic/027-pad",
            "valueString": pad
        }]
    });
    let bytes = serde_json::to_vec(&v).unwrap();
    assert!(
        bytes.len() <= 1_048_576,
        "harness FHIR payload {} exceeds MAX_INGEST_BYTES",
        bytes.len()
    );
    bytes
}

fn hardware_note() -> String {
    format!(
        "os={} arch={} cores_hint={:?} hostname_env={:?}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::thread::available_parallelism().ok().map(|n| n.get()),
        std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .ok()
    )
}

fn git_output(args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo_root())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if s.is_empty() { None } else { Some(s) }
}

fn rustc_version_line() -> String {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|| "rustc-version-unavailable".to_owned())
}

fn cargo_lock_sha256() -> String {
    let path = repo_root().join("Cargo.lock");
    match fs::read(&path) {
        Ok(bytes) => {
            let digest = Sha256::digest(&bytes);
            digest.iter().map(|b| format!("{b:02x}")).collect()
        }
        Err(_) => "cargo-lock-unreadable".to_owned(),
    }
}

fn cpu_note() -> String {
    #[cfg(windows)]
    {
        std::env::var("PROCESSOR_IDENTIFIER")
            .or_else(|_| std::env::var("PROCESSOR_ARCHITECTURE"))
            .unwrap_or_else(|_| "cpu-unknown".to_owned())
    }
    #[cfg(not(windows))]
    {
        fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("model name"))
                    .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_owned())
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("arch={}", std::env::consts::ARCH))
    }
}

#[test]
fn perf_harness_027_runs_and_reports_numbers_without_budget_pass() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let vault_root = open_vault(&facade);
    let subject = OpaqueId::new("subject-027");
    let timeline_events = if delivery_plan_scale_requested() {
        // Default near-delivery-plan scale for CI. Override with MEDSCALE_027_TIMELINE_EVENTS
        // up to 10_000 for full delivery-plan host measurement.
        timeline_event_count().max(1_000)
    } else {
        timeline_event_count()
    };
    seed_timeline(&facade, &subject, timeline_events);

    let runs = timed_run_count();
    let (tl_p50, tl_p95, tl_raw) = measure_ms(WARMUP, runs, || {
        let resp = facade
            .dispatch(req(
                Capability::GetTimeline,
                RequestBody::GetTimeline {
                    subject_ref: subject.clone(),
                },
            ))
            .result
            .unwrap();
        assert!(matches!(resp, ResponseBody::Timeline { .. }));
    });

    let (lex_p50, lex_p95, lex_raw) = measure_ms(WARMUP, runs, || {
        let resp = facade
            .dispatch(req(
                Capability::RetrieveLexical,
                RequestBody::RetrieveLexical {
                    request: LexicalRetrieveRequest {
                        query: "hypertension blood pressure".to_owned(),
                        corpus_id: lexical_corpus_id().to_owned(),
                        max_hits: 5,
                        include_retracted: false,
                    },
                },
            ))
            .result
            .unwrap();
        assert!(matches!(resp, ResponseBody::LexicalRetrieve { .. }));
    });

    let fhir_target = fhir_ingest_target_bytes();
    let mut fhir_salt = 0_u64;
    let mut fhir_len = 0_usize;
    let (fhir_p50, fhir_p95, fhir_raw) = measure_ms(WARMUP, runs, || {
        fhir_salt += 1;
        let bytes = padded_fhir_bytes(fhir_target, fhir_salt);
        fhir_len = bytes.len();
        let _ = facade
            .dispatch(req(
                Capability::IngestFhirSynthetic,
                RequestBody::IngestFhirSynthetic {
                    media_type: "application/fhir+json".to_owned(),
                    bytes,
                    fhir_version_hint: Some("4.0.1".to_owned()),
                    attach_validator_fixture_id: None,
                },
            ))
            .result
            .unwrap();
    });

    let lexical_scale = lexical_scale_requested();
    let spec_id = if lexical_scale && delivery_plan_scale_requested() {
        "045-lexical-10k-corpus"
    } else if delivery_plan_scale_requested() {
        "042-perf-delivery-plan-scale"
    } else {
        "027-perf-sbom-release-evidence"
    };
    let report = json!({
        "spec_id": spec_id,
        "schema_version": 2,
        "binding_spec_id": "032-privacy-probes-notice-perf",
        "budgets_claimed_met": false,
        "release_ready": false,
        "binding": {
            "git_sha": git_output(&["rev-parse", "HEAD"]),
            "git_tree": git_output(&["rev-parse", "HEAD^{tree}"]),
            "rustc_version": rustc_version_line(),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "hostname": std::env::var("COMPUTERNAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .ok(),
            "cpu_note": cpu_note(),
            "cargo_lock_sha256": cargo_lock_sha256(),
            "fixture_identity": {
                "timeline_events": timeline_events,
                "lexical_corpus_id": lexical_corpus_id(),
                "lexical_doc_count": if lexical_scale { 10_000 } else { 0 },
                "lexical_query": "hypertension blood pressure",
                "fhir_version_hint": "4.0.1",
                "fhir_ingest_target_bytes": fhir_target,
                "warmup_runs": WARMUP,
                "timed_runs": runs,
                "delivery_plan_scale": delivery_plan_scale_requested(),
                "lexical_scale": lexical_scale
            },
        },
        "methodology": {
            "warmup_runs": WARMUP,
            "timed_runs": runs,
            "percentile_method": "sort ascending; p50=index floor(n*50/100); p95=index floor(n*95/100) clamped to n-1",
            "units": "milliseconds",
            "synthetic_only": true,
            "real_phi": false,
        },
        "hardware_note": hardware_note(),
        "toolchain_note": format!(
            "rustc={}; CARGO_TARGET_DIR={:?}",
            rustc_version_line(),
            std::env::var("CARGO_TARGET_DIR").ok()
        ),
        "delivery_plan_targets_not_claimed": {
            "timeline_10000_events_p95_ms": 250,
            "lexical_10000_records_p95_ms": 300,
            "fhir_1mib_ingest_p95_ms": 500,
        },
        "measured_scale": {
            "timeline_events": timeline_events,
            "lexical_corpus": if lexical_scale {
                "synthetic-lexical-scale@10k.0.0 (procedural 10000 docs)"
            } else {
                "synthetic-lexical (builtin; not 10000 records)"
            },
            "fhir_ingest_bytes": fhir_len,
            "fhir_ingest_bytes_note": "CI default 128KiB; MEDSCALE_027_DELIVERY_PLAN_SCALE=1 selects 1MiB + elevated timeline",
            "delivery_plan_scale": delivery_plan_scale_requested(),
            "lexical_scale": lexical_scale
        },
        "results_ms": {
            "timeline_projection": { "p50": tl_p50, "p95": tl_p95, "samples": tl_raw },
            "lexical_search": { "p50": lex_p50, "p95": lex_p95, "samples": lex_raw },
            "fhir_ingest": { "p50": fhir_p50, "p95": fhir_p95, "samples": fhir_raw },
        },
        "honesty": [
            "Numbers are READY_BASE engineering measurements only.",
            "Do not treat p50/p95 as RELEASE_READY or budget attainment.",
            "Scale may differ from delivery-plan 10k / 1 MiB acceptance targets.",
            "Spec 032 binding fields (git/rustc/lock/fixtures) do not imply budgets_claimed_met.",
            "Spec 045 procedural lexical scale does not claim clinical quality or RELEASE_READY.",
        ],
        "vault_root_ephemeral": vault_root.display().to_string(),
    });

    // Assert harness ran and reported finite numbers — never budget pass/fail.
    assert!(tl_p50.is_finite() && tl_p95.is_finite() && tl_p95 >= tl_p50);
    assert!(lex_p50.is_finite() && lex_p95.is_finite() && lex_p95 >= lex_p50);
    assert!(fhir_p50.is_finite() && fhir_p95.is_finite() && fhir_p95 >= fhir_p50);
    assert!(tl_p50 >= 0.0 && lex_p50 >= 0.0 && fhir_p50 >= 0.0);

    let out_dir = evidence_dir();
    fs::create_dir_all(&out_dir).unwrap();
    let out_name = if delivery_plan_scale_requested() {
        "perf_harness_delivery_plan_scale.json"
    } else {
        "perf_harness_latest.json"
    };
    let out_path = out_dir.join(out_name);
    fs::write(
        &out_path,
        serde_json::to_string_pretty(&report).expect("serialize"),
    )
    .expect("write evidence");
    assert!(Path::new(&out_path).is_file());
    assert!(!report["budgets_claimed_met"].as_bool().unwrap());
    assert!(!report["release_ready"].as_bool().unwrap());
    assert!(
        report["binding"]["cargo_lock_sha256"]
            .as_str()
            .unwrap()
            .len()
            >= 16
    );
    assert_eq!(
        report["binding"]["fixture_identity"]["lexical_corpus_id"],
        lexical_corpus_id()
    );

    let _ = fs::remove_dir_all(&vault_root);
}
