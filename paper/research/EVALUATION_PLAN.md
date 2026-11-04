# Dedicated paper evaluation plan

**Snapshot:** `1e2b7d94e970256b38bda15fa91f62bc397e825a`

**Data policy:** synthetic only. No real PHI. No production credentials.

**Interpretation rule:** existing repository qualification is prior evidence. Dedicated paper experiments are new measurements and must be stored separately. Historical repaired defects are not counted as prospective failures.

## RQ1 — Provenance integrity

### Objective
Measure whether invalid provenance relationships are detected/refused without falsely refusing known-valid controls.

### Planned case families

1. source content changed while recorded digest is held constant;
2. digest changed while source bytes are held constant;
3. derived artifact bound to the wrong source identity;
4. transform identity/version mismatch where validation exists;
5. cross-project artifact/source binding;
6. forged or mismatched receipt where a receipt contract exists;
7. malformed lineage / parent reference;
8. evidence assessment with nonexistent citation but supporting relation;
9. retracted citation incorrectly marked as support;
10. tampered recovery material that changes provenance-bearing state;
11. recovery/receipt mismatch after restore;
12. matched valid controls for every mutation class.

### Metrics
- invalid cases tested;
- false accepts (primary safety-oriented error count);
- detected/refused invalid cases by class;
- valid controls tested;
- false refusals;
- exact emitted error/state;
- deterministic repeat agreement.

Do not report a percentage without its numerator/denominator and case-generation rule.

## RQ2 — Adversarial and failure qualification

### Pre-specified classes
Malformed input; forged signatures; replay; revoked identity; cross-project access; corrupt backup; interrupted migration; stale/invalid state; malicious digest; wrong privacy classification; unknown/unavailable external effect; ID-sequence rollback; duplicate submission; unauthorized capability; corrupt event/state chain when the subsystem exposes such a path.

### Result schema

| case_id | subsystem | fault | precondition | expected | observed | evidence | result |
|---|---|---|---|---|---|---|---|

A fail-closed refusal counts as correct only if refusal is the pre-specified expected behavior.

## RQ3 — Reproducibility and recovery

### Determinism
For deterministic operations, run identical inputs in independent fresh synthetic vaults and compare the declared invariant (digest, canonical serialization, query result, or state transition). Avoid comparing timestamps/random identifiers unless the contract claims deterministic identity.

### Recovery chain

```text
seed synthetic state
-> verify
-> backup
-> verify backup
-> restore into fresh destination
-> verify restored state
-> restart/reopen
-> re-verify
-> create valid post-restore objects/actions
-> verify monotonic identity/state invariants
```

### Cross-platform scope
Reuse hosted Linux/Windows/macOS CI only for cases that genuinely execute on each OS. Platform-specific exclusions remain explicit.

### Executed C1 protocol
C1 reuses the production `whole_platform_092` synthetic backup/restore/restart test without changing its endpoint. It is executed three times in independent Cargo test processes on each hosted OS. The pre-specified unit is one complete process execution of the integrated recovery path; timestamps and random identifiers are not compared across repeats.

At exact revision `e1cf8935f3920c95cf7229221619fbfb24fd0fa4`, all 9/9 planned executions completed successfully in paper-evaluation run `36645800815`. This is repeatability/conformance evidence for the specified synthetic workload, not production disaster-recovery qualification.

## RQ4 — Absolute performance and storage cost

RQ4 is pre-specified before observing paper-specific D0/D1 measurements. Historical Spec 027/050 performance artifacts are engineering context only and are not imported as paper results.

### D0 — Core operation latency

D0 measures three existing Core operations at one fixed synthetic scale:

- timeline projection after seeding **1,000** promoted synthetic observations;
- lexical retrieval over the built-in procedural **10,000-document** scale corpus using the fixed query `hypertension blood pressure` and `max_hits=5`;
- synthetic FHIR R4 ingestion with a target serialized payload size of **131,072 bytes (128 KiB)**.

Two fresh Cargo test processes are used per OS:

1. **D0-cold:** zero warm-up iterations and one timed sample per operation. “Cold” means the first measured invocation after fixture construction in that fresh test process; it does **not** mean cold OS boot, cold compiler cache, or cold machine cache.
2. **D0-warm:** five untimed warm-up iterations followed by **100** timed samples per operation.

The harness records exact Git revision/tree, Cargo.lock digest, rustc version, OS, architecture, CPU/core hints, fixture identity, raw monotonic timings, p50, p95, p25/p75/IQR, minimum, and maximum. Hosted-runner memory/RSS is not promoted as a cross-platform paper metric because a comparable process-memory collection method is not yet established in this harness.

No delivery-plan latency threshold is used as a hypothesis test. D0 reports absolute observed cost only. No result may be called “overhead” because there is no paired semantically equivalent no-provenance baseline.

### D1 — Persisted storage footprint of integrated synthetic state

D1 runs the unchanged `whole_platform_092` integrated synthetic workload once in a fresh process after removing only stale temporary directories owned by that test prefix. After the test exits, a separate measurement script recursively counts files and bytes for:

- live `vault` state;
- `backup` state;
- `restored` state;
- staging state when present;
- top-level composition of the live vault.

The report binds the measurements to the exact checkout and runner OS. Ratios such as `backup_bytes / vault_bytes` and `restored_bytes / vault_bytes` are descriptive size ratios only; they are not compression, efficiency, durability, or readiness claims.

D1 explicitly excludes compiled binaries, Cargo caches, model weights, OS caches, external network stores, and real PHI. It sets no storage threshold and makes no production-capacity claim.

### Cross-platform execution and evidence

D0 and D1 run on hosted Linux, macOS, and Windows in the dedicated `paper-evaluation` workflow. Every raw log and JSON result is retained in the per-OS artifact and recursively SHA-256 hashed. Paper tables/figures must be regenerated from these artifacts rather than manually transcribed.

### Statistical reporting rule

For D0-warm, report the 100 raw samples and descriptive p50/p95/IQR/min/max by OS and operation. Do not pool heterogeneous hosted operating systems into a single latency estimate. D0-cold is a single-run descriptive observation and receives no confidence interval. D1 is one deterministic corpus construction per OS and is reported as exact byte/file counts for that generated state, not as a population estimate.

## Optional Experiment E — ablation

Deferred by default. It may proceed only as an isolated research harness after documenting:
- scientific question;
- identical input corpus;
- what validation is removed/simulated;
- why the comparison is not confounded by unrelated code paths;
- why production MedScale is not weakened.

## Artifact layout

```text
paper/artifact/
  README.md
  manifests/
  fixtures/
  scripts/
  raw/
  generated/
  checksums/
```

Raw results are append-only per experiment run. Generated manuscript tables/figures must be reproducible from `raw/` and never hand-edited as authoritative data.
