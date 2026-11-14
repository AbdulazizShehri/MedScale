//! Spec 085 qualification harness: a worker that misbehaves on purpose.
//!
//! Core never launches this binary on its own; only tests name it,
//! through an explicit runtime, to prove that Core contains each fault.
//! It reads the request like the real worker, then acts out the mode given
//! as its first argument.

use std::io::{Read, Write};
use std::time::Duration;

use medscale_contracts::analytics::ResultTableDoc;
use medscale_contracts::compute::{
    SandboxMechanism, SandboxReport, SandboxRequirement, WORKER_REQUEST_BYTES_MAX, WorkerOutcome,
    WorkerRequest, WorkerResponse, apply_worker_sandbox, respond,
};
use medscale_contracts::objects::{DigestSha256, OpaqueId};

fn honest(request: &WorkerRequest) -> WorkerResponse {
    let env_var_count = u32::try_from(std::env::vars_os().count()).unwrap_or(u32::MAX);
    respond(request, apply_worker_sandbox(env_var_count))
}

const fn no_sandbox() -> SandboxReport {
    SandboxReport {
        mechanism: SandboxMechanism::None,
        ready_base_applied: false,
        platform_qualified: false,
        env_var_count: 0,
    }
}

fn emit(bytes: &[u8]) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(bytes);
    let _ = out.flush();
}

fn emit_response(response: &WorkerResponse) {
    emit(&serde_json::to_vec(response).unwrap_or_default());
}

fn rewrite_table(response: &mut WorkerResponse, edit: impl FnOnce(&mut ResultTableDoc) -> String) {
    if let WorkerOutcome::Completed {
        output_digest,
        output_json,
    } = &mut response.outcome
    {
        let mut table: ResultTableDoc =
            serde_json::from_str(output_json).unwrap_or(ResultTableDoc {
                columns: vec![],
                rows: vec![],
            });
        *output_json = edit(&mut table);
        *output_digest = DigestSha256::of(output_json.as_bytes());
    }
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let mut raw = Vec::new();
    let _ = std::io::stdin()
        .lock()
        .take(WORKER_REQUEST_BYTES_MAX + 1)
        .read_to_end(&mut raw);
    let Ok(request) = serde_json::from_slice::<WorkerRequest>(&raw) else {
        std::process::exit(2);
    };
    match mode.as_str() {
        "hang" => loop {
            std::thread::sleep(Duration::from_secs(60));
        },
        "slow" => {
            std::thread::sleep(Duration::from_secs(20));
            emit_response(&honest(&request));
        }
        "flood-stdout" => {
            let chunk = vec![b'x'; 64 * 1024];
            let mut out = std::io::stdout().lock();
            while out.write_all(&chunk).is_ok() {}
        }
        "flood-stderr" => {
            let chunk = vec![b'e'; 64 * 1024];
            let mut err = std::io::stderr().lock();
            for _ in 0..64 {
                let _ = err.write_all(&chunk);
            }
            drop(err);
            emit_response(&honest(&request));
        }
        "crash" => std::process::exit(101),
        "partial" => {
            let bytes = serde_json::to_vec(&honest(&request)).unwrap_or_default();
            emit(&bytes[..bytes.len() / 2]);
        }
        "malformed" => emit(b"{\"protocol_version\": 1, \"not\": \"a response\""),
        "trailing" => {
            let mut bytes = serde_json::to_vec(&honest(&request)).unwrap_or_default();
            bytes.extend_from_slice(b"\n{}");
            emit(&bytes);
        }
        "wrong-job" => {
            let mut response = honest(&request);
            response.job_id = OpaqueId::new("compute-job-999999");
            emit_response(&response);
        }
        "wrong-digest" => {
            let mut response = honest(&request);
            if let WorkerOutcome::Completed { output_digest, .. } = &mut response.outcome {
                *output_digest = DigestSha256::of(b"not the output");
            }
            emit_response(&response);
        }
        "noncanonical" => {
            let mut response = honest(&request);
            rewrite_table(&mut response, |table| {
                serde_json::to_string_pretty(table).unwrap_or_default()
            });
            emit_response(&response);
        }
        "bad-shape" => {
            let mut response = honest(&request);
            rewrite_table(&mut response, |table| {
                if let Some(first) = table.rows.first().cloned() {
                    table.rows.push(first);
                }
                serde_json::to_string(table).unwrap_or_default()
            });
            emit_response(&response);
        }
        // Applies no OS mechanism and says so.
        "no-sandbox" => emit_response(&respond(&request, no_sandbox())),
        // Applies none, but computes anyway and claims completion.
        "no-sandbox-completes" => {
            let mut relaxed = request.clone();
            relaxed.sandbox = SandboxRequirement::ProcessIsolationOnly;
            emit_response(&respond(&relaxed, no_sandbox()));
        }
        "env-leak" => {
            let mut response = honest(&request);
            response.sandbox.env_var_count = 3;
            emit_response(&response);
        }
        "claims-qualified" => {
            let mut response = honest(&request);
            response.sandbox.platform_qualified = true;
            emit_response(&response);
        }
        _ => emit_response(&honest(&request)),
    }
}
