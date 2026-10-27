# C1 protocol — repeated fresh-vault recovery

Status: **preregistered before result inspection**.

## Question

Does the production Spec 092 integrated recovery path complete repeatedly from fresh synthetic state across the three hosted CI operating systems at one exact research revision?

## Production path under test

`crates/medscale-core/tests/whole_platform_092.rs::every_plane_survives_backup_restore_and_restart_through_core`

The test exercises one synthetic vault through the integrated MedScale planes, performs storage-layer backup and restore into a new directory, runs the consistency verifiers, reopens the restored vault through Core, and asserts the documented post-restore semantics.

C1 reuses this production qualification path rather than creating a weaker paper-only simulation.

## Repetition design

For each hosted runner operating system:

1. invoke the exact production integration test in a separate Cargo test process;
2. repeat three times (`c1-r1`, `c1-r2`, `c1-r3`);
3. retain each raw log independently;
4. record the environment manifest and exact checkout SHA;
5. SHA-256 hash every retained log before artifact upload.

The test constructs its temporary directory from the test-process PID and removes any pre-existing directory at that path before creating the synthetic vault. Separate Cargo invocations therefore provide separate process-local executions and fresh test directories.

## Platforms

- `ubuntu-latest`
- `macos-latest`
- `windows-latest`

No result is promoted to cross-platform C1 evidence unless all nine planned executions (3 repeats × 3 hosted OS runners) complete successfully at the same exact checkout revision.

## Primary endpoint

Binary semantic conformance per execution: all assertions in the production `whole_platform_092` test complete successfully.

Planned summary:

- successful repeats / 3 for each OS;
- successful platform-repeat executions / 9 overall.

C1 does **not** require byte-identical random IDs, timestamps, log text, or wall-clock timing across platforms. The endpoint is preservation of the production test's semantic recovery invariants.

## Failure handling

- assertion failure: record as test failure;
- build/toolchain/runner/evidence-wrapper failure before test completion: record separately as infrastructure failure, not product assertion failure;
- any failed or methodologically invalid run remains in the results ledger;
- repair requires a new exact revision and a new qualification run; prior failures are not overwritten.

## Claim boundary

A successful C1 can support only this bounded statement: the specified integrated synthetic backup/restore/restart path satisfied its assertions in three fresh-process repetitions on each of the three hosted CI operating systems at the exact tested revision.

It does not establish:

- durability under arbitrary crashes or storage corruption;
- production disaster-recovery fitness;
- universal determinism;
- clinical safety or validity;
- regulatory compliance;
- real-PHI readiness;
- platform qualification;
- release readiness.
