# Security and Threat Delta — Spec 077

**Status:** implementation contract

Spec 077 adds local-first agent execution (identity, bounded context,
typed tool invocation, run lifecycle, receipts, evidence-only proposal
output). It adds no new product runtime network path, no remote/bounded
worker, no browser/external-provider capability, no Hub sync execution,
and no real-PHI authorization.

## 1. Trust boundary

```text
CLI/Desktop
   -> typed Core command/query
      -> actor/session/scope checks (Core Capability, unchanged mechanism)
         -> AgentCapabilityManifest check (which tool kinds this identity
            may ever request)
            -> ContextManifest boundary check (what this run may read)
               -> input + revision validation
                  -> encrypted storage transaction
                     -> RunReceipt / tool receipt append (same transaction
                        as the mutation it records)
```

No CLI/Desktop component may access 077 tables directly. No agent-run
mutation may call any other subsystem's mutation path (effects, actions/
outbox, proposal promotion, clinical-assertion creation) directly or
transitively, except the one explicit `Proposal`/`CreateProposal` submission
path this spec's `AgentProposal` design decision reuses.

## 2. Threats and required controls

### T1 — Agent-output-triggers-effect escalation

Attack: agent output (a proposal, a tool result) is wired, now or by a
future careless change, to automatically call `PromoteProposal`,
`TransitionEffect`, `CreateExternalActionIntent`, or any clinical/external
effect, so that a model's text silently performs a canonical or external
action.

Controls:

- `AgentProposal` recording is a pure `Proposal`-class write (via the
  existing `CreateProposal` capability) plus `RunReceipt`/tool-receipt
  append; the Core function that records it has no code path to any
  effect/action/`ClinicalAssertion`-creation/proposal-promotion function,
  verified structurally by dependency-direction review, not just by test;
- distinct-and-visible in the type system: nothing in 077's contracts maps
  1:1 to `EffectState`/`ClinicalAssertion`; any future spec wiring agent
  output to a real effect must add an explicit, separately-authorized Core
  capability, not overload this one;
