# AGENTS.md — MedScale Repository Execution Rules

## Authority order

1. Explicit founder instruction and `docs/planning/IMPLEMENTATION_AUTHORITY.md`.
2. Live GitHub/repository truth for mutable state.
3. Canonical planning/spec/task artifacts in this repository.
4. Frozen MedScale architecture and contracts.
5. Primary standards/upstream evidence within their scoped subject.
6. Historical planning/review material.

Never use a stale handoff or remembered SHA over live repository truth.

## Active execution state

```text
PHASE = AUTONOMOUS_IMPLEMENTATION_AUTHORIZED
CURSOR_IMPLEMENTATION_AUTHORITY = YES_WITHIN_CANONICAL_V2_PLAN
ASK_FOUNDER_FOR_ORDINARY_IMPLEMENTATION_DECISIONS = NO
CONTINUE_TO_NEXT_ELIGIBLE_UNIT = YES
REAL_PHI_AUTHORITY = NO
UNRELATED_REPOSITORY_MUTATIONS = NOT_AUTHORIZED
PRODUCT_RUNTIME_NETWORK_EGRESS = DEFAULT_DENY
```

Earlier planning files may contain historical metadata saying implementation was not authorized. `docs/planning/IMPLEMENTATION_AUTHORITY.md` explicitly supersedes those authorization-only lines. Their architecture and scope rules remain canonical.

## Mandatory execution entry point

Cursor must read `CURSOR.md`, `docs/planning/START_HERE.md`, `docs/planning/BUILD_QUEUE.md`, and the current owning Spec Kit package before material work.

MedScale uses GitHub Spec Kit and a Spec-of-Specs process. For each material executable spec:

```text
/speckit.specify
/speckit.clarify
/speckit.plan
/speckit.checklist
/speckit.tasks
/speckit.analyze
-> qualification
-> /speckit.implement
-> /speckit.converge
```

Use `/speckit.constitution` when repository constitution is created/changed. Spec 000 is already `CLOSED_CANONICAL` at planning/source-authority level; Spec 001 is also closed. Use the live queue for current work; do not restart bootstrap. Scoped closure does not establish product or privacy release readiness.

## Autonomous behavior

Do not ask the founder for routine implementation choices already covered by the plan. Resolve them using `IMPLEMENTATION_DECISION_DEFAULTS.md`, primary evidence, tests, and the safest minimal design. Record material decisions in the owning spec/research/ADR and continue.

When a unit becomes `CLOSED_CANONICAL`, immediately recompute `BUILD_QUEUE.md` and start the next eligible unit. Do not stop after one task, PR, milestone, phase, or passing demo.

If a real human/external gate is required, record it in `EXTERNAL_GATES.md`, continue all independent work, and do not reinterpret the gate as project-wide permission to stop.

## Non-negotiable architecture

- Rust is the primary and trusted-core language.
- Local-first and privacy-first are constitutional.
- Useful core operation requires neither cloud account nor remote model.
- No hidden telemetry/remote logging/crash upload is required.
- Source bytes != derived representation; source identity != content hash.
- Durable object classes remain distinct. `Proposal != ClinicalAssertion`.
- Identity merge is explicit; projections are rebuildable/non-authoritative.
- FHIR R4 4.0.1 is initial interchange, not canonical DB; validator output is evidence only.
- One Rust authority path serves all product surfaces.
- Workers receive no ambient canonical DB, master keys, unrestricted filesystem, network, secrets, or authority.
- Unrelated repositories are outside MedScale execution authority. Legacy interoperability contracts do not authorize cross-repository changes.
- External-action `UNKNOWN` is never blindly retried.

## Donor and source rule

`docs/planning/SOURCE_ACQUISITION_AND_COPY_PLAN.md` is mandatory before donor use. Use permitted sources through the documented MedScale acquisition process. Every copied/ported/vendor component must bind exact upstream URL/revision/path, rights/license/NOTICE, content identity, local target, modifications, tests, security placement, maintenance/update/exit strategy, and SBOM/provenance.

Permission to copy code does not imply permission to redistribute model weights, datasets, licensed terminology, or other assets.

## H0 boundary

H0-A/H0-B are synthetic-only and LLM-free. No real PHI, unqualified models, external research-project integration, cross-repository mutation, OCR/ASR/vector/agents, live SMART/NPHIES, external actions, or product runtime network.

## Mutation discipline

Before each material mutation, re-read live branch/head, active PRs, changed files, CI/review state, current spec/tasks, and dependency gates. No force-push, destructive rebase/history rewrite, skipped tests, or unsupported PASS claims.

## Graft — repository context layer

Use Graft (https://github.com/trailhq/Graft, `@nanonets/graft`) as local developer/agent tooling for repository context and navigation. This does not authorize Graft, a remote model, network egress, or any related mechanism as a MedScale product/runtime dependency.

This policy section is maintained by MedScale and deliberately sits **outside** Graft's managed markers. `graft init` owns and rewrites only the region between its own `graft:start` / `graft:end` HTML-comment markers (upstream `src/hosts/sections.ts`), so any block it adds below must not be relied on for MedScale policy.

Before installing or running Graft, disable its anonymous usage telemetry. The published npm package sends usage events, including one from its install hook, unless opted out (upstream `TELEMETRY.md`). Set `DO_NOT_TRACK=1` in the environment before `npm install`, then run `graft telemetry disable`, and leave "anonymous usage stats" unchecked in the `graft init` picker.

If Graft is unavailable or the local `graft/` graph is absent/stale, run `graft init`, select the active agent(s), then run `graft build`. Before broad source exploration prefer `graft check`, `graft map`, `graft ask "<question>" --source`, `graft skeleton <file>`, `graft callers <symbol>`, and `graft grep "<literal>"`. After material code changes, run `graft build` again.

Treat `graft/` as a local regenerable cache; it is excluded by `.gitignore` and must not be committed. Keep usage zero-cost and consistent with MedScale's local-first/network boundaries: do not introduce paid model/API usage; any model-backed enrichment requires separate existing authorization and must use an already-authorized local or free provider.

Graft is context/navigation, not correctness, clinical evidence, or qualification evidence. Continue all repository-required tests, Spec Kit gates, Jev review/qualification where applicable, Alibaba Open Code Review, CI, security, provenance, and privacy checks. Never fabricate Graft output, tool execution, CI, reviews, or evidence.
