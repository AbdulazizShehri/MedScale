# Exact-Head Qualification — Spec 086

```text
BASE_SHA   = 61855e4b33ec5691bf53a673c7f08e9f62117c69 (PR #151 merge, Spec 085 closure)
BRANCH     = spec/086-r-workspace (PR #152)
FINAL_HEAD = c8e855176a8d7e0cdbaa9539351eb4dd74d58c96 (code and final head)
CI_RUN     = 36241268247 (pull_request), bound to FINAL_HEAD, 6/6 success
TOOLCHAIN  = rustc/cargo 1.97.x (CI)
```

| Job | Conclusion on FINAL_HEAD |
|---|---|
| supply-chain policy present | success |
| cargo-deny | success |
| perf delivery-plan scale (windows) | success |
| rust (ubuntu-latest) | success (every step) |
| rust (macos-latest) | success (every step) |
| rust (windows-latest) | success (every step) |

Each `rust (*)` job runs fmt check, dependency direction, Clippy with
`-D warnings`, the full workspace tests, and portable package
qualification. Counts from the job logs (`test result:` lines summed):

| Job | passed | failed | ignored |
|---|---|---|---|
| rust (ubuntu-latest) | 899 | 0 | 1 |
| rust (windows-latest) | 895 | 0 | 1 |
| rust (macos-latest) | 897 | 0 | 1 |

Every Spec 086 test named in `QUALIFICATION.md` appears with `ok` in the
Linux and Windows logs; `a_linked_outputs_directory_is_not_followed` and
the linked-script case are Unix-only (`#[cfg(unix)]`) and so run on Linux
and macOS only. No `FAILED` line appears in any log.

Earlier heads (historical, superseded): `a80a30a` run `36234643727`
failed Clippy (`field_reassign_with_default` in a test; fixed in
`dc2fd47`); `dc2fd47` run `36234775798` passed ubuntu and macOS and, on
Windows, every Spec 086 Core, CLI and contract test before it was
cancelled during the slow storage suite; the head then changed when `main`
was merged forward.
