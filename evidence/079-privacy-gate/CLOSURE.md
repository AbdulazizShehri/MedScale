# CLOSURE — Spec 079 Privacy Gate

## Terminal truth

```text
SPEC_079_CLOSED_CANONICAL=true
MERGE_SHA=e2a90ba1ba9ddd1c56cad4b492bbda425193e79a (PR #137)
FINAL_HEAD=0168f057847a37f46613838fd95961d1fff2b7cc
EXACT_HEAD_CI=35806329807 (6/6)
CODE_HEAD_CI=35802759307 (6/6 on 82d3154)
POST_MERGE_MAIN_CI=35809030069 (6/6 on e2a90ba)
REVIEW_POLICY=FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22 (no external reviewer;
  deterministic scope record in EXACT_RANGE_REVIEW.md)
CONTRACTS=PASS (13)  STORAGE_MIGRATION_RECOVERY=PASS (12, v7->v8 additive)
CORE_AUTHORITY=PASS (10)  RECOGNIZERS=PASS (12 incl. corpus benchmark)
CLI=PASS (2)  DESKTOP_VIEW_MODEL=PASS (2)
SECURITY_ADVERSARIAL=PASS (T1-T14 mapped in SECURITY_ADVERSARIAL.md)
NO_NETWORK=PASS (structural)
REAL_PHI_AUTHORIZED=false
PHI_REMOVAL_CLAIMED=false (receipts always state automated recognition is incomplete)
RELEASE_READY=false
PRIVATE_DATA_READY=false
SPEC_080_IMPLEMENTATION_AUTHORIZED=false until its own promotion
```

## What this closure establishes

MedScale has one Core-owned Privacy Gate: artifacts are `local_phi` unless
classified; de-identification writes new artifacts with receipts that bind
source and output digests, profile revision, recognizer identities and a
residual scan that is never reported clean when a recognizer did not
complete; pseudonym keys live only in a key store, and re-identification is
a separate, always-audited capability; and every current and future
boundary gets one fail-closed, persisted egress decision.

## Honest residuals (non-blocking, recorded)

- No rendered Desktop screenshot (no CI rendering step), as in Specs 075-078.
- Profiles, transforms and re-identification are CLI-only; Desktop shows
  classifications, receipts, egress decisions, egress checks and receipt
  revocation.
- The recognizer corpus is a development set; real-world recall is
  unmeasured. Names are found only after honorifics or labels.
- Non-English narrative fixtures were not built (founder English-only
  directive); recorded for a founder decision.
- The OS keyring path (`select_keystore`) is not exercised in CI tests.
- Log-capture leakage is proven structurally (no print/log calls in 079
  Core/storage modules), not by an automated capture test.
- The local WSL build environment failed with disk I/O errors during this
  spec; all authoritative qualification is GitHub Actions CI.
