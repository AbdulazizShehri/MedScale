# Exact-Head Qualification — Spec 094

```text
BASE       = 4dfc8ad (Spec 093 closure)
PR         = #167
FINAL_HEAD = ec1c94356ef5072fbab9acf93a8c2804e1509c57
CI_RUN     = 36428223212 (pull_request), bound to FINAL_HEAD, 6/6 success
```

| Job | passed | failed | ignored |
|---|---|---|---|
| rust (ubuntu-latest) | 972 | 0 | 1 |
| rust (windows-latest) | 968 | 0 | 1 |
| rust (macos-latest) | 970 | 0 | 1 |

All 3 Spec 094 tests named in `QUALIFICATION.md` appear with `ok`
in the Linux and Windows logs; no `FAILED` line in any log. Draft runs
before promotion are historical.
