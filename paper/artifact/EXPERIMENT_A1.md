# Experiment A1 — Core-level provenance binding

**Status:** implementation prepared; execution evidence pending exact-head CI.

**Scientific base:** `1e2b7d94e970256b38bda15fa91f62bc397e825a`

**Harness:** `crates/medscale-core/tests/paper_provenance_system.rs`

## Research purpose

A1 moves beyond A0's contract-level controls and exercises the production `CoreFacade` source/derived-artifact path under strict session enforcement. It tests whether an admitted derived artifact can silently bind to a source outside the requesting authority scope or to a nonexistent source, and whether successful creation preserves the exact source and output digest expected by the contract.

## Pre-specified cases

| Case | Condition | Expected behavior |
|---|---|---|
| A1-01 | mutating operation without a client session | `SessionRequired` |
| A1-02 | live session lacks mutation capability | `SessionDenied` |
| A1-03 | valid same-scope source -> derived artifact | exact source id + transform id/version + output bytes + SHA-256 preserved |
| A1-04 | derived artifact references a real source in a different scope | `WrongScope`; no derived artifact admitted |
| A1-05 | derived artifact references nonexistent source | `NotFound`; no derived artifact admitted |
| A1-06 | read a real object through a different scope | `WrongScope` |

## Primary endpoints

- false acceptance of an invalid source binding;
- false refusal of the valid control;
- exact emitted authority state/error;
- preservation of source/transform/output-digest binding for a valid control.

## Interpretation boundary

A1 is a Core-level conformance experiment. It does **not** establish:

- durable-storage tamper resistance;
- complete mediation of every MedScale subsystem;
- cryptographic authenticity of all provenance receipts;
- resistance to a compromised host/kernel;
- clinical correctness or clinical safety.

Those properties require separate experiments or remain explicitly out of scope.

## Planned command

```bash
cargo test -p medscale-core --test paper_provenance_system
```

A1 remains `PENDING` until an exact harness revision completes CI. The result record must include exact head SHA, workflow run, platform(s), observed test count, and any failure output.
