# Experiment B0 — Authority promotion and external-effect conformance

**Status:** implementation prepared; execution evidence pending exact-head CI.

**Scientific base:** `1e2b7d94e970256b38bda15fa91f62bc397e825a`

**Harness:** `crates/medscale-core/tests/paper_authority_effects.rs`

## Research purpose

B0 tests whether proposal authority, review/promotion authority, external-action intent, and observed effect state remain distinct in the production Core path under strict client-session enforcement.

## Pre-specified cases

| Case | Fault / control | Expected behavior |
|---|---|---|
| B0-01 | session can create a proposal but lacks promotion capability | promotion refused with `SessionDenied`; no external-action intent appears |
| B0-02 | separately granted promotion capability | resulting assertion records the originating proposal id and reviewer/authorizer identity |
| B0-03 | valid external-action intent | starts `Pending` and binds exact payload digest |
| B0-04 | attempt `Sent` on a generic audit object without payload binding | explicit invalid-argument refusal |
| B0-05 | effect transitions `Sent -> Unknown -> Pending` without reconciliation token | blind retry refused with `UnknownRequiresReconcile`; explicit reconciliation is the matched control |
| B0-06 | NPHIES invocation with session capability but without the separately required workflow-evidence gate | `ExternalGateRequired` |

## Interpretation boundary

B0 tests authority/effect conformance on admitted Core contracts. It does not prove that a real external partner received or executed an action, does not qualify NPHIES transport, and does not establish clinical correctness. `Pending`, `Sent`, `Confirmed`, `Failed`, and `Unknown` are system effect states, not clinical outcome labels.

## Planned command

```bash
cargo test -p medscale-core --test paper_authority_effects
```

The experiment remains `PENDING` until exact-head CI is complete and the observed result is captured.
