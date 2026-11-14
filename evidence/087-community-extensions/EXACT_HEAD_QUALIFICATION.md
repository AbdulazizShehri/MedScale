# Exact-Head Qualification — Spec 087

```text
BASE_SHA   = 742a93a76fdb581e1aeb6d253e630015d9c409e9 (PR #154 merge, Spec 086 closure)
BRANCH     = spec/087-community-extensions (PR #153)
FINAL_HEAD = 44050620ab438830c60e7ce945c26baeb7e031ab
CI_RUN     = 36258146789 (pull_request), bound to FINAL_HEAD, 6/6 success
```

| Job | passed | failed | ignored |
|---|---|---|---|
| rust (ubuntu-latest) | 912 | 0 | 1 |
| rust (windows-latest) | 908 | 0 | 1 |
| rust (macos-latest) | 910 | 0 | 1 |

All 13 Spec 087 tests (5 contract, 3 Core, 4 storage, 1 CLI) appear with
`ok` in the Linux and Windows logs; no `FAILED` line in any log. The
supply-chain, cargo-deny and Windows perf jobs passed.

Historical: head `7175645` (before promotion) passed run `36246872221`
6/6; the head then changed when `main` was merged forward and the
promotion recorded.
