# CLOSURE — Spec 085 MedScale Compute (local bounded worker foundation)

## Terminal truth

```text
SPEC_085_CLOSED_CANONICAL=true
MERGE_SHA=91021e21ecf2695c9b20dad3d54ed9ca6fa70774 (PR #149)
FINAL_HEAD=68c20d59cec97781f352d27a26ff90c94bf8a3b3
EXACT_HEAD_CI=36203947745 (6/6)
CODE_HEAD_CI=36199048976 (on 3bd00d1; ubuntu 874 passed / 0 failed /
  1 ignored, windows 871 passed / 0 failed / 1 ignored with Clippy and
  Test green before a documentation push cancelled the job, macOS green)
POST_MERGE_MAIN_CI=36208910691 (6/6 on 91021e2)
BASE=434d1c7 (PR #150 merge, Spec 084 closure; post-main run 36203706536 6/6)
REVIEW_POLICY=FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22 (no external reviewer;
  deterministic scope record in EXACT_RANGE_REVIEW.md; security challenge in
  SECURITY.md)
JOB_KINDS=column_profile v1, sorted_projection v1 (closed)
ARBITRARY_CODE=none (no shell, Python, R, script, plugin or worker path)
NETWORK_PATH=none
SANDBOX=worker self-applies its OS ReadyBaseMeasured mechanism
  (Linux Landlock composition, Windows Job Object, macOS Seatbelt);
  platform_qualified=false
DEPENDENCIES_ADDED=none
STORAGE_SCHEMA=v14 (v13->v14 additive; 13 restore tamper cases)
CONTRACTS=PASS (6)  STORAGE_MIGRATION_RECOVERY=PASS (5)
CORE_AUTHORITY=PASS (7, real worker process on Linux, Windows, macOS;
  14 injected worker faults)  CLI=PASS (2)
REAL_PHI_AUTHORIZED=false
RELEASE_READY=false
PRIVATE_DATA_READY=false
SPEC_086_IMPLEMENTATION_AUTHORIZED=false until its own promotion
```

## What this closure establishes

MedScale has a Core-owned Compute path with a separate MedScale-owned
worker process:
- Core admits a typed job from a closed set of kinds, pins one exact Spec
  075 snapshot by its content digest, and records the job with a manifest
  digest over input, runtime identity, execution policy and ceilings.
  Refusals are terminal `denied` jobs with receipts.
- A job runs at most once. A run re-reads and re-digests the stored bytes,
  then pipes them to `medscale-compute-worker`, which starts with a
  cleared environment, an empty private working directory and stdio pipes
  only, and applies its OS mechanism to itself before computing.
- Core bounds stdout and stderr, enforces the timeout and cancellation,
  and treats the answer as a candidate: job binding, digest, canonical
  encoding and shape are checked before the terminal state, receipt and
  output are committed in one transaction.
- Eleven distinct states are kept apart; stderr text is never stored; a
  job left `running` by a crash is recovered as `interrupted` and never
  re-run.
- Outputs are derived and `unreviewed`; nothing writes a source snapshot.

## Defects found and fixed during qualification

- The contract test found `{"kind":"column_profile","code":"x"}` parsing:
  serde ignores unknown fields on unit variants of internally tagged
  enums. `ColumnProfile {}` is now an empty struct variant (`3bd00d1`).
- A scripted edit had rewritten `medscale-cli/src/main.rs` from CRLF to LF;
  found by the scope record's diff statistics and restored (`e26a773`).
- CI found three Clippy/compile defects in new code and tests, fixed
  forward (see `EXACT_RANGE_REVIEW.md`).

## Honest residuals (non-blocking, recorded)

- The worker's OS mechanism is `ReadyBaseMeasured` only; this is defense
  in depth for MedScale's own code, not a sandbox for untrusted code.
- No OS memory or CPU quota; the process-count limit is OS-enforced on
  Windows only; the Windows mechanism has no network axis (the worker has
  no network code).
- Inherited-handle hygiene relies on the Rust standard library; no OS
  handle allowlist.
- Cancelling a running job needs the in-process handle; the CLI runs jobs
  synchronously.
- The worker binary is not in the portable release package; Compute there
  reports `unavailable`.
- No remote or self-hosted worker, GPU, scheduler, R or Python (later
  slices; arbitrary code waits for platform qualification).
- Compute outputs carry no Spec 079 classification row (as Spec 082
  derived tables); they have no egress or export path, and any later
  export must treat them at least as restrictively as their input, whose
  unclassified default is `local_phi`.
- No Desktop surface; no rendered Desktop evidence.
- No local compile or test run completed on this workstation; GitHub
  Actions is the compiler of record.
