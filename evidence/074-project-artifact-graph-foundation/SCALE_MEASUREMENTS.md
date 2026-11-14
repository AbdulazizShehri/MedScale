# SCALE MEASUREMENTS — Spec 074 (074-F, T074-11)

## Binding

```text
BASE_SHA=ee8daef3a2782bbdbcb3324766a5d6b95c09fa09
BRANCH=spec/074-project-artifact-graph-foundation
HARNESS=crates/medscale-core/tests/project_graph_scale_074.rs (Core facade paths,
  synthetic vaults in temp dirs, Instant timings, --nocapture lines)
HOST=Windows 11 x64, Rust 1.97.1 debug profile
REAL_PHI_USED=false
```

Fixtures (spec minimums): Personal = 10 Projects x 100 refs + 100 edges each
(1,000 refs, 1,000 edges); Lab = 100 Projects (>= 100 bar) x 2 refs/edges;
reopen = 1 project x 50 refs. Routine `cargo test` runs a small smoke shape;
full fixtures run with `MEDSCALE_074_FULL_SCALE=1` (repo `perf_harness_027`
convention). Measurements are observations, never guarantees or budgets.

## Historical curve that forced the architecture (pre-fix code, debug)

Per-attach latency with per-op memory audit rows + full-snapshot rewrite
(`probe_attach_cost_curve`, 30 sequential attaches, same vault):

```text
i=0:14.89 i=1:17.49 i=2:23.58 i=3:26.52 i=4:32.46 i=5:38.88 i=6:43.15
i=7:42.27 i=8:49.68 i=9:51.55 i=10:56.50 i=11:63.61 i=12:66.25 i=13:70.57
i=14:73.70 i=15:79.89 i=16:85.51 i=17:89.52 i=18:92.08 i=19:96.13 i=20:89.54
i=21:101.86 i=22:114.16 i=23:107.02 i=24:109.10 i=25:113.94 i=26:122.98
i=27:130.47 i=28:128.11 i=29:134.30  (ms)
```

Slope ~= +4 ms/op/history-row. Extrapolated personal fixture (3,020 ops):
~5 hours; two 60-minute runs confirmed non-completion. First-attach on an
empty vault measured 42-47 ms across runs (snapshot + blob fsync baseline,
not graph cost). This curve is the explicit reason for the frozen 074-C
amendment (lifecycle audits + durable receipts + sqlite id sequence +
snapshot-skip for the four high-frequency ops).

## Post-fix expectations (to be filled by the full run)

With the amendment, high-frequency per-op cost is constant in graph size
(one sqlite transaction + unchanged-memory snapshot skip); the snapshot term
grows only with lifecycle/source history. Predicted personal fixture: single
minutes, not hours. The full run fills this table:

```text
personal_mutations: ops=3020 total_ms=<pending> mean_ms_per_op=<pending>
personal_first_attach_ms=<pending> personal_last_attach_ms=<pending>
personal_list_ms=<pending> (10 projects)
personal_context_ms=<pending> (100 refs + 100 edges page)
personal_neighbors_ms=<pending> (100 edges)
personal_db_bytes=<pending>
lab_mutations: ops=800 total_ms=<pending>
lab_list_ms=<pending> (100 projects)
lab_db_bytes=<pending>
reopen_ms=<pending> (measured 5.67 pre-fix at 50 refs / 188,416 bytes)
```

## CI full-scale partials (post-fix code, run 35312537392, job 105497342591)

Windows CI runner, debug profile, `MEDSCALE_074_FULL_SCALE=1`. Shapes
confirmed full (`personal_shape=projects=10 refs_per_project=100 full=true`,
`lab_shape=projects=100 full=true`). Step hit its 30-minute timeout before
completion, so these are PARTIAL observations, never a pass claim:

```text
reopen_duration_after_scale: PASS (reopen_ms=1.31, 172,032 bytes, 1 project / 50 refs)
personal_first_attach_ms=485.00 (empty vault; CI disk is far slower than dev iron)
personal_progress refs=100 elapsed_s=498.1
personal_progress refs=200 elapsed_s=1729.7
lab_progress projects=20 elapsed_s=197.9
lab_progress projects=40 elapsed_s=615.1
lab_progress projects=60 elapsed_s=1266.6
correctness failures before timeout: ZERO (no failing assert, no panic; the
  step died on the clock, and every completed op verified)
```

Root cause of the residual curve: EVERY persisting op (project/experiment
lifecycle, and each `CreateSourceRecord`) runs the frozen
`sync_store_to_meta`, which rewrites ALL source blob files, upserts ALL
source meta rows, re-serializes EVERY memory object, and replaces the whole
authority snapshot. The scale fixtures are dominated by source creations
(1,000 of 3,020 personal ops), so per-op cost still grows with total
history. This is pre-existing frozen behavior (Specs 016/035), NOT 074
scope: the 074 amendment covers only 074-owned high-frequency ops
(attach/detach/edge create/remove), which no longer persist at all.

Consequence: full-fixture TIMINGS cannot be produced inside bounded CI
time. A CI perf-job full-scale step was tried on the closure branch and
reverted the same day (red-marked the perf job on timeout with zero
failures). Full timings remain local-only evidence for after the
workstation toolchain recovery (external gate below). No budget is claimed.

## Pre-fix full-shape lab run (measured, debug, before the amendment)

Source: `C:\Users\Shehr\AppData\Local\Temp\opencode\scale-run.log` (2026-09-18,
run completed before the toolchain event; code WITH per-op memory audits):

```text
lab_mutations: ops=800 total_ms=328157.4 mean_ms_per_op=410.197
lab_list_ms=844.12 projects=100
lab_db_bytes=491520
reopen_db_bytes=172032
reopen_ms=3.71 projects=1 refs=50
```

Personal full shape did not complete in that run (toolchain event
intervened); its projection under the old code is ~5 h (curve above), which
is the explicit reason for the amendment. Post-fix full numbers remain
pending (toolchain gate).

## Status

```text
SMOKE_COVERAGE=CI (fast shape asserts structure on every push)
FULL_NUMBERS=PENDING (workstation MSVC toolchain removed overnight 2026-09-17/18:
  cl.exe/link.exe/nmake.exe absent, Strawberry registry entry without files;
  local native builds impossible; recorded as external blocker below)
```

Known limitation: `list_all_*` snapshot scans load full row sets into memory;
observed sizes are reported above, never converted into budgets.
Storage growth is dominated by the authority snapshot (lifecycle/source
history), not by 074 rows (scalar columns + two small JSON enums per row).
