# Exact-Head Qualification — Spec 085

```text
BASE_SHA   = 434d1c71f9739d74e886b235a237cb741ddc51b5 (PR #150 merge, Spec 084 closure)
BRANCH     = spec/085-compute (PR #149)
CODE_HEAD  = 3bd00d1ca46a0a20d8724bd033aa61920dd9f6d2
CI_RUN     = 36199048976 (pull_request), bound to CODE_HEAD
TOOLCHAIN  = rustc/cargo 1.97.x (CI)
BASE POST-MAIN RUN = 36203706536 (push on 434d1c7), verified before merge in
                     POST_MERGE_VERIFICATION.md
```

| Job | Conclusion on CODE_HEAD |
|---|---|
| supply-chain policy present | success |
| cargo-deny | success |
| perf delivery-plan scale (windows) | success |
| rust (ubuntu-latest) | success (every step) |
| rust (macos-latest) | success (every step) |
| rust (windows-latest) | fmt, dependency direction, Clippy and Test success; the job was then cancelled during portable package qualification when a documentation-only push superseded the run (`cancel-in-progress`) |

Each `rust (*)` job runs fmt check, dependency direction, Clippy with
`-D warnings`, the full workspace tests, and portable package
qualification. Counts from the job logs (`test result:` lines summed):

| Job | `test result` lines | passed | failed | ignored | `FAILED`/`panicked` lines |
|---|---|---|---|---|---|
| rust (ubuntu-latest) | 129 | 874 | 0 | 1 | 0 |
| rust (windows-latest) | 129 | 871 | 0 | 1 | 0 |

All 20 Spec 085 tests in `QUALIFICATION.md` appear with `ok` in both logs,
including `jobs_run_in_the_worker_and_match_hand_computed_fixtures` (the
real worker applied the Landlock composition on Linux and the Job Object
on Windows, with zero environment variables),
`faulty_workers_are_contained_and_commit_nothing`,
`a_running_job_cancels_through_its_handle` and
`tampered_compute_backups_are_refused`. The macOS job passed (Seatbelt);
its per-test counts were not extracted.

Commits after the code head (`143150e` merge of main, `4018239` and the
evidence commit) change documentation only
(`git diff 3bd00d1 <final head> -- crates` is empty). The final head needs
its own complete 6/6 run before merge, recorded in
`POST_MERGE_VERIFICATION.md` and `CLOSURE.md`.

Earlier heads on this branch (historical only):

| Head | Run | Result |
|---|---|---|
| 6b88ad8 | 36195757108 | failure (Clippy `collapsible_if`); cancelled |
| 0bbfaed | 36196024944 | cancelled (superseded) |
| 7adf90f | 36196129573 | failure (Clippy `assertions_on_constants`); cancelled |
| 7560ad5 | 36196419080 | failure (test compile: borrow of `lab`) on all OSes |
| 9f173fe | 36197476199 | Clippy success on all OSes; failure: contract test found unknown fields accepted on `column_profile` params |
