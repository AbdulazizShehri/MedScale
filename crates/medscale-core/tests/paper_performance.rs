//! Paper RQ4 D0 — pre-specified absolute latency measurements.
//!
//! This harness reports observed costs only. It does not assert delivery-plan
//! budgets, release readiness, platform qualification, clinical performance,
//! or a provenance "overhead" relative to an artificial baseline.

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::evidence::LexicalRetrieveRequest;
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::CoreFacade;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

const DEFAULT_TIMELINE_EVENTS: usize = 1_000;
const DEFAULT_FHIR_BYTES: usize = 128 * 1024;
const DEFAULT_WARMUP: usize = 5;
const DEFAULT_RUNS: usize = 100;

fn env_usize(name: &str, default: usize, min: usize, max: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

fn warmup_runs() -> usize {
    env_usize("MEDSCALE_PAPER_PERF_WARMUP", DEFAULT_WARMUP, 0, 100)
}

fn timed_runs() -> usize {
    env_usize("MEDSCALE_PAPER_PERF_RUNS", DEFAULT_RUNS, 1, 500)
}

fn timeline_events() -> usize {
    env_usize(
        "MEDSCALE_PAPER_PERF_TIMELINE_EVENTS",
        DEFAULT_TIMELINE_EVENTS,
        1,
        10_000,
    )
}

fn fhir_target_bytes() -> usize {
    env_usize(
        "MEDSCALE_PAPER_PERF_FHIR_BYTES",
        DEFAULT_FHIR_BYTES,
        4_096,
        1_000_000,
    )
}

fn mode() -> String {
    std::env::var("MEDSCALE_PAPER_PERF_MODE").unwrap_or_else(|_| "manual".to_owned())
}

fn realm() -> RealmId {
    RealmId::new("realm-paper-perf")
}

fn scope() -> AuthorityScopeId {
    AuthorityScopeId::new("scope-paper-perf")
}

fn vault() -> VaultId {
    VaultId::new("vault-paper-perf")
}

fn request(capability: Capability, body: RequestBody) -> AuthorityRequest {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let sequence = COUNTER.fetch_add(1, Ordering::SeqCst);
    AuthorityRequest::new(
        OpaqueId::new(format!("req-paper-perf-{sequence}")),
        vault(),
        realm(),
        scope(),
        capability,
        body,
    )
}

fn repo_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root
}

fn work_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "medscale-paper-perf-{}-{}",
        mode(),
        std::process::id()
    ))
}

fn output_path() -> PathBuf {
    if let Ok(path) = std::env::var("MEDSCALE_PAPER_PERF_OUTPUT") {
        return PathBuf::from(path);
    }
    let runner = std::env::var("RUNNER_OS").unwrap_or_else(|_| std::env::consts::OS.to_owned());
    repo_root()
        .join("paper/artifact/raw/ci")
        .join(format!("d0-{}", mode()))
        .join(format!("perf-{runner}.json"))
}

fn open_vault(facade: &CoreFacade) -> PathBuf {
    let root = work_root();
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create paper perf vault root");
    facade
        .dispatch(request(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("paper-perf-client"),
                holder_id_hint: None,
            },
        ))
        .result
        .expect("acquire lease");
    facade
        .dispatch(request(
            Capability::OpenSyntheticVault,
            RequestBody::OpenSyntheticVault {
                vault_root: root.to_string_lossy().into_owned(),
            },
        ))
        .result
        .expect("open synthetic vault");
    root
}

fn percentile_ms(sorted: &[Duration], pct: usize) -> f64 {
    assert!(!sorted.is_empty());
    let index = ((sorted.len() * pct) / 100).min(sorted.len() - 1);
    sorted[index].as_secs_f64() * 1000.0
}

fn measure_ms<F>(warmup: usize, runs: usize, mut operation: F) -> Vec<Duration>
where
    F: FnMut(),
{
    for _ in 0..warmup {
        operation();
    }
    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let start = Instant::now();
        operation();
        samples.push(start.elapsed());
    }
    samples
}

fn summarize(samples: &[Duration]) -> Value {
    assert!(!samples.is_empty());
    let mut sorted = samples.to_vec();
    sorted.sort();
    let p25 = percentile_ms(&sorted, 25);
    let p50 = percentile_ms(&sorted, 50);
    let p75 = percentile_ms(&sorted, 75);
    let p95 = percentile_ms(&sorted, 95);
    let raw_ms: Vec<f64> = samples
        .iter()
        .map(|duration| duration.as_secs_f64() * 1000.0)
        .collect();
    json!({
        "n": samples.len(),
        "p25_ms": p25,
        "p50_ms": p50,
        "p75_ms": p75,
        "p95_ms": p95,
        "iqr_ms": p75 - p25,
        "min_ms": sorted.first().unwrap().as_secs_f64() * 1000.0,
        "max_ms": sorted.last().unwrap().as_secs_f64() * 1000.0,
        "samples_ms_in_execution_order": raw_ms,
    })
}

