# Evidence — Spec 080 Governed Browse

Files: README, LIVE_TRUTH, QUALIFICATION (contract, network/SSRF, storage,
Core, CLI and Desktop evidence in one file), SECURITY_ADVERSARIAL,
EXACT_RANGE_REVIEW, EXACT_HEAD_QUALIFICATION, and after merge
POST_MERGE_VERIFICATION and CLOSURE.

Hermetic tests use a scripted, socket-free transport. GitHub Actions CI is
the authoritative qualification path (the local Windows toolchain cannot
link, and the local WSL distro has been unavailable since 2026-09-23).
