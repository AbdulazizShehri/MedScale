# Perf harness methodology (Spec 027 + Spec 032 binding)

## Delivery-plan targets (NOT claimed met)

From `docs/planning/TRUSTED_V1_DELIVERY_PLAN.md`:

| Path | Target | Measurement coverage |
|---|---|---|
| Cold model-free desktop launch | p95 ≤ 2 s | Spec 057 process-launch harness; attainment not claimed |
| Timeline for 10,000 events | p95 ≤ 250 ms | Specs 027/042/050 harness path |
| Lexical search over 10,000 local records | p95 ≤ 300 ms | Specs 045/050 harness path |
| 1 MiB bounded FHIR ingest (excl. external validation) | p95 ≤ 500 ms | Specs 027/042/050 harness path |
| Final UI interaction response | ≤ 100 ms | `FINAL_UI_PRESENT_EXTERNAL_INTERACTION_MEASUREMENT_REQUIRED`; no result claimed |
| Model-free desktop idle memory | ≤ 250 MiB | Spec 057 RSS harness; attainment not claimed |

## Harness procedure

1. Synthetic fixtures only; REAL_PHI unauthorized.
2. Explicit warmup runs (default 3), then timed runs (default 30).
3. Sort sample durations ascending.
4. p50 = sample at index `floor(n * 50 / 100)`; p95 = sample at index `floor(n * 95 / 100)` (clamped).
5. Report milliseconds + hardware/toolchain note + measured scale.
6. Set `budgets_claimed_met=false` and `release_ready=false` in JSON evidence.

## Spec 032 evidence binding

Each `perf_harness_latest.json` also records (schema_version ≥ 2):

- `binding.git_sha` / `binding.git_tree`
- `binding.rustc_version`
- `binding.os` / `binding.arch` / `binding.hostname` / `binding.cpu_note`
- `binding.cargo_lock_sha256`
- `binding.fixture_identity` (timeline event count, lexical corpus/query, FHIR scale)

Binding does **not** imply budget attainment or `RELEASE_READY`.

## CI policy

Assert only that the harness executes and emits finite p50/p95 numbers plus binding fields.
**Do not** fail CI when measured times exceed delivery-plan budgets.

## Regenerating evidence

```powershell
$env:CARGO_TARGET_DIR = "D:\medscale-target"
# Optional: nearer to delivery-plan 1 MiB ingest (still not a budget claim)
# $env:MEDSCALE_027_FHIR_BYTES = "1000000"
cargo test -p medscale-core --test perf_harness_027 --locked -- --nocapture
```

Output: `evidence/027-perf-sbom-release-evidence/perf_harness_latest.json`

### Spec 042 delivery-plan scale

```powershell
$env:MEDSCALE_027_DELIVERY_PLAN_SCALE = "1"   # timeline 10_000 + FHIR 1 MiB
$env:MEDSCALE_027_TIMED_RUNS = "5"            # optional (default 5 under delivery-plan scale)
cargo test -p medscale-core --test perf_harness_027 --locked -- --nocapture
```

Output: `evidence/042-perf-delivery-plan-scale/perf_harness_delivery_plan_scale.json`  
CI job `perf delivery-plan scale (windows)` runs this path. Lexical remains builtin (not 10k). `budgets_claimed_met=false`.
