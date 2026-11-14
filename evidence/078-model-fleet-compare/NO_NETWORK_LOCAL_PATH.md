# No-Network / Local-Path Qualification — Spec 078

## Claim

Spec 078 adds no network path. Fleet lanes run locally through Spec 077's
unmodified execution machinery against a local Pack path, using synthetic data
only.

## Evidence

| Item | How it is proven | Result |
|---|---|---|
| No network API in any 078 module | `grep -n -E "std::net\|TcpStream\|UdpSocket\|reqwest\|ureq\|hyper::"` over `crates/medscale-{contracts,storage,cli,desktop}/src/model_fleet*.rs` and `crates/medscale-core/src/authority/model_fleet*.rs` | none |
| No network crate in 078's crates | `grep -E "^(reqwest\|ureq\|hyper\|tokio\|isahc\|curl\|surf\|attohttpc)"` over `crates/medscale-{contracts,storage,core,cli,desktop}/Cargo.toml` | none; no manifest changed in the PR (`EXACT_RANGE_REVIEW.md`) |
| Lane execution path | `ModelFleet::execute_fleet_lane` (`crates/medscale-core/src/authority/model_fleet.rs`) calls Spec 077 `execute_agent_run(run_id, local_path, max_tokens, synthetic_only)` and nothing else for inference | local Pack path only |
| Real-PHI gate before any lane-run mutation | `execute_fleet_lane` returns `ExternalGateRequired { gate: "REAL_PHI_MODEL_RUNTIME" }` when `synthetic_only == false`; test `real_phi_gate_and_foreign_lanes_leave_the_lane_run_untouched` (Core `tests/model_fleet_078.rs`) | pass in CI run `35768404980` |
| Real local execution | `two_real_lanes_produce_a_factual_grounded_report` and the Desktop `model_fleet_workspace` tests run the real ONNX fixture Pack in-process on hosted CI runners | pass in CI run `35768404980` |

## Limitations

- There is no OS-level egress capture (for example a firewall or packet trace
  during the tests). The proof is structural (no network code or crate on the
  path) plus Spec 077's existing runtime deny posture. Runtime product network
  stays default-deny per `IMPLEMENTATION_AUTHORITY.md`.
- Model execution is on synthetic fixtures only. This says nothing about PHI
  readiness (`REAL_PHI_AUTHORIZATION = NOT_AUTHORIZED`).
