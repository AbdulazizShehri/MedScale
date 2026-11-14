# Evidence — Spec 078 Model Fleet + Compare

## Index

Required evidence set (see `specs/078-model-fleet-compare/plan.md`, section
078-H):

```text
README.md (this index + per-file rules)
LIVE_TRUTH.md (T078-00 branch/base/main/PR/CI state)
CONTRACT_QUALIFICATION.md (frozen fields to Rust paths + tests)
STORAGE_MIGRATION_RECOVERY.md (fixtures, before/after, backup/migrate/reopen/restore)
CORE_AUTHORITY_QUALIFICATION.md (mutation/query matrix + denial/conflict/scope)
CLI_QUALIFICATION.md (human + JSON, exits, Core parity)
DESKTOP_QUALIFICATION.md (native state/adapter/a11y + renders or honest residual)
SECURITY_ADVERSARIAL.md (threat-gate results from security.md, incl. the
  structural no-effect proof and the fabricated-comparison/T4 proof)
NO_NETWORK_LOCAL_PATH.md (egress-disabled proof)
LANE_PLURALITY_QUALIFICATION.md (exactly how the two-lane closure fixture
  proves genuine independence given the repository's real model inventory,
  with real run evidence)
EXACT_RANGE_REVIEW.md (authorized scope only, unexpected files, new dependencies)
EXACT_HEAD_QUALIFICATION.md (candidate head + live CI)
POST_MERGE_VERIFICATION.md (merge SHA + main checks)
CLOSURE.md (terminal truth block)
logs/ (CLI vertical-slice log, any UI transcript)
```

## Per-file rules

Every evidence file must record, where applicable:

```text
base SHA
candidate/head SHA
branch
commands (exact)
environment + platform
toolchain versions (rustc/cargo)
fixture/data identity and version
test counts (pass/fail/ignored)
CI run IDs + per-job conclusions
logs/artifacts paths
limitations + unresolved failures
```

A screenshot is not sufficient evidence for clinical quality, privacy,
security, reproducibility, or release readiness. Screenshots here prove
only rendered UI state.

No real PHI in any fixture. No production credentials.

## Known workstation constraint (carried forward from Spec 075/076/077)

This workstation's local Windows toolchain cannot link (no MSVC
`link.exe`/`cl.exe` installed). Local `cargo build`/`cargo test` cannot
produce a working binary. `cargo fmt --check` and `cargo metadata` remain
available locally. GitHub Actions CI is the authoritative qualification
path for every build/test/clippy/deny claim in this evidence packet;
nothing here should be read as "verified by local execution" unless
explicitly stated.
