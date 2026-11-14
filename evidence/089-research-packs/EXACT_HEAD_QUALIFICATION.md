# Exact-Head Qualification — Spec 089

```text
BASE       = 1ac0362 (Spec 088 closure)
PR         = #156
FINAL_HEAD = 546ef2414e67bda1688b118968ebecff793165be
CI_RUN     = 36294669199 (pull_request), bound to FINAL_HEAD, 6/6 success
```

| Job | passed | failed | ignored |
|---|---|---|---|
| rust (ubuntu-latest) | 939 | 0 | 1 |
| rust (windows-latest) | 935 | 0 | 1 |
| rust (macos-latest) | 937 | 0 | 1 |

All 11 Spec 089 tests named in `QUALIFICATION.md` appear with `ok`
in the Linux and Windows logs; no `FAILED` line in any log. Draft runs
before promotion are historical.
