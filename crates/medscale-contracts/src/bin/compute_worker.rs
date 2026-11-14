//! Spec 085 MedScale Compute worker.
//!
//! Reads one `WorkerRequest` from stdin, applies this OS's
//! `ReadyBaseMeasured` mechanism to itself, runs the one admitted job kind
//! the request names, and writes one `WorkerResponse` to stdout. It links
//! only `medscale-contracts`: it cannot open a vault, reach Core or use a
//! network client. It spawns nothing and reads no file.
//!
//! Exit codes: 0 after writing a response; 2 when the request cannot be
//! read or parsed (Core records the job as `failed`).

use std::io::{Read, Write};

use medscale_contracts::compute::{
    WORKER_REQUEST_BYTES_MAX, WorkerRequest, apply_worker_sandbox, respond,
};

fn main() {
    let env_var_count = u32::try_from(std::env::vars_os().count()).unwrap_or(u32::MAX);
    let mut raw = Vec::new();
    let read = std::io::stdin()
        .lock()
        .take(WORKER_REQUEST_BYTES_MAX + 1)
        .read_to_end(&mut raw);
    if read.is_err() || raw.len() as u64 > WORKER_REQUEST_BYTES_MAX {
        std::process::exit(2);
    }
    let Ok(request) = serde_json::from_slice::<WorkerRequest>(&raw) else {
        std::process::exit(2);
    };
    drop(raw);
    let sandbox = apply_worker_sandbox(env_var_count);
    let response = respond(&request, sandbox);
    let Ok(bytes) = serde_json::to_vec(&response) else {
        std::process::exit(2);
    };
    let mut out = std::io::stdout().lock();
    if out.write_all(&bytes).and_then(|()| out.flush()).is_err() {
        std::process::exit(2);
    }
}
