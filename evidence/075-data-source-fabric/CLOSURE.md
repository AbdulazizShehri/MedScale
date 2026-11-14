# CLOSURE — Spec 075 Data Source Fabric + Data Workbench Foundation

## Terminal truth

```text
SPEC_075_CLOSED_CANONICAL=true
MERGE_SHA=89a88cfbbe7b67582886fb07fb6de4fd9ca28dff
POST_MERGE_MAIN_CI=35582548200 (6/6 success on the exact merge commit)
EXACT_HEAD_CI=35580861670 (6/6 success on head 9f84a6e, PR #129)
MIGRATION_RECOVERY=PASS (storage suite incl. v3->v4 additive migration,
  repeated-open safety, interrupted-journal fail-closed, backup/restore
  round-trip incl. tampered-credential-on-restore rejection, corrupted
  id-sequence fail-closed; see STORAGE_MIGRATION_RECOVERY.md)
CORE_CLI_DESKTOP_PARITY=PASS (one Core authority path; CLI/Desktop render
  typed Core results; dependency-direction gate holds)
SECURITY_ADVERSARIAL=PASS (T1..T13 mapped to tests in SECURITY_ADVERSARIAL.md,
  including two real findings the exact-range review surfaced and this
  closure fixed: a vault-metadata-store cross-scope read escape (T3) and a
  credential-reintroduction gap on backup restore (T2))
EXACT_RANGE_REVIEW=evidence/075-data-source-fabric/EXACT_RANGE_REVIEW.md
  (18 confirmed findings; material ones fixed with regression tests in
  aa771e9; non-defects and deliberately out-of-scope items recorded with
  reasoning, not silently dropped)
DESKTOP_COVERAGE_RESIDUAL=no rendered PNG evidence and no headless
  AppWindow test harness exist in this codebase for Desktop UI logic;
  two Desktop-only exact-range-review fixes were verified by tracing
  property flow, not by execution (see DESKTOP_QUALIFICATION.md)
SPEC_076_PLUS_IMPLEMENTATION_AUTHORIZED=false (no promotion exists;
  candidates 076+ remain planning-only pending fresh promotion)
RESEARCH_OS_COMPLETE=false
REAL_PHI_AUTHORIZED=false
RELEASE_READY=false (unchanged by this lane)
PRIVATE_DATA_READY=false (unchanged by this lane)
MULTI_CLIENT_RELEASE_READY=false (unchanged by this lane)
```

## What this closure establishes

MedScale has a durable local Data Source Fabric and native Data Workbench:
CSV/TSV local file sources and one qualified read-only external-SQLite
database adapter, immutable content-addressed DataSnapshots with exact
lineage, saved Grid views, a frozen deterministic transformation set
producing new immutable snapshots, versioned Dataset Card releases scoped
to a Project, and a CLI/Desktop surface backed exclusively by Core
authority. Malformed/quarantined/missing/oversized input, stale-revision
conflicts, and cross-scope access all fail closed with typed errors. A
full exact-range semantic review of the entire PR (not just the two
CI-failure fixes that opened this closure) found and fixed a real
authority-scope-bypass read path and a real credential-reintroduction path
on backup restore before merge; see EXACT_RANGE_REVIEW.md for the complete
finding-by-finding disposition, including what was deliberately left
unchanged and why.

## What this closure does NOT establish

Remote dataset acquisition (Hugging Face/Kaggle) beyond its fail-closed
deny-before-socket posture: the code path, parsing, and hash-verification
logic exist and are unit-tested, but no live successful fetch was ever
exercised (the Network Broker's allowlist is empty by default and
`UreqTransport::send` returns `ExternalGateRequired` unconditionally in
this repository state). JSON/JSONL local files are qualified; Parquet,
Arrow IPC/Feather, and XLSX remain explicitly `Unsupported` pending a
separate slice. Postgres/MySQL/SQL Server database engines remain
explicitly `Unsupported` pending separate qualification. Rendered Desktop
visual evidence. Team collaboration, MedAgent, Model Fleet, Privacy Gate
expansion, Governed Browse, AudioFlow, Analytics/Cohort Builder, Clinical
Graph/Research Canvas, Hub, Compute, R Workspace, Community Extensions,
Research/Evidence Packs, Institutional Adapters, federation, real-PHI
readiness, product release readiness, or Research OS completion.

## Closure trail

```text
PR_129_MERGE=89a88cfbbe7b67582886fb07fb6de4fd9ca28dff
POST_MERGE_MAIN_CI=35582548200 (6/6)
EXACT_HEAD_CI=35580861670 (6/6, head 9f84a6e)
FIRST_EXACT_HEAD_CI_ATTEMPT=35422433021 (FAILED: 2 real Core test
  failures on all 3 platforms - AcquireFail classification bugs)
FIX_1=b6de6a2 (AcquireFail::Rejected -> Quarantined for malformed UTF-8;
  AcquireFail::Missing -> Unavailable for a missing DB object within a
  present DB file)
SECOND_CI_ATTEMPT=35573757788 (FAILED on a different, previously-masked
  bug: crates/medscale-storage/tests/project_graph_074.rs pinned the
  pre-075 finished_version at 3; reached only because FIX_1 let cargo's
  fail-fast progress past the earlier failure)
FIX_2=bbc4d89 (update the three stale finished_version==3 assertions to 4)
THIRD_CI_ATTEMPT=35574786250 (PASSED 6/6, but not yet exact-range reviewed)
EXACT_RANGE_REVIEW_FIX=aa771e9 (18 confirmed findings from /code-review
  high over the full PR range; material ones fixed with regression tests)
EVIDENCE_DOC=9f84a6e (this evidence packet updated to match)
FINAL_EXACT_HEAD_CI=35580861670 (head 9f84a6e, PASSED 6/6)
```

## Open external residuals (not repository-owned work)

- `LOCAL_WINDOWS_TOOLCHAIN_DESTRUCTION_2026_09_18` (`OPEN_WORKSTATION_ONLY`,
  `docs/planning/EXTERNAL_GATES.md`): local link/C builds remain impossible
  on this workstation. Every fix in this closure was verified by tracing
  exact call sites and error-mapping chains against the real code, then
  qualified for real on GitHub Actions CI - never claimed as locally
  tested. CI remains the authoritative qualification path.
- No rendered Desktop screenshots and no headless `AppWindow` test harness
  exist in this codebase (confirmed at close, not merely assumed); tracked
  for whichever future spec adds Desktop UI-logic/rendering test
  infrastructure.
- Remote dataset acquisition's live-fetch path (identity/version/resumable
  download/hash-verification against a real Hugging Face/Kaggle endpoint)
  has never been exercised end-to-end; only its fail-closed deny-before-
  socket posture and its unit-tested parsing/merging logic are proven.
- All other open gates are the standing ones in
  `docs/planning/EXTERNAL_GATES.md` (unchanged by this lane).
