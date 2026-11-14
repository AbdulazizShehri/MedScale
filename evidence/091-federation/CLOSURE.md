# CLOSURE — Spec 091 Federation (bounded bundle exchange)

## Terminal truth

```text
SPEC_091_CLOSED_CANONICAL=true
MERGE_SHA=c4438eff5ea4220bfab2c2663cc71ec3854a9adc (PR #158)
FINAL_HEAD=c9c523ea6589c75980bc3ae25ba0299f2e9f3fa3
EXACT_HEAD_CI=36331357728 (6/6; ubuntu 965/0/1, windows 961/0/1, macOS 963/0/1)
POST_MERGE_MAIN_CI=36338491427 (6/6 on c4438ef)
BASE=ee7e1d5 (Spec 090 closure; post-main run 36331339772 6/6)
REVIEW_POLICY=FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22 (no external reviewer)
IDENTITY=ed25519 per vault; secret never exported
TRUST=explicit peers with ceilings (never local_phi); terminal revocation
BUNDLES=signed, sequenced, provenance, tombstones; moved out of band; imports never overwrite
STORAGE_SCHEMA=v20 (v19->v20 additive)
DEPENDENCIES_ADDED=none
REAL_PHI_AUTHORIZED=false
RELEASE_READY=false
PRIVATE_DATA_READY=false
```

## Closure-gate reading

"A bounded multi-institution synthetic scenario proves signed exchange, provenance, revocation, policy denial, and no hidden central authority." Met by the two-institution scenario in `federation_091`.

## Honest residuals (non-blocking, recorded)

- No federated analysis; no network transport; no central service (by design).
- The identity secret is not recoverable from a backup; a restored vault needs a new identity.
- Key rotation is revoke and re-trust; one bundle format version.
- No Desktop surface.
