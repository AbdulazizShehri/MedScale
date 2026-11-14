# Security Challenge — Spec 085 MedScale Compute

Deterministic challenge under `FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`.
No external or LLM reviewer ran. The threat table with controls and proofs
is `specs/085-compute/security.md` (C1-C20); this file records what the
challenge changed and what remains.

## Sandbox truth at promotion (live source)

```text
OsSandboxDoctorStatus::ready_base()  platform_qualified = false
try_apply_os_sandbox(PlatformQualified plan) -> NotPlatformQualified
composition_residuals_open = macos_app_sandbox_signed_enforcement,
                             multi_os_platform_qualified_composition
Mechanisms the worker applies to itself (all ReadyBaseMeasured):
  Linux   linux_landlock_composition_ready_base  (Spec 052: FS allowlist = scratch dir,
                                                  TCP bind/connect deny, RLIMIT_NOFILE)
  Windows windows_job_object_ready_base          (Spec 030: active process limit 1,
                                                  kill on job close)
  macOS   macos_seatbelt_ready_base              (Spec 031: network deny)
```

Compute does not change any of this and is never reported as a secure
sandbox for untrusted code: it runs only MedScale's own code for two
closed job kinds.

## Design decisions made by the challenge (before first CI)

| Finding | Decision |
|---|---|
| Staging inputs as files in a temp directory would add path, symlink, replacement and cleanup races | inputs are piped on stdin as the exact stored bytes; the worker opens no file; its working directory is a new empty directory (`create_dir` refuses an existing path) removed after the run |
| A configurable worker path (environment variable or request field) would let a caller substitute any executable | the worker is found only next to the running executable; harness workers are installable only by in-process Rust callers (`set_compute_runtime`), never through a request |
| Trusting the worker's own "sandbox required" check | Core re-checks the sandbox report independently; a completion without an applied mechanism under `ready_base_required` is `unavailable` (fault `no-sandbox-completes`) |
| Accepting any JSON that parses to the right table | the output must be the canonical encoding of the table, digested over the exact bytes, echo the job id and manifest digest, and match the kind's shape |
| Recording a denied job's input fields when the input could not be read would invent values | `input` is `None` for jobs refused before their input was read; only `denied` and `corrupt` jobs may lack a runnable input |
| Keeping worker stderr as diagnostics | stderr text is never stored; only its byte count and a prefix digest |
| A crash between claim and commit | recovery marks the job `interrupted`; it is never re-run automatically, and a run recovers orphans first |
| Environment leakage | the environment is cleared and the worker reports the count it sees; any non-zero count ends `unavailable` |

## Defect found by qualification

| Finding | Fix |
|---|---|
| The contract test showed `{"kind":"column_profile","code":"x"}` parsing: serde ignores unknown fields on unit variants of internally tagged enums, so the closed-parameters claim did not hold for that kind (nothing would have executed the extra field) | `ColumnProfile {}` is an empty struct variant; unknown fields are refused and the JSON shape is unchanged (`3bd00d1`) |

## Honest residuals

- The worker's OS mechanism is `ReadyBaseMeasured` only; `platform_qualified=false`.
- No OS memory or CPU quota; the process-count limit is OS-enforced on
  Windows only; the Windows mechanism has no network axis (the worker has
  no network code).
- Inherited-handle hygiene relies on the Rust standard library opening
  handles non-inheritable; no OS handle allowlist is applied.
- Cancelling a running job needs the in-process handle; the CLI runs jobs
  synchronously.
- The worker is not yet in the portable release package; Compute there is
  `unavailable`.
