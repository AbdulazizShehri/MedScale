# MedScale � Start Here

## Productization entry — 2026-09-29

The later 2026-09-30 founder presentation decision is recorded in `TAURI_DESKTOP_MIGRATION_DECISION_2026-09-30.md`. Future 096–100 work migrates to Tauri 2 + React/TypeScript/Vite/Tailwind in bounded stages. 095 remains Slint-era brand evidence, unmerged review/dependency gates remain real, Slint is retained, and the Tauri prototype is synthetic-only.

Approved paired M supersedes older active visuals. Read PRODUCTIZATION_PROGRAM.md and specs/095-brand-foundation after standing authority/live BUILD_QUEUE. Closed 068–094 remain intact; 095–100 is autonomously authorized. Queue/closure evidence supersede older summaries. Architecture, PHI exclusion and release limitations remain.

## 1. Current gate

```text
PLAN = CANONICAL_V2
REPOSITORY_PLANNING_FINALIZATION = COMPLETE
SPEC_000 = CLOSED_CANONICAL
SPEC_001 = CLOSED_CANONICAL (see BUILD_QUEUE.md for live states)
FOUNDER_STANDING_CURSOR_IMPLEMENTATION_AUTHORITY = ACTIVE
REAL_PHI = NOT_AUTHORIZED
MESC_PROJECT = SEPARATE_REPOSITORY_OUT_OF_SCOPE
```

The earlier planning-only gate has been superseded by `IMPLEMENTATION_AUTHORITY.md`. Cursor must follow live BUILD_QUEUE.md; Specs 001�011 and 013�015 are CLOSED_CANONICAL; Specs 012 and 036 are retained as historical interoperability provenance only under `MESC_PROJECT_SEPARATION.md`; they are not current MedScale execution, completion, or release authority. It must not ask the founder for ordinary engineering decisions already governed by the plan.

## 2. Mandatory read order

1. `/CURSOR.md`
2. `/AGENTS.md`
3. `IMPLEMENTATION_AUTHORITY.md`
4. `MESC_PROJECT_SEPARATION.md`
5. `BUILD_QUEUE.md`
6. `MASTER_BUILD_PLAN_V2.md`
7. `SPECKIT_MASTER_ROADMAP_V2.md`
8. `IMPLEMENTATION_DECISION_DEFAULTS.md`
9. `SOURCE_ACQUISITION_AND_COPY_PLAN.md`
10. `V0_UI_INTEGRATION_CONTRACT.md`
11. relevant source/OSS/OpenMed matrices and current spec package

## 3. Build order

```text
000 CLOSED_CANONICAL
  -> 001 Rust Repository + Spec Kit Bootstrap
  -> 002 Trusted Object / Source / Authority + Process/Text Foundation
  -> 003 H0-A Trusted Ingest + Durability
  -> 004 H0-B Trusted Presentation + Coverage
  -> 005 Local Private Vault + Encryption + Recovery
  -> 006 CLI + Desktop Foundation
```

Then dependency-controlled tracks:

```text
004 -> 007 OpenMed parity/absorption research
005 + 006 + qualified 007 -> 008 Local AI Capability Fabric
005 + 006 -> 009 Mobile base; AI features also require 008
008 -> 010 Documents/OCR/Voice
004 + 008 -> 011 Evidence/Retrieval/Medical Intelligence
Historical only: Spec 012 records prior MESC artifact-interoperability work; it is not a current MedScale build dependency.
005 + 006 -> 013 FHIR/SMART/Network Broker -> 014 Controlled Actions/NPHIES
008 + 013 (+ mobile pack constraints) -> 015 Online Pack/HF Ecosystem
016+ remains deferred until canonically promoted
```

## 4. Current follow-on work

