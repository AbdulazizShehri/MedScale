# CLI Qualification — Spec 078 (T078-03..T078-07)

`medscale model-fleet <command>` (`crates/medscale-cli/src/model_fleet.rs`).
Every command opens its own `CliSession` over `--vault-id/--vault-root`,
dispatches one typed Core request (two when a Pack must first be admitted in
the process), and prints either human lines or pretty JSON (`--json`).
Errors use the shared `CliJsonError` shape with the same code mapping as
`medagent`: `denied`, `not_found`, `conflict`, `invalid`, `corrupt`,
`unsupported_schema`, `unavailable`, `internal`.

| Command | Core request | Notes |
|---|---|---|
| `lane-create` | `AgentLaneCreate` | `--tool-kinds`, `--context-artifacts` optional comma lists; omitted means inherit |
| `lane-show` / `lane-list` | `AgentLaneGet` / `AgentLaneList` | `--status active\|retired`; unknown values refused |
| `lane-retire` | `AgentLaneRetire` | `--expected-revision` |
| `fleet-create` | `FleetRunCreate` | |
| `fleet-show` / `fleet-list` | `FleetRunGet` / `FleetRunList` | human output prints the prompt length, not the prompt |
| `fleet-dispatch` | `PacksInstallLocal`* + `FleetRunDispatch` | `--lanes a,b`, repeatable `--pack-dir` (Core keeps admitted Packs per process) |
| `fleet-execute-lane` | `PacksInstallLocal` + `FleetRunExecuteLane` | `--pack-dir`, `--max-tokens` (default 64); synthetic-only |
| `fleet-cancel` | `FleetRunCancel` | `--expected-revision` |
| `compare-run` / `compare-list` | `ComparisonCompute` / `ComparisonReportList` | human output: one line per observation (`kind`, lanes, escaped detail) |

Role labels and observation details are printed with `escape_debug`, so
control or bidi characters stay visible as escapes (`security.md` T7).

## Tests

`model_fleet::tests::lane_commands_run_through_core_across_fresh_sessions`
runs the real command handlers, each in its own fresh session over one
persisted vault: lane create (JSON and human), T2 refusal and unknown tool
kind refused, show/list in both modes, unknown status refused, retire and
stale-retire conflict, then fleet create, dispatch refused for a retired
lane and for lanes whose Pack is not admitted, dispatch, show/list in both
modes, cancel, readback of the cancelled fleet, compare refused on a
cancelled fleet, and compare-list in both modes. Real execution and
comparison paths are covered at the Core level
(`CORE_AUTHORITY_QUALIFICATION.md`), since each CLI command is a single
`CliSession` call.