- the acceptance test suite includes a structural check that
  `authority::medagent` (077's Core module) contains no call into
  `authority::promote`, `authority::amend`, or `contracts::actions`
  (mirroring Spec 076's T1 discipline exactly).

Tests: recording an `AgentProposal` produces no side effect outside 077
tables + the one `Proposal` submission + `RunReceipt`; a direct unit test
asserting no cross-module call exists.

### T2 — Tool invocation escapes its granted capability

Attack: a model requests a tool kind its `AgentCapabilityManifest` does not
grant, or requests a tool with arguments that reach outside its bound
`ContextManifest`, and Core executes it anyway.

Controls:

- every `ToolInvocation` is checked against the run's
  `AgentCapabilityManifest` before dispatch; an ungranted tool kind is
  refused (`Unauthorized`) before any execution, never silently
  downgraded to a no-op that still leaks a result;
- every context-reading tool argument (e.g., "read artifact X") is checked
  against the run's bound `ContextManifest`; an artifact outside the
  manifest is refused, never silently substituted with the "nearest"
  in-context artifact;
- tool execution runs entirely inside Core, never inside model-generated
  code; the model can only name a tool and supply typed arguments.

Tests: an ungranted tool kind is refused and recorded as a refusal; an
in-context-boundary tool call succeeds; an out-of-context artifact request
is refused with the same discipline as a cross-room read in Spec 076's T3.

### T3 — Model output executed as code

Attack: agent output (free text, a tool-call payload) is interpreted as a
shell command, SQL fragment, or Rust expression rather than treated purely
as data.

Controls:

- no 077 code path passes model-generated text to `std::process::Command`,
  a SQL string-builder, `eval`-equivalent, or any dynamic-dispatch
  mechanism keyed by model output;
- tool arguments are deserialized into closed, typed Rust structs (`serde`
  with `deny_unknown_fields`) before any use; a malformed/unexpected
  argument shape is a typed rejection, never partially parsed and used.

Tests: fuzzed/malformed tool-argument payloads are rejected, not partially
executed; a tool-argument value containing shell/SQL-metacharacter-like
text is treated as inert data end to end (mirrors Spec 076's T9 discipline).

### T4 — Context-boundary leakage / ambient vault access

Attack: an agent run reads vault content outside its bound
`ContextManifest` via a code path that does not go through the explicit
context-check boundary (a "just this once" convenience read).

Controls:

- there is exactly one Core-internal function an agent run's tool
  dispatch may call to resolve artifact content, and it always consults
  the bound `ContextManifest` first; no second, unchecked read path exists
  for agent-run code;
- reviewed structurally (dependency-direction / call-graph check), not
  merely asserted by test.

Tests: a run bound to `ContextManifest{A, B}` cannot resolve artifact `C`
through any exposed tool, even one that would trivially succeed for a
human operator with full Project access.

### T5 — Stale/revoked agent identity retains execution rights

Attack: a revoked `AgentIdentity`, or one whose bound model Pack was later
un-admitted, continues to start or continue runs.

Controls:

- `AgentIdentity.status` and the bound Pack's admission state are checked
  on every run-start and every turn continuation, not cached across a run's
  lifetime;
- revocation/un-admission is immediate and does not require a new session
  to take effect, matching the existing `SessionRegistry` per-request
  validation discipline and Spec 076's T4 precedent.

Tests: a run cannot be started against a revoked identity; a run already
`Running` when its identity is revoked is denied its next turn and
transitions to `Failed`, never silently continuing.

### T6 — Stale-write overwrite on mutable 077 state

Attack: two callers update an `AgentIdentity`, `ContextManifest`, or
`AgentRun` from the same old state and the later request silently
overwrites the newer result.

Controls:

- explicit `expected_revision` precondition on every mutable 077 row;
- mismatch -> `Conflict`, no write, no last-write-wins;
- `AgentRunState` transitions are additionally gated by the frozen
  transition table (`migration.md` section 13), so even a revision-matched
  request cannot force an illegal state edge.

Tests: stale identity/manifest/run update all `Conflict`; an illegal state
transition (e.g. `Completed -> Running`) is rejected even with a correct
`expected_revision`.

### T7 — Cancellation race / zombie run

Attack: a cancel request races a concurrently-completing run, and the run
ends up either not actually stopped (continues issuing tool calls after
the caller was told it was cancelled) or lands in an ambiguous/dual
terminal state.

Controls:

- cancellation is a revision-guarded transition like any other; the first
  transition to reach a terminal state wins and the losing side's
  `expected_revision` mismatches, receiving `Conflict` rather than
  silently double-applying;
- once `Cancelled`, no further `ToolInvocation` may be dispatched for that
  run -- checked at dispatch time, not only at run-creation time.

Tests: a simulated race between a cancel request and a completing turn
proves exactly one terminal state is reached, never both, and never a
`Running` run that keeps accepting new turns after `Cancelled`.

### T8 — Malformed/hostile metadata injection

Attack: oversized/control-character/markup/SQL-like prompts, tool
arguments, or proposal payloads cause storage/query/UI issues.

Controls:

- bounded UTF-8 validation with explicit constants on every free-text
  field (prompt text, proposal payload strings, tool-argument strings);
- parameterized storage APIs (no string-built SQL);
- UI/CLI renders text as text, never as executable markup;
- logging safely escapes/bounds user/model metadata; prompt/proposal
  bodies never appear unbounded in operational logs.

Tests: empty/whitespace, max boundary, over-max, unusual Unicode, control
characters, SQL-like strings per frozen policy (mirrors Spec 076's T9
suite).

### T9 — Audit tampering / run-history rewrite

Attack: a `RunReceipt`, `ToolReceipt`, or `AgentTurn` is edited or deleted
out of band to hide what an agent actually did.

Controls:

- `AgentTurn`/`ToolInvocation`/`ToolReceipt`/`RunReceipt` rows have no
  update API; storage exposes append/terminal-write and read only;
- migration/backup/restore evidence includes a re-verification pass proving
  a terminal run's `RunReceipt` still matches its recorded tool-invocation
  history after restore (learning directly from Spec 076's exact-range
  review finding that its own restore path skipped exactly this kind of
  re-verification -- 077 must not repeat that gap).

Tests: tamper with a stored `RunReceipt`/`ToolReceipt` row directly in a
test fixture and prove the mismatch is detectable; restore-from-backup
re-verifies run/receipt consistency and fails closed on mismatch.

### T10 — Content leakage into logs or cross-scope caches

Attack: prompt text, tool-argument content, or proposal payloads (which may
carry sensitive research/clinical discussion) leak into operational logs,
crash dumps, or a shared cache visible across authorization scopes.

Controls:

- logging conventions already required elsewhere apply identically:
  free-text bodies are never logged in full at info/debug level by
  default;
- 077 introduces no new shared cache; reads are scoped per run's
  `ContextManifest`, not memoized across authorization contexts.

Tests: debug-log capture scans across every 077 mutation path assert no
raw prompt/proposal/tool-argument body appears unbounded in log output.

### T11 — Half-committed state after crash

Attack: a run's terminal-state write commits but its `RunReceipt` append
does not (or the reverse), leaving an untraceable run or an orphaned
receipt.

Controls:

- every terminal-state transaction includes its `RunReceipt` append in the
  same commit (`migration.md` section 5); there is no 077 write path that
  commits one without the other;
- migration journal fail-closed semantics from the existing framework are
  preserved for v5 -> v6.

Tests: crash-point matrix from `migration.md` section 6, including "before
`RunReceipt` commits."

### T12 — Dependency and supply-chain smuggling

Attack: 077 pulls in a new dependency (e.g. for a general-purpose local
LLM inference engine) that introduces copyleft/transitive risk, unreviewed
native-code execution, or unqualified behavior.

Controls:

- 077's foundation requires no new dependency beyond what
  `medscale-pack`/Spec 008/069 already qualifies: the model execution
  surface is the existing `PackRuntimeAdapter`/`OnnxTokenClassifierRuntime`
  path, not a new inference stack;
- if a later slice genuinely needs a new dependency, exact version/
  license/security/exit review is recorded in the evidence packet before
  admission, and `cargo-deny`/supply-chain gates must pass on the exact
  head.

Tests: dependency-direction and supply-chain gates green; if `Cargo.toml`
gains any new dependency, a review record exists explaining why the
existing `medscale-pack` runtime was insufficient.

## 3. Explicit non-capabilities

077 introduces no capability for: triggering a clinical/external effect
from agent output, ambient vault access outside a bound `ContextManifest`,
browser/network tool access, agent self-registration or self-escalation,
multi-agent orchestration, remote/bounded-worker run execution, or a second
model-inference engine. Any test or review finding suggesting such a
capability is a blocking defect, not a scope question.
