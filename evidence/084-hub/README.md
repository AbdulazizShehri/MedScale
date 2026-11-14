# Evidence — Spec 084 MedScale Hub Foundation

Files: README, LIVE_TRUTH, QUALIFICATION, SECURITY, EXACT_RANGE_REVIEW,
EXACT_HEAD_QUALIFICATION, and after merge POST_MERGE_VERIFICATION and
CLOSURE.

All data is synthetic: three synthetic vaults (a Hub and two clients) in
one test process, device keys generated per run, and collaboration rows
created through the Spec 076 authority. Transport is in-process (JSON round
trip) and the Spec 024 local socket; there is no network code. GitHub
Actions CI is the authoritative qualification path; the local workstation
cannot link Rust on Windows.
