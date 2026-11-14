# Trusted V1 Performance Coverage Dossier — Spec 057

## Canonical target coverage

| Trusted V1 target | Measurement path after Spec 057 | Attainment state |
|---|---|---|
| Cold model-free launch p95 ≤ 2 s | `runtime_perf_057` process-launch samples | NOT CLAIMED |
| Timeline 10,000 events p95 ≤ 250 ms | Specs 027/042/050 | NOT CLAIMED |
| Lexical 10,000 records p95 ≤ 300 ms | Specs 045/050 | NOT CLAIMED |
| 1 MiB FHIR ingest p95 ≤ 500 ms | Specs 027/042/050 | NOT CLAIMED |
| Final UI response ≤ 100 ms | `BLOCKED_BY_FINAL_V0_UI` | NOT MEASURED |
| Model-free idle memory ≤ 250 MiB | `runtime_perf_057` RSS samples | NOT CLAIMED |

## Evidence policy

Runtime JSON is generated under `target/medscale-evidence/057-release-qualification-residual-integrity/` and uploaded by the existing three-OS CI matrix. Reports bind git SHA/tree, Cargo.lock SHA256, rustc, OS, architecture, method parameters, and raw samples.

CI verifies the harness executes and evidence exists; it does not fail on the proposed release budgets. Qualification on declared release hardware is a later gate.