# RELEASE_READY checklist vs current state (synced through Spec 060 canonical closure)

Trusted V1 `RELEASE_READY` requires all applicable release bars below. Spec 060 begins the separately promoted Desktop+CLI product phase without weakening any release bar; **current product claim remains FALSE**.

| Requirement | Current | Notes |
|---|---|---|
| Immutable source/tree + lock binding procedure | TRUE | Spec 047/054/058 bind source/tree/Cargo.lock into release manifests, SBOM and package evidence |
| Toolchain + Cargo.lock identity | TRUE | Toolchain pin + committed lock + locked CI; package/SBOM bind lock digest |
| Qualified OS matrix | FALSE | Required CI and portable package qualification run on Windows/Linux/macOS, but macOS final product/App Sandbox signed enforcement remains external |
| Mandatory CI + repository review policy on protected main | TRUE | Active ruleset `23259329`: PR required, conversation resolution, six required checks, no bypass actors; approving-review count is 0 by configured policy, so no human-approval claim |
| Reproducible package contents | TRUE | Spec 058 proves bit-for-bit deterministic portable ZIP assembly from identical release binaries/head/platform/revision; compiler-output reproducibility is not claimed |
| SBOM incl. native/model assets | TRUE | Spec 054 deterministic CycloneDX 1.5 release SBOM + fail-closed verification; signing provenance still external |
| Rights / license decision | TRUE | Apache-2.0 selected 2026-09-14; NOTICE included in Spec 058 package |
| Checksums / provenance / signing verification | FALSE | Verification paths exist; production signing identity/provenance is `DESKTOP_RELEASE_SIGNING_PROVENANCE` external gate |
| Migration + recovery proof at release bar | TRUE | Spec 048 vault migration/recovery proof + Spec 058 real package install/upgrade/rollback lifecycle |
| Qualified-hardware performance attainment | FALSE | Harness coverage exists; hosted CI is not qualified hardware; external gate `QUALIFIED_RELEASE_PERFORMANCE_HARDWARE` |
| Final v0 UI / WCAG qualification | FALSE | Spec 060 native shell/design system is canonically closed; patient/workflow/utility/CLI/hardening/final qualification remain in Specs 061–067, and final keyboard/screen-reader/contrast/WCAG evidence remains open under `FINAL_V0_UI_ACCESSIBILITY_QUALIFICATION` |
| Source-linked claims + limitations | TRUE | Release evidence/limitations are source/tree/lock bound and preserve non-claims |
| No unresolved repository-owned material findings | TRUE | PR #101 exact-head run `34894123749` qualified the bounded Windows RSS parser fix; merge commit `449e4ba00b21eeabb526b699e90d78954bcd01f8` then passed post-merge main run `34895017496` with all six required jobs |

**Verdict: `RELEASE_READY = FALSE`**

Trusted V1 implementation remains complete, but whole-project status is `PRODUCT_LAUNCH_IMPLEMENTATION_IN_PROGRESS` while Specs 061–067 remain. This is not a `RELEASE_READY` claim. Also preserved: `PRIVATE_DATA_READY = FALSE`, `MULTI_CLIENT_RELEASE_READY = FALSE`.
