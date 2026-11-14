# Evidence — Spec 075 Data Source Fabric + Data Workbench Foundation

## Index

Required evidence set (see `specs/075-data-source-fabric/plan.md`, section 075-J):

```text
README.md (this index + per-file rules)
LIVE_TRUTH.md (T075-00 branch/base/main/PR/CI state)
CONTRACT_QUALIFICATION.md (frozen fields to Rust paths + tests)
STORAGE_MIGRATION_RECOVERY.md (fixtures, before/after, backup/migrate/reopen/restore)
CORE_AUTHORITY_QUALIFICATION.md (mutation/query matrix + denial/conflict/scope)
CLI_QUALIFICATION.md (human + JSON, exits, Core parity)
DESKTOP_QUALIFICATION.md (native state/adapter/a11y + renders)
SECURITY_ADVERSARIAL.md (threat-gate results from security.md)
SCALE_MEASUREMENTS.md (fixtures/generators/hardware, no marketing budgets)
NO_NETWORK_LOCAL_PATH.md (egress-disabled proof for local sources)
EXACT_RANGE_REVIEW.md (authorized scope only, unexpected files, new dependencies)
EXACT_HEAD_QUALIFICATION.md (candidate head + live CI)
POST_MERGE_VERIFICATION.md (merge SHA + main checks)
CLOSURE.md (terminal truth block)
DATA_WORKBENCH_LIGHT.png / DATA_WORKBENCH_DARK.png (deterministic renders)
logs/ (CLI vertical-slice log, workbench UI transcript, scale log)
```

## Per-file rules

Every evidence file must record, where applicable:

```text
base SHA
candidate/head SHA
branch
commands (exact)
environment + platform
toolchain versions (rustc/cargo, perl where relevant)
fixture/data identity and version
test counts (pass/fail/ignored)
CI run IDs + per-job conclusions
logs/artifacts paths
limitations + unresolved failures
```

A screenshot is not sufficient evidence for clinical quality, privacy, security, reproducibility, or release readiness. Screenshots here prove only rendered UI state.

No real PHI in any fixture. No licensed dataset content. No production credentials.

## Honest gaps at closure

`DATA_WORKBENCH_LIGHT.png` / `DATA_WORKBENCH_DARK.png` and `logs/` were
never produced for this spec: this workstation's toolchain cannot link
locally, and `.github/workflows/ci.yml` has no rendering/transcript-capture
step. See `DESKTOP_QUALIFICATION.md` and `CLI_QUALIFICATION.md` for how
each surface was qualified instead (compiled + passing in-module/CLI-facing
tests on real CI, not a fabricated render or transcript). Tracked as an
open residual in `CLOSURE.md`.
