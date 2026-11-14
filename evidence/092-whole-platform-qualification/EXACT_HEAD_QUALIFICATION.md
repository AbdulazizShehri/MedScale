# Exact-Head Qualification — Spec 092

```text
BASE       = bef016f (Spec 091 closure)
PR         = #159
FINAL_HEAD = 7c4f85059644689b6ca858a664476dce0883258e
CI_RUN     = 36360503149 (pull_request), bound to FINAL_HEAD, 6/6 success
```

| Job | passed | failed | ignored |
|---|---|---|---|
| rust (ubuntu-latest) | 967 | 0 | 1 |
| rust (windows-latest) | 963 | 0 | 1 |
| rust (macos-latest) | 965 | 0 | 1 |

All 2 Spec 092 tests named in `QUALIFICATION.md` appear with `ok`
in the Linux and Windows logs; no `FAILED` line in any log. Draft runs
before promotion are historical.

## Earlier failed exact head (recorded)

Head `1010d17` run `36359133877` failed on macOS in
`privacy_gate_079::corpus_transforms_write_new_artifacts_and_bind_receipts_without_leaking_values`
("10115" matched inside an unrelated digest, id or timestamp). It was not
re-run. The check was fixed forward (`7c4f850`, token-bounded matching plus
`value_matching_ignores_digest_collisions_but_finds_leaks`), and the final
head above ran green on all three platforms.
