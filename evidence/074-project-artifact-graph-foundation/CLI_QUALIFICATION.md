# CLI QUALIFICATION — Spec 074 (074-D, T074-08)

## Binding

```text
BASE_SHA=ee8daef3a2782bbdbcb3324766a5d6b95c09fa09
BRANCH=spec/074-project-artifact-graph-foundation
BINARY=target/debug/medscale.exe (local debug build, Rust 1.97.1)
VAULT=temp synthetic vault (C:\...\Temp\opencode\cli074), vault-id cli-074
FIXTURE=fixtures/synthetic/fhir/r4/valid/patient-min.json (ingested for attach target)
LOG=evidence/074-project-artifact-graph-foundation/logs/cli-vertical-slice.log
REAL_PHI_USED=false
```

## Surface

New module `crates/medscale-cli/src/project.rs` (~1100 lines) + two
`Commands` variants. All commands open a `CliSession` (operator grants) and
the synthetic vault, dispatch one typed Core request, and render human lines
or stable JSON. The CLI never writes storage (dependency-direction gate holds;
`cli_does_not_depend_on_storage_or_rusqlite` passes).

Commands: `project create/list/show/update/archive/restore/context/summary/
attach/detach/list-refs/graph neighbors|edge-create|edge-remove`,
`experiment create/list/show/update/archive`.

Typed errors in both modes: denied/not_found/conflict/stale_reference/
invalid/corrupt/unsupported_schema/unavailable/internal. Human errors exit 1
(`invalid: unknown artifact kind: proves`); JSON errors exit 2
(`{"error":"cli_error","code":"not_found","message":"NotFound",...}`).

## Live vertical slice (29 steps, CLI_VERTICAL_SLICE=PASS, see log)

```text
create x2 (human+json) -> list(2) -> show -> update rev1->2 -> stale rev1 Conflict
-> experiment create/list/show -> ingest synthetic fixture (src-8)
-> attach (ref-12) -> duplicate Conflict -> missing NotFound
-> unknown kind Invalid -> list-refs -> edge-create (edge-14)
-> unknown predicate Invalid -> neighbors(json+filtered) -> context(json)
-> summary -> edge-remove -> detach -> archive(rev3) -> attach-on-archived Denied
-> restore(active rev4) -> show-missing NotFound
```

Parity: every step asserts Core semantics (revisions, Conflict zero-write,
archive freeze, fail-closed vocabularies). IDs are Core-allocated
(proj-1, exp-6, src-8, ref-12, edge-14).

## Platform fix (honest record)

After the project commands landed, the debug binary overflowed its stack on
*every* command including `status` (Windows 1 MiB main-thread default).
Diagnosis: wire-enum sizes tiny (Commands=208B, RequestBody=240B,
ResponseBody=416B, asserted by `cli_wire_enum_sizes_stay_stack_safe`); every
parse path passes in 1 MiB spawned threads; 64 MiB main stack works. Fix:
`main()` runs `run()` on an explicit 8 MiB thread (`CLI_STACK_SIZE`,
documented in code). No behavior change; all existing CLI tests pass.

## Gates

```text
cargo fmt --all -- --check => PASS
check-dependency-direction => PASS
cargo clippy -p medscale-cli -p medscale-contracts --all-targets --locked -- -D warnings => PASS
cargo test -p medscale-cli -p medscale-contracts --locked => PASS (13 + 38 incl.
  closed-vocabulary parser tests, endpoint-spec tests, enum-size guard)
```

Core parity: `cargo test -p medscale-core --test project_graph_074` PASS (12/12)
covers the same semantics the CLI renders.
