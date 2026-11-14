# T077-01 Gap Closure — Contracts freeze record reconciliation

## What was found

While reading `contracts.md` to prepare T077-03 (Core wiring for
`AgentIdentity`/`AgentCapabilityManifest`), this session found that
`specs/077-medagent-workbench/contracts.md` section 8 ("Freeze record,
filled in against real Rust source") was still the literal
pre-implementation template placeholder text (`<exact path, e.g. ...>`,
`<exact name/value>`, `<exact count>`), never actually filled in against
`crates/medscale-contracts/src/medagent.rs` after commit `0e6c210` froze
the real contracts.

Cross-checking sections 4 and 6 against the real Rust struct definitions
also found two fields present in the pre-implementation sketch that do not
exist in the frozen source:

- `AgentRun.started_at_seq: Option<u64>` / `ended_at_seq: Option<u64>` --
  absent from the real `AgentRun` struct.
- `ToolReceipt.resolution: Option<ReferenceResolution>` -- absent from the
  real `ToolReceipt` struct.

Neither field is referenced by any acceptance requirement in
`security.md`, `migration.md`, `spec.md`, or `plan.md` (grepped this
session); they appear to have been dropped as a reasonable simplification
during implementation, but the spec document was never updated to say so,
which is itself the kind of stale-checkbox/stale-doc drift this spec's own
T077-00 discipline exists to catch.

## What was done

- `contracts.md` sections 4 and 6 now show the real struct shape and carry
  an explicit "T077-01 reconciliation" note explaining why each field was
  dropped (run ordering is already recoverable from `AgentTurn.seq` +
  `RunReceipt`; artifact-resolution evidence for `ReadContextArtifact` can
  live inside that tool kind's typed `result: Value` without forcing every
  other `ToolKind`'s receipt to carry an always-`None` column) and what a
  later task must still decide (T077-06 must record exactly what
  `ReadContextArtifact`'s result payload contains).
- `contracts.md` section 8 is now filled in with the real module path,
  schema version constant, bound constants, enum vocabularies, validation
  helper convention, the exact 10-test list, and an explicit
  `FIELD_DRIFT_FROM_PRE_IMPLEMENTATION_SKETCH` line naming this reconciliation.

No Rust code changed in this pass -- this was a documentation/spec
reconciliation only, since the actual frozen contracts (already CI-green
on this branch) are what every other 077 task and its tests are correctly
building against. Deliberately not adding the two dropped fields back
speculatively: neither is required by any frozen acceptance bullet, and
the architectural discipline this codebase follows (see `CLAUDE.md`-level
guidance reflected throughout this repository's own specs) is to avoid
fields with no current caller.
