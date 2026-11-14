# Exact-Head Qualification — Spec 079

```text
BASE_SHA   = cf8731e497273633efe2968ade4cbdd6523e5595
BRANCH     = spec/079-privacy-gate (PR #137)
CODE_HEAD  = 82d31547df44047c45736127a67396992630b115
CI_RUN     = 35802759307 (pull_request), 6/6 success, bound to CODE_HEAD
TOOLCHAIN  = rustc/cargo 1.97.1
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
The ubuntu log shows 0 `FAILED` lines.

The commit that adds this evidence creates a new head; that final head needs
its own required CI run, recorded in `CLOSURE.md` / `POST_MERGE_VERIFICATION.md`.

Earlier heads on this branch:

| Head | Run | Result |
|---|---|---|
| 3403883 (promotion) | 35799706442 | 6/6 success |

Local runs (WSL Ubuntu, before that distro failed with disk I/O errors):
contracts 13/13, storage 12/12 plus all earlier storage suites, recognizers
12/12, Core 10/10, CLI 3/3 (2 new). They are supporting evidence only; CI is
authoritative.
