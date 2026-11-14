# Exact-Head Qualification — Spec 090

```text
BASE       = 8df3e45 (Spec 089 closure)
PR         = #157
FINAL_HEAD = b1dc801b95b5f552cc0d785206a9d21636e53ec3
CI_RUN     = 36310428876 (pull_request), bound to FINAL_HEAD, 6/6 success
```

| Job | passed | failed | ignored |
|---|---|---|---|
| rust (ubuntu-latest) | 954 | 0 | 1 |
| rust (windows-latest) | 950 | 0 | 1 |
| rust (macos-latest) | 952 | 0 | 1 |

All 15 Spec 090 tests named in `QUALIFICATION.md` appear with `ok`
in the Linux and Windows logs; no `FAILED` line in any log. Draft runs
before promotion are historical.
