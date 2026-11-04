# Pre-specified research questions and expected invariants

The paper is primarily a systems/conformance study, not a clinical-effectiveness trial. These statements define falsifiable expected invariants before the dedicated experiments are interpreted.

## RQ1 — provenance integrity

**Question:** To what extent does MedScale detect or refuse invalid provenance bindings under controlled mutation or scope mismatch?

Expected invariants:

- **H1a:** a source whose stored bytes no longer match its recorded digest fails contract verification;
- **H1b:** Core refuses a derived-artifact binding to a source outside the requesting authority scope;
- **H1c:** Core refuses a derived-artifact binding to a nonexistent source;
- **H1d:** a valid admitted derived artifact preserves its exact source identity, transform identity/version, output bytes, and output digest.

Primary error endpoint: false acceptance of a pre-specified invalid binding. Valid matched controls are retained to observe false refusal.

## RQ2 — authority and fail-closed effects

**Question:** Do authority and effect boundaries produce their specified states under missing capability, invalid transition, unavailable external gate, and adversarial inputs?

Expected invariants:

- mutating Core operations without a required session are refused;
- sessions lacking the requested capability are refused;
- agent/model output remains a non-authoritative proposal unless a separately authorized promotion path is invoked;
- an effect state of `Unknown` cannot be blindly retried through a transition that requires explicit reconciliation;
- external paths whose governing gate/transport is not admitted return an explicit refusal/unavailable state rather than a simulated success.

## RQ3 — deterministic recovery

**Question:** Which deterministic and recovery invariants reproduce at the frozen snapshot?

Expected invariants are defined per operation. No test will compare random identifiers or wall-clock timestamps unless the corresponding contract declares them deterministic. Recovery must preserve pre-specified semantic state and permit valid post-restore work without identity-sequence regression.

## RQ4 — operation and storage cost

**Question:** What measured latency and storage costs accompany provenance-bearing and recovery operations as synthetic scale increases?

No directional performance hypothesis is asserted in advance. `Overhead` will be reported only when a semantically equivalent paired baseline is available; otherwise the paper reports absolute operation cost.

## Interpretation rule

The mutation corpus is a **conformance suite**, not a probability sample of real-world attacks or clinical failures. A high pass rate on the corpus estimates neither the prevalence of failures in deployment nor universal security.
