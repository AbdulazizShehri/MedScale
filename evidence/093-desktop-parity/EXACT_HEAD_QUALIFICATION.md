# Exact-Head Qualification — Spec 093

```text
BASE       = 41d2a9a (Spec 092 closure)
PR         = #162
FINAL_HEAD = a5317e4669ac056935defc90a75e9c23ea6ca173
CI_RUN     = 36383307415 (pull_request), bound to FINAL_HEAD, 6/6 success
```

| Job | passed | failed | ignored |
|---|---|---|---|
| rust (ubuntu-latest) | 969 | 0 | 1 |
| rust (windows-latest) | 965 | 0 | 1 |
| rust (macos-latest) | 967 | 0 | 1 |

All 2 Spec 093 tests named in `QUALIFICATION.md` appear with `ok`
in the Linux and Windows logs; no `FAILED` line in any log. Draft runs
before promotion are historical.
