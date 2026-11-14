# Local Qualification — Spec 063

Date: 2026-09-15
Platform: macOS development host; canonical `spec/063-workflow-studio-tasks-messages` branch based on main merge `dd3f6bd1c35afa53ae9a3cf83aad0e3e769c76ce`.

## Results
- `cargo fmt --all -- --check`: PASS.
- `cargo test -p medscale-desktop --locked`: PASS (12 unit tests plus 3 runtime-performance tests).
- `cargo test -p medscale-core --test native_workflow_studio_063 --locked`: 4/4 PASS.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS.
- `cargo test --workspace --locked`: LOCAL HOST STORAGE LIMIT. The observed canonical-branch attempt reached test-binary linking after Clippy and targeted/Desktop tests passed, then stopped with `ENOSPC`; no test assertion failed. Full workspace tests remain mandatory in exact-head CI before merge.
- `cargo +1.88.0 check --workspace --all-targets --locked`: PASS.
- `cargo +1.88.0 run -p medscale-desktop --locked -- --smoke`: PASS (`medscale-desktop smoke ok`, `tauri_admitted=false`, `desktop_shell=slint_native`).
- `cargo +1.88.0 run -p medscale-desktop --locked -- --perf-idle-ms 100`: PASS (`medscale-desktop perf-idle-ready`).
- `git -c core.whitespace=cr-at-eol diff --check`: PASS.

The local host has limited free disk space. The full-workspace test attempt stopped during linking with `ENOSPC`; this is recorded as an environment limitation, not as a passing local workspace-test result. Only the untracked MedScale `target/` build cache was removed to recover disk space. No tracked repository content or external project artifact was deleted. Exact-head CI must supply the full-workspace test evidence before merge.

Exact-head three-OS CI/package qualification, merge, post-merge main verification, and canonical closure remain required. This local qualification does not establish release readiness, qualified-hardware budget attainment, WCAG conformance, signing/notarization, real-PHI authorization, or MESC admission.
