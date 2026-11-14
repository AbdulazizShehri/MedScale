# Evidence — Spec 085 MedScale Compute

Files: README, LIVE_TRUTH, QUALIFICATION, SECURITY, EXACT_RANGE_REVIEW,
EXACT_HEAD_QUALIFICATION, and after merge POST_MERGE_VERIFICATION and
CLOSURE.

All data is synthetic: small CSV files imported as Spec 075 snapshots in
per-test vaults. Jobs run in the real `medscale-compute-worker` process;
fault cases name the qualification harness `medscale-compute-fault-worker`
through an explicit runtime. There is no network code and no arbitrary
code path. GitHub Actions CI is the authoritative qualification path; the
local workstation cannot link Rust on Windows.
