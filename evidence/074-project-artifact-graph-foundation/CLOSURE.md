# CLOSURE — Spec 074 Project + Artifact Graph Foundation

## Terminal truth

```text
SPEC_074_CLOSED_CANONICAL=true
MERGE_SHA=3d59255d0f37800cdda85dd4a7f12238356b02ad
POST_MERGE_MAIN_CI=35309949710 (6/6 success on the exact merge commit)
MIGRATION_RECOVERY=PASS (storage suite incl. v2->v3 identity preservation,
  interrupted-journal fail-closed, backup/restore round-trip, reopen durability)
CORE_CLI_DESKTOP_PARITY=PASS (one Core authority path; CLI/Desktop render
  typed Core results; dependency-direction gate holds)
SECURITY_ADVERSARIAL=PASS (T1..T12 mapped to tests in SECURITY_ADVERSARIAL.md)
SCALE_EVIDENCE=evidence/074-project-artifact-graph-foundation/SCALE_MEASUREMENTS.md
  (smoke shapes green in every CI run; lab-full correctness passed; reopen
  measured; CI full-scale partials show zero failures before the clock;
  full-fixture timings are local-only evidence pending toolchain recovery)
SPEC_075_IMPLEMENTATION_AUTHORIZED=false (no promotion exists; candidates
  075+ remain planning-only)
RESEARCH_OS_COMPLETE=false
REAL_PHI_AUTHORIZED=false
RELEASE_READY=false (unchanged by this lane)
PRIVATE_DATA_READY=false (unchanged by this lane)
MULTI_CLIENT_RELEASE_READY=false (unchanged by this lane)
```

## What this closure establishes

MedScale has a durable local Project/Experiment/Artifact Graph organizing
foundation for the qualified scope: create/archive Projects and Experiments,
attach references to existing canonical objects without copying authority
payloads, relate them with a bounded organizational predicate vocabulary,
resolve bounded contexts, inspect everything through Core/CLI/Desktop,
migrate pre-074 vaults without identity loss, and recover from crashes and
backups.

## What this closure does NOT establish

Team collaboration, MedAgent, Fleet, Privacy Gate expansion, Browse,
AudioFlow, Analytics, RAG, Hub, Compute, Research Packs, federation,
real-PHI readiness, product release readiness, or Research OS completion.

## Closure trail

```text
PR_122_MERGE=3d59255d0f37800cdda85dd4a7f12238356b02ad
POST_MERGE_MAIN_CI=35309949710 (6/6)
PR_126_QUEUE_MARK_MERGE=79e8475c5ac6da304c3c9d9fa60eadd6e0979511
PR_126_POST_MERGE_MAIN_CI=35318563856 (6/6)
```

## Open external residuals (not repository-owned work)

- `LOCAL_WINDOWS_TOOLCHAIN_DESTRUCTION_2026_09_18` (`OPEN_WORKSTATION_ONLY`):
  local link/C builds impossible until a human restores the toolchain.
  CI remains the authoritative qualification path.
- Full-fixture scale TIMINGS ride the perf job automatically from the
  closure commit onward; numbers bind into SCALE_MEASUREMENTS.md from CI logs.
- All other open gates are the standing ones in
  `docs/planning/EXTERNAL_GATES.md` (unchanged by this lane).
