# Live Truth — Spec 085 (T085-00)

Observed 2026-09-26 with `git fetch`, `gh pr list`, `gh pr view`, `gh run view`.

```text
BASE (origin/main) = 434d1c71f9739d74e886b235a237cb741ddc51b5
                     (merge of the Spec 084 closure PR #150)
SPEC 084           = PR #147 merged as 6caa698 (exact-head run 36033616030
                     6/6; post-main run 36193194092 6/6); closure PR #150
                     exact-head run 36199070870 6/6 on bafea22, merged as
                     434d1c7, post-main run 36203706536
SPEC 083           = closure PR #148 merged as 426bb34; post-main run
                     36033406508 6/6
BRANCH             = spec/085-compute (PR #149, draft until qualified);
                     started from 6caa698 (the Spec 084 merge) while 084
                     closed, then forward-merged with main (434d1c7); no
                     rebase, no force-push
OPEN PRS           = #124, #125 (drafts, unrelated; not touched)
STORAGE SCHEMA     = v13 on base; this spec adds v14
SANDBOX            = every OS axis ReadyBaseMeasured; platform_qualified=false
                     (OsSandboxDoctorStatus::ready_base, EXTERNAL_GATES open)
DEPENDENCIES       = none added (serde, serde_json, sha2, landlock, libc,
                     windows-sys already admitted)
NETWORK            = none
REVIEW POLICY      = FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22 (no external
                     reviewer)
```
