# CLOSURE — Spec 087 Community Extensions (declarative foundation)

## Terminal truth

```text
SPEC_087_CLOSED_CANONICAL=true
MERGE_SHA=2045998be9abe1bb74cbcbfb059a3b1dfe9146e6 (PR #153)
FINAL_HEAD=44050620ab438830c60e7ce945c26baeb7e031ab
EXACT_HEAD_CI=36258146789 (6/6; ubuntu 912/0/1, windows 908/0/1, macOS 910/0/1)
POST_MERGE_MAIN_CI=36263227436 (6/6 on 2045998)
BASE=742a93a (PR #154 merge, Spec 086 closure; post-main run 36258108623 6/6)
REVIEW_POLICY=FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22 (no external reviewer)
EXTENSION_KIND=declarative only (signed manifest; Host API v1 read-only operations run by Core)
EXTENSION_CODE_EXECUTION=NOT_ADMITTED (no WASM runtime admission; platform_qualified=false)
TRUST=explicit local publisher keys; no built-in trust root; strict ed25519
DEPENDENCIES_ADDED=none (ed25519 via the admitted medscale-keys)
STORAGE_SCHEMA=v16 (v15->v16 additive; 8 backup tamper cases)
REAL_PHI_AUTHORIZED=false
RELEASE_READY=false
PRIVATE_DATA_READY=false
```

## Closure-gate reading

"A third-party sample extension can be built, packaged, installed, denied
a non-granted capability, upgraded with capability re-consent, revoked,
and run without direct trusted-process/vault access." Met for declarative
extensions by `a_sample_extension_installs_is_denied_until_granted_and_runs_through_core`,
`untrusted_forged_or_incompatible_packs_are_refused` and
`revocation_quarantines_and_nothing_crosses_projects`. Executable
extensions remain a recorded gate.

## Honest residuals (non-blocking, recorded)

- No executable extensions (WASM or worker) until a runtime and sandbox
  are admitted.
- No Hub-hosted registry, review tiers or distribution; packs move as files.
- Licenses are declared, not verified; the identity behind a publisher key
  is the user's judgment.
- Project metadata has no classification row, so `project_summary` needs a
  `local_phi` ceiling.
- No Desktop surface.