Read [whole-product review](WHOLE_PRODUCT_REVIEW_2026-09-09.md),
[delivery plan](TRUSTED_V1_DELIVERY_PLAN.md), and live [BUILD_QUEUE.md](BUILD_QUEUE.md).
Specs 016�023 are `CLOSED_CANONICAL` READY_BASE (022 = Q05 release-qualification **prep** only;
023 = Q03 open-metadata SQLCipher; `PRIVATE_DATA_READY` still FALSE).
Spec **024** is `CLOSED_CANONICAL` READY_BASE (Q04 host OS IPC; `MULTI_CLIENT_RELEASE_READY` still FALSE).
Spec **025** is `CLOSED_CANONICAL` READY_BASE (Q10 versioned synthetic evidence corpus; clinical quality / `RELEASE_READY` still FALSE).
Spec **026** is `CLOSED_CANONICAL` READY_BASE (Q09 pack signer/anti-rollback + Linux Landlock measured; `platform_qualified=false`; `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` still OPEN).
Spec **027** is `CLOSED_CANONICAL` READY_BASE (Q05 perf harness + SBOM/checksum scaffolds; budgets not claimed; `RELEASE_READY` still FALSE).
Spec **028** is `CLOSED_CANONICAL` READY_BASE (Q03 OS keyring custody; `os_keyring_available`/`os_keyring_used`; `PRIVATE_DATA_READY` still FALSE � swap/snapshot residual).
Spec **029** is `CLOSED_CANONICAL` READY_BASE (macOS CI matrix + CLI/fixture accessibility honesty; `macos_ci_present=true`; `macos_qualified=false`; no WCAG / `RELEASE_READY` claim).
Spec **030** is `CLOSED_CANONICAL` READY_BASE (Q09 Windows Job Object measured; `windows_measured=true`; AppContainer scaffold; `platform_qualified=false`; sandbox gate still OPEN).
Spec **031** is `CLOSED_CANONICAL` READY_BASE (Q09 macOS Seatbelt measured; `macos_measured=true`; App Sandbox entitlements scaffold; `platform_qualified=false`; sandbox gate still OPEN).
Spec **032** is `CLOSED_CANONICAL` READY_BASE (Q03 privacy probes + Q05 NOTICE inventory + perf binding; `rights_license_decision=false` was its closure-time state; current project license is Apache-2.0 per the 2026-09-14 external-gate update below; `PRIVATE_DATA_READY` / `RELEASE_READY` remain FALSE).
Spec **033** is `CLOSED_CANONICAL` READY_BASE (Q09 Windows AppContainer FS measured; `windows_appcontainer_fs_measured=true`; `platform_qualified=false`).
Spec **034** is `CLOSED_CANONICAL` READY_BASE (Q12 durable outbox restart; `outbox_restart_qualified=true`; NPHIES still gated).
Spec **035** is `CLOSED_CANONICAL` READY_BASE (EncryptedVault authority sync; `encrypted_authority_sync_qualified=true`; `PRIVATE_DATA_READY` still FALSE).
Spec **036** is retained as `CLOSED_CANONICAL` historical interoperability provenance; it is not a current MedScale lane, gate, completion axis, or release axis.
Spec **037** is `CLOSED_CANONICAL` READY_BASE (required-checks packet + checksum verify + license counsel packet + broker transport-fail fixtures; `RELEASE_READY` still FALSE).
Spec **038** is `CLOSED_CANONICAL` READY_BASE (Q09 AppContainer network measured; `windows_appcontainer_network_measured=true`; LPAC scaffold; sandbox gate still OPEN).
Spec **039** is `CLOSED_CANONICAL` READY_BASE (entry-doc honesty + PHI readiness checklist + signing/provenance prep packets; no RELEASE_READY claim).
Spec **040** is `CLOSED_CANONICAL` READY_BASE (Windows AppContainer LPAC ReadyBaseMeasured; `windows_appcontainer_lpac_measured`; `platform_qualified=false`).
Spec **041** is `CLOSED_CANONICAL` READY_BASE (macOS App Sandbox entitlements artifact/probe; enforcement_measured=false; see `SPEC_041_PROMOTION.md`).
Spec **042** is `CLOSED_CANONICAL` READY_BASE (delivery-plan scale perf harness; `budgets_claimed_met=false`).
Spec **046** is `CLOSED_CANONICAL` READY_BASE (SBOM Cargo.lock binding; `sbom_lock_bound`; not full release SBOM).
Spec **047** is `CLOSED_CANONICAL` READY_BASE (release dry-run + cross-verifier; `release_dry_run_verifier_present`; RELEASE_READY still FALSE; see `SPEC_047_PROMOTION.md`).
Spec **048** is `CLOSED_CANONICAL` READY_BASE (migration/recovery release-bar; `migration_recovery_ready_base`; RELEASE_READY still FALSE; see `SPEC_048_PROMOTION.md`).
Spec **049** is `CLOSED_CANONICAL` READY_BASE (package upgrade/rollback dry-run scaffold; `package_upgrade_rollback_scaffold_present`; RELEASE_READY still FALSE; see `SPEC_049_PROMOTION.md`).
Spec **050** is `CLOSED_CANONICAL` READY_BASE (host perf measurement path; `host_perf_measurement_path_present`; budgets still not claimed; see `SPEC_050_PROMOTION.md`).
Spec **051** is `CLOSED_CANONICAL` READY_BASE (REQUIRED_CHECKS live CI sync; `required_checks_packet_synced`; post-close external gate now `CONFIGURED` by active ruleset `23259329` targeting `main`; see `SPEC_051_PROMOTION.md` and ruleset qualification evidence).
Spec **052** is `CLOSED_CANONICAL` READY_BASE (Linux Landlock FS+TCP+rlimit composition; `linux_landlock_composition_measured`; seccomp still open; `platform_qualified=false`; see `SPEC_052_PROMOTION.md`).
Spec **053** is `CLOSED_CANONICAL` READY_BASE (Linux seccomp-bpf strict allowlist composition, child-measured SIGSYS deny; `linux_seccomp_composition_measured`; `platform_qualified=false`; see `SPEC_053_PROMOTION.md`).
Spec **054** is `CLOSED_CANONICAL` READY_BASE (deterministic CycloneDX 1.5 release SBOM + fail-closed verifier; `release_sbom_qualified`; `reproducible_build=unproven`; `RELEASE_READY` still FALSE; see `SPEC_054_PROMOTION.md`).
Spec **055** is `CLOSED_CANONICAL` READY_BASE (performance non-attainment dossier; `PERFORMANCE_NON_ATTAINMENT` bound to source/tree/lock; budgets not claimed; see `SPEC_055_PROMOTION.md`).
Spec **056** is `CLOSED_CANONICAL` READY_BASE (fixture role/keyboard/announcement semantics deepening; no WCAG claim; see `SPEC_056_PROMOTION.md`).
External gate update (2026-09-14): `PUBLIC_SOURCE_LICENSE_CHOICE=DECIDED`; founder selected `Apache-2.0` with package NOTICE aggregation required. Historical Spec 032 text records its closure-time state.
Specs **057**–**059** are `CLOSED_CANONICAL`, preserving Trusted V1 implementation closure. The separately promoted Desktop+CLI phase is also repository-complete: Specs **060**–**067** are `CLOSED_CANONICAL`. Product-differentiation and Research OS foundation lanes are also repository-complete: Specs **068**–**075** are `CLOSED_CANONICAL`, most recently Spec 075 (Data Source Fabric + Data Workbench Foundation) — exact-head run `35580861670` and post-merge main run `35582548200` passed all six required jobs; PR #129 merged as `89a88cfbbe7b67582886fb07fb6de4fd9ca28dff`; see `evidence/075-data-source-fabric/CLOSURE.md`. No repository-owned implementation unit remains promoted; candidates 076+ require a fresh promotion package. `RELEASE_READY=false`, `PRIVATE_DATA_READY=false`, and `MULTI_CLIENT_RELEASE_READY=false`; Spec 012 remains optional/deferred.
Do **not** claim `RELEASE_READY`, `PRIVATE_DATA_READY`, or `MULTI_CLIENT_RELEASE_READY`.
Q03 residual page encryption landed in Spec 023; OS keyring READY_BASE in Spec 028; Spec 032/043 probes classify residuals but do not clear swap/snapshot (still block PRIVATE_DATA_READY).
Foundation and prep closure is not product or privacy release readiness.

