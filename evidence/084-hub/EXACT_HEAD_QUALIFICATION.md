# Exact-Head Qualification — Spec 084

```text
BASE_SHA   = 2892860e64ffed9ff9ae6387f681f1657730e19c (PR #145 merge)
BRANCH     = spec/084-hub (PR #147)
CODE_HEAD  = 785810645e965b3d9db9191ff6df0ad440a6ffd7
CI_RUN     = 36023022359 (pull_request), 6/6 success, bound to CODE_HEAD
TOOLCHAIN  = rustc/cargo 1.97.1
BASE POST-MAIN RUN = 36014350193 (push on 2892860, 6/6 success)
```

| Job | Conclusion |
|---|---|
| supply-chain policy present | success |
| cargo-deny | success |
| perf delivery-plan scale (windows) | success |
| rust (ubuntu-latest) | success |
| rust (macos-latest) | success |
| rust (windows-latest) | success |

Each `rust (*)` job runs fmt check, dependency direction, Clippy with
`-D warnings`, the full workspace tests, and portable package qualification.
Counts from the job logs (`test result:` lines summed):

| Job | `test result` lines | passed | failed | ignored | `FAILED`/`panicked` lines |
|---|---|---|---|---|---|
| rust (ubuntu-latest) | 125 | 854 | 0 | 1 | 0 |
| rust (windows-latest) | 125 | 851 | 0 | 1 | 0 |

All 17 named storage, Core, CLI and key tests in `QUALIFICATION.md` appear
with `ok` in both logs, including `restore_rejects_hand_edited_084_snapshots`
(14 cases), `a_hub_served_over_local_ipc_syncs_a_client` and
`hub_commands_run_through_core_over_local_ipc` (real local socket on Linux
and Windows). The macOS job passed; its per-test counts were not extracted.

Commits after the code head change documentation only; the final head
needs its own required CI run before merge, recorded in
`POST_MERGE_VERIFICATION.md` and `CLOSURE.md`.

Earlier heads on this branch (historical only):

| Head | Run | Result |
|---|---|---|
| 8f28157 | 36006576798 | cancelled (superseded) after its ubuntu job failed to compile (`OutboxEntry` name clash) |
| 21a5c16 | 36006930338 | cancelled (superseded) after its ubuntu job failed Clippy (`large_enum_variant`) |
| ae116fc | 36007228901 | failure (Clippy `unused_mut`) |
| d731747 | 36011098373 | failure (test compile: shadowed helper) |
| 4b1aef7 | 36014421782 | ubuntu/macOS/deny/supply/perf success (854 passed, 0 failed on ubuntu); Windows cancelled (superseded) |
| 6b8e6d8 | 36017596418 | ubuntu/macOS/deny/supply/perf success; Windows cancelled (superseded) |
