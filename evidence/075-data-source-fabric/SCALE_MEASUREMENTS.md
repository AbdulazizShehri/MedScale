# SCALE_MEASUREMENTS — Spec 075

## Fixtures

- `csv-scale`: 20,000 deterministic rows (`town-{i}`, dose `i % 1000`) in
  `crates/medscale-core/tests/data_sources_075.rs: large_table_import_pages_correctly`.
- Storage/contract suites use small synthetic fixtures; row/page bounds are
  constants in `medscale-contracts/src/data_sources.rs`.

## Method

Timings below are local-only observations from CI logs. They are capacity
evidence, not budget claims: `budgets_claimed_met=false` is unchanged.

```text
SCALE_RUN = PASS: exact-head CI run 35580861670 (head 9f84a6e) green 6/6;
PR #129 merged as 89a88cf; post-merge main run 35582548200 green 6/6.
COMMAND = cargo test --workspace --locked (rust ubuntu/macos/windows jobs)
```

## Results

```text
Cargo's default test runner does not print per-test timing, only per-binary
aggregates, so this records the binary-level number honestly rather than a
fabricated per-test figure. On post-merge main run 35582548200
(rust ubuntu-latest, GitHub-hosted runner), the 15-test Core
data_sources_075 binary - which includes large_table_import_pages_correctly
(20,000 rows) alongside 14 other Core integration tests - completed in
5.38s total. No per-test isolation, snapshot-byte-size, part-count, or
page-count figures were captured; these remain an open evidence gap, not a
budget claim (budgets_claimed_met stays false either way).
```

## Limits stated honestly

- Snapshot bytes cap `SNAPSHOT_BYTES_MAX` (256 MiB), rows cap
  `SNAPSHOT_ROWS_MAX` (1M), page cap 1,000 rows, parts of 500 rows.
- Paging reads one snapshot blob per query (bounded by the byte cap);
  per-part windowing without full-blob reads is future work, not claimed.
