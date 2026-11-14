# Evidence — Spec 074 Project + Artifact Graph Foundation

This directory contains proof for the exact revisions used to implement and close Spec 074. Do not paste unverified claims here.

## Required evidence set

Create/update these artifacts as work progresses; use the repository's established evidence conventions if live truth prefers different filenames:

```text
LIVE_TRUTH.md
CONTRACT_QUALIFICATION.md
STORAGE_MIGRATION_RECOVERY.md
CORE_AUTHORITY_QUALIFICATION.md
CLI_QUALIFICATION.md
DESKTOP_QUALIFICATION.md
SECURITY_ADVERSARIAL.md
SCALE_MEASUREMENTS.md
EXACT_RANGE_REVIEW.md
EXACT_HEAD_QUALIFICATION.md
POST_MERGE_VERIFICATION.md
CLOSURE.md
logs/
```

## Evidence rules

Every qualification artifact must bind as applicable:

- exact commit SHA;
- canonical base/merge-base SHA;
- branch;
- command executed;
- host OS/architecture/toolchain;
- fixture version/digest;
- start/end or captured run identity where useful;
- result (`PASS`, `FAIL`, `SKIPPED`, `NOT_RUN`, `UNAVAILABLE`, `UNKNOWN`);
- limitations;
- owning acceptance criterion/task.

Never convert `SKIPPED`, `NOT_RUN`, `UNAVAILABLE`, `UNKNOWN`, a green compile, or a planning statement into `PASS`.

## LIVE_TRUTH.md

Record at T074-00:

```text
MAIN_SHA=
BRANCH=
HEAD_SHA=
MERGE_BASE_SHA=
SPEC_074_PROMOTION_PRESENT=
OPEN_PR_STATE=
BASELINE_CI_STATE=
PREEXISTING_FAILURES=
REAL_PHI_USED=false
```

## CONTRACT_QUALIFICATION.md

Map the frozen contract fields/invariants to exact Rust paths and focused tests. Include the freeze block from `contracts.md`.

## STORAGE_MIGRATION_RECOVERY.md

For each fixture, record before/after schema/version, stable object identity checks, backup checkpoint, forward migration, old workflow regression, 074 workflow, reopen, repeated open/migration and restore result.

## CORE_AUTHORITY_QUALIFICATION.md

Map every mutation/query to actor/session/scope/precondition/validation/storage/audit behavior. Include denial/conflict/cross-scope/idempotency/corrupt cases.

## CLI_QUALIFICATION.md

Record human + JSON behavior, exit/error semantics and Core parity for representative Project/Experiment/ref/edge commands.

## DESKTOP_QUALIFICATION.md

Record native state/adapter/accessibility qualification and rendered evidence according to the current MedScale UI evidence practice. Do not use mock/fake capability as proof.

## SECURITY_ADVERSARIAL.md

At minimum close every security gate listed in `security.md`, including cross-scope reference, stale write, unknown predicate, graph bound, permission-filtered context/summary, archive non-cascade and no new runtime egress.

## SCALE_MEASUREMENTS.md

Bind the Personal and Lab synthetic fixture definitions, exact generator/config, hardware, measurements and correctness result. Report measurements without turning them into universal marketing budgets.

## EXACT_RANGE_REVIEW.md

Review the complete diff from the verified canonical base/merge base to candidate head. State:

```text
AUTHORIZED_SCOPE_ONLY=true|false
UNEXPECTED_FILES=
NEW_DEPENDENCIES=
NETWORK_AUTHORITY_CHANGE=false|...
REAL_PHI=false
MESC_CHANGE=false
SPEC_075_PLUS_IMPLEMENTATION=false
```

Any unexpected material scope must be removed or canonically authorized before merge.

## EXACT_HEAD_QUALIFICATION.md

Bind the exact candidate head and every live required CI/check run. All required gates must be completed successfully; pending is not pass.

## POST_MERGE_VERIFICATION.md

Bind merge SHA and post-merge main checks. Closure waits for this evidence.

## CLOSURE.md

Only after all repository-owned 074 work is proven and post-main verification succeeds:

```text
SPEC_074_CLOSED_CANONICAL=true
SPEC_075_IMPLEMENTATION_AUTHORIZED=false
RESEARCH_OS_COMPLETE=false
REAL_PHI_AUTHORIZED=false
```

List anything deferred/unqualified rather than hiding it.