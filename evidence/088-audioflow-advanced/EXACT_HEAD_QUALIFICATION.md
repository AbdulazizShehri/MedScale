# Exact-Head Qualification — Spec 088

```text
BASE       = a40bd1e (PR #161 merge, Spec 087 closure)
PR         = #155
FINAL_HEAD = 25216565b9df873ed6125cc01676684c62f13ae7
CI_RUN     = 36275414657 (pull_request), bound to FINAL_HEAD, 6/6 success
```

| Job | passed | failed | ignored |
|---|---|---|---|
| rust (ubuntu-latest) | 928 | 0 | 1 |
| rust (windows-latest) | 924 | 0 | 1 |
| rust (macos-latest) | 926 | 0 | 1 |

All 14 Spec 088 tests named in `QUALIFICATION.md` appear with `ok`
in the Linux and Windows logs; no `FAILED` line in any log. Draft runs
before promotion are historical.