fn seed_timeline(facade: &CoreFacade, subject: &OpaqueId, count: usize) {
    let base = json!({
        "resourceType": "Observation",
        "status": "final",
        "code": { "text": "synthetic-paper-perf" },
        "subject": { "reference": "Patient/synthetic-paper-perf" },
        "valueQuantity": { "value": 1, "unit": "kg" }
    });
    let bytes = serde_json::to_vec(&base).expect("serialize seed observation");
    let ingested = facade
        .dispatch(request(
            Capability::IngestFhirSynthetic,
            RequestBody::IngestFhirSynthetic {
                media_type: "application/fhir+json".to_owned(),
                bytes: bytes.clone(),
                fhir_version_hint: Some("4.0.1".to_owned()),
                attach_validator_fixture_id: None,
            },
        ))
        .result
        .expect("ingest seed observation");
    let ResponseBody::Ingested { receipt } = ingested else {
        panic!("expected ingestion receipt");
    };
    let source_id = receipt.source_id.expect("source identity");
    let resource: Value = serde_json::from_slice(&bytes).expect("parse seed observation");

    for index in 0..count {
        let payload = json!({
            "resource": resource,
            "effective_hint": format!("2020-01-{:02}T09:00:00Z", (index % 28) + 1),
            "paper_sequence": index,
        });
        let created = facade
            .dispatch(request(
                Capability::CreateProposal,
                RequestBody::CreateProposal {
                    subject_ref: Some(subject.clone()),
                    claim_kind: "observation".to_owned(),
                    payload,
                    evidence_refs: vec![source_id.clone()],
                },
            ))
            .result
            .expect("create timeline proposal");
        let ResponseBody::Created {
            object_id: proposal_id,
        } = created
        else {
            panic!("expected proposal identity");
        };
        facade
            .dispatch(request(
                Capability::PromoteProposal,
                RequestBody::PromoteProposal {
                    proposal_id,
                    authorized_by: OpaqueId::new("clinician-paper-perf"),
                    subject_ref: subject.clone(),
                },
            ))
            .result
            .expect("promote timeline proposal");
    }
}

fn padded_fhir_bytes(target: usize, salt: u64) -> Vec<u8> {
    let overhead = 240 + salt.to_string().len();
    let padding = target.saturating_sub(overhead);
    let value = json!({
        "resourceType": "Patient",
        "id": format!("synthetic-paper-perf-{salt}"),
        "active": true,
        "name": [{ "family": "Harness", "given": ["Synthetic"] }],
        "extension": [{
            "url": "https://medscale.local/synthetic/paper-perf-pad",
            "valueString": format!("{salt}:{}", "x".repeat(padding))
        }]
    });
    let bytes = serde_json::to_vec(&value).expect("serialize padded FHIR");
    assert!(bytes.len() <= 1_048_576, "paper FHIR payload too large");
    bytes
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(repo_root())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!text.is_empty()).then_some(text)
}

fn cargo_lock_sha256() -> String {
    match fs::read(repo_root().join("Cargo.lock")) {
        Ok(bytes) => Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
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
    #[cfg(target_os = "macos")]
    {
        command_output("sysctl", &["-n", "machdep.cpu.brand_string"])
            .unwrap_or_else(|| format!("arch={}", std::env::consts::ARCH))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|text| {
                text.lines()
                    .find(|line| line.starts_with("model name"))
                    .map(|line| line.split(':').nth(1).unwrap_or("").trim().to_owned())
            })
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| format!("arch={}", std::env::consts::ARCH))
    }
}

