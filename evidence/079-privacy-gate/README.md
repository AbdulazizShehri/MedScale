# Evidence — Spec 079 Privacy Gate

Required evidence set (see `specs/079-privacy-gate/plan.md`):

```text
README.md, LIVE_TRUTH.md, CONTRACT_QUALIFICATION.md,
STORAGE_MIGRATION_RECOVERY.md, RECOGNIZER_BENCHMARK.md,
CORE_AUTHORITY_QUALIFICATION.md, CLI_QUALIFICATION.md,
DESKTOP_QUALIFICATION.md, SECURITY_ADVERSARIAL.md, NO_NETWORK_LOCAL_PATH.md,
EXACT_RANGE_REVIEW.md, EXACT_HEAD_QUALIFICATION.md,
POST_MERGE_VERIFICATION.md, CLOSURE.md
```

Every file records, where applicable: base and head SHA, exact commands,
platform and toolchain, fixture identity, test counts, CI run ids and job
conclusions, and limitations. Synthetic data only; no production credentials.

Workstation note: the local Windows toolchain cannot link (external gate
`LOCAL_WINDOWS_TOOLCHAIN_DESTRUCTION_2026_09_18`). Some 079 tests were first
run in a local WSL Ubuntu build; that distro later failed with disk I/O
errors (2026-09-23) and cannot start. GitHub Actions CI is the authoritative
qualification path; local runs are named as such wherever cited.
