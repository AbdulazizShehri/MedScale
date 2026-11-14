# Exact-Head Qualification — Spec 091

```text
BASE       = ee7e1d5 (Spec 090 closure)
PR         = #158
FINAL_HEAD = c9c523ea6589c75980bc3ae25ba0299f2e9f3fa3
CI_RUN     = 36331357728 (pull_request), bound to FINAL_HEAD, 6/6 success
```

| Job | passed | failed | ignored |
|---|---|---|---|
| rust (ubuntu-latest) | 965 | 0 | 1 |
| rust (windows-latest) | 961 | 0 | 1 |
| rust (macos-latest) | 963 | 0 | 1 |

All 11 Spec 091 tests named in `QUALIFICATION.md` appear with `ok`
in the Linux and Windows logs; no `FAILED` line in any log. Draft runs
before promotion are historical.
