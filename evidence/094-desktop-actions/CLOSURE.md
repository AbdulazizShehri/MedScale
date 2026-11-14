# CLOSURE — Spec 094 Research OS Desktop row actions (reversible slice)

## Terminal truth

```text
SPEC_094_CLOSED_CANONICAL=true
MERGE_SHA=c2a1ee9d6215bc24253913856ed4206485c6c52c (PR #167)
FINAL_HEAD=ec1c94356ef5072fbab9acf93a8c2804e1509c57
EXACT_HEAD_CI=36428223212 (6/6; ubuntu 972/0/1, windows 968/0/1, macOS 970/0/1)
POST_MERGE_MAIN_CI=36444728694 (6/6 on c2a1ee9)
BASE=4dfc8ad (Spec 093 closure; post-main run 36428188226 6/6)
REVIEW_POLICY=FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22 (no external reviewer)
DESKTOP_ROW_ACTIONS=Compute cancel; extension enable/disable; adapter suspend/resume
OTHER_ACTIONS=refused before Core; terminal and data-moving actions CLI only
PARITY=PARTIAL
PLATFORM_QUALIFIED=false
DEPENDENCIES_ADDED=none
REAL_PHI_AUTHORIZED=false
RELEASE_READY=false
PRIVATE_DATA_READY=false
```

## Closure-gate reading

"A refused action is never reported as done." Actions run through `CliSession`; the adapter round trip leaves Core receipts.

## Honest residuals (non-blocking, recorded)

- Terminal and data-moving Research OS actions remain CLI-only; parity is `PARTIAL`.
- Rendered UI and assistive-technology qualification stay under `FINAL_V0_UI_ACCESSIBILITY_QUALIFICATION`.