#[test]
fn paper_rq4_d0_absolute_latency() {
    let warmup = warmup_runs();
    let runs = timed_runs();
    let timeline_count = timeline_events();
    let fhir_target = fhir_target_bytes();
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let vault_root = open_vault(&facade);
    let subject = OpaqueId::new("subject-paper-perf");
    seed_timeline(&facade, &subject, timeline_count);

    let timeline_samples = measure_ms(warmup, runs, || {
        let response = facade
            .dispatch(request(
                Capability::GetTimeline,
                RequestBody::GetTimeline {
                    subject_ref: subject.clone(),
                },
            ))
            .result
            .expect("timeline projection");
        assert!(matches!(response, ResponseBody::Timeline { .. }));
    });

    let lexical_samples = measure_ms(warmup, runs, || {
        let response = facade
            .dispatch(request(
                Capability::RetrieveLexical,
                RequestBody::RetrieveLexical {
                    request: LexicalRetrieveRequest {
                        query: "hypertension blood pressure".to_owned(),
                        corpus_id: medscale_core::SCALE_CORPUS_ID.to_owned(),
                        max_hits: 5,
                        include_retracted: false,
                    },
                },
            ))
            .result
            .expect("lexical retrieval");
        assert!(matches!(response, ResponseBody::LexicalRetrieve { .. }));
    });

    let mut fhir_salt = 0_u64;
    let mut observed_fhir_bytes = 0_usize;
    let fhir_samples = measure_ms(warmup, runs, || {
        fhir_salt += 1;
        let bytes = padded_fhir_bytes(fhir_target, fhir_salt);
        observed_fhir_bytes = bytes.len();
        facade
            .dispatch(request(
                Capability::IngestFhirSynthetic,
                RequestBody::IngestFhirSynthetic {
                    media_type: "application/fhir+json".to_owned(),
                    bytes,
                    fhir_version_hint: Some("4.0.1".to_owned()),
                    attach_validator_fixture_id: None,
                },
            ))
            .result
            .expect("FHIR ingestion");
    });

    let runner_os = std::env::var("RUNNER_OS").unwrap_or_else(|_| std::env::consts::OS.to_owned());
    let report = json!({
        "schema_version": 1,
        "experiment": "D0-core-absolute-latency",
        "mode": mode(),
        "claim_scope": "descriptive absolute cost only; not overhead, not budget attainment",
        "synthetic_only": true,
        "real_phi": false,
        "release_ready": false,
        "platform_qualified": false,
        "binding": {
            "git_sha": command_output("git", &["rev-parse", "HEAD"]),
            "git_tree": command_output("git", &["rev-parse", "HEAD^{tree}"]),
            "cargo_lock_sha256": cargo_lock_sha256(),
            "rustc": command_output("rustc", &["--version"]),
            "cargo": command_output("cargo", &["--version"]),
            "runner_os": runner_os,
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "cpu_note": cpu_note(),
            "available_parallelism": std::thread::available_parallelism().ok().map(|n| n.get()),
            "process_memory_metric": "not_collected",
        },
        "fixture": {
            "timeline_events": timeline_count,
            "lexical_corpus_id": medscale_core::SCALE_CORPUS_ID,
            "lexical_document_count": 10_000,
            "lexical_query": "hypertension blood pressure",
            "lexical_max_hits": 5,
            "fhir_version_hint": "4.0.1",
            "fhir_target_bytes": fhir_target,
            "fhir_observed_serialized_bytes": observed_fhir_bytes,
        },
        "methodology": {
            "warmup_runs_per_operation": warmup,
            "timed_runs_per_operation": runs,
            "clock": "std::time::Instant monotonic duration",
            "execution_order": ["timeline_projection", "lexical_search", "fhir_ingest"],
            "cold_definition": "first measured invocation after fixture construction in a fresh Cargo test process; not cold OS boot or cold compiler cache",
            "percentile_method": "sort ascending; index=floor(n*p/100), clamped to n-1",
            "units": "milliseconds",
            "pool_across_operating_systems": false,
        },
        "results": {
            "timeline_projection": summarize(&timeline_samples),
            "lexical_search": summarize(&lexical_samples),
            "fhir_ingest": summarize(&fhir_samples),
        },
        "limitations": [
            "Hosted runner latency is descriptive and may vary with runner hardware and load.",
            "No semantically equivalent no-provenance baseline is used, so these measurements are not provenance overhead.",
            "No clinical workload or real PHI is used.",
            "Process RSS is not collected because the current harness lacks a comparable cross-platform method.",
        ],
    });

    for samples in [&timeline_samples, &lexical_samples, &fhir_samples] {
        assert_eq!(samples.len(), runs);
        assert!(
            samples
                .iter()
                .all(|sample| sample.as_secs_f64().is_finite())
        );
    }

    let output = output_path();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).expect("create paper performance evidence directory");
    }
    fs::write(
        &output,
        serde_json::to_string_pretty(&report).expect("serialize paper performance report"),
    )
    .expect("write paper performance report");
    assert!(output.is_file());

    let _ = fs::remove_dir_all(vault_root);
}