### Historical bootstrap requirements (already closed)

Spec 001 must:

- establish the Rust workspace and minimal dependency-directed crate skeleton;
- install/bootstrap GitHub Spec Kit and materialize the already-canonical constitution/source authority without reopening founder decisions;
- create CI, formatting/lint/test/evidence skeleton and supply-chain gates;
- establish repository contribution/review/evidence conventions;
- create the next complete Spec 002 package;
- add no medical functionality beyond bootstrap contracts required by 001.

When 001 closes, Cursor immediately starts 002.

## 5. First product wedge

The first useful product outcome is the **Trusted Local Longitudinal Record**: exact source custody, explicit identity/time, deterministic timeline and narrow LLM-free Brief, coverage/conflict/unknown-vs-absence accounting, provenance drill-down, and one Rust authority path shared by CLI/Desktop. It remains valuable with no model installed.

## 6. UI

The founder uses **v0** for visual UI. Cursor continues backend/core/integration work without waiting for final visual polish. v0 output enters through `imports/v0/` and the `V0_UI_INTEGRATION_CONTRACT.md`; generated server/database/network shortcuts never become MedScale authority automatically.

## 7. Evidence and completion

No `PASS`, `PARITY`, `SURPASS`, `PRIVATE`, `OFFLINE`, `CONFORMANT`, or `CLOSED_CANONICAL` claim is valid without exact evidence. Follow `DEFINITION_OF_DONE.md`. An external gate blocks only its own path; record it in `EXTERNAL_GATES.md` and continue.
