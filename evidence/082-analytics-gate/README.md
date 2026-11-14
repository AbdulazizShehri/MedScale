# Evidence — Spec 082 Analytics Gate

Files: README, LIVE_TRUTH, QUALIFICATION (contracts, engine, storage, Core,
CLI and Desktop evidence in one file), SECURITY, EXACT_RANGE_REVIEW,
EXACT_HEAD_QUALIFICATION, and after merge POST_MERGE_VERIFICATION and
CLOSURE.

All inputs are synthetic CSV files imported as Spec 075 snapshots. GitHub
Actions CI is the authoritative qualification path; the local workstation
cannot link Rust on Windows, and local WSL builds were stopped for memory
pressure during Specs 080-081.
