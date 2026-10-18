# MedScale Spec Kit Master Roadmap V2

**Date:** 2026-08-25  
**Status:** `CANONICAL_PLANNING_V2`  
**Development process:** GitHub Spec Kit  
**Runtime/trusted-core language:** Rust  
**Implementation authorization:** NO

## 1. Spec-of-Specs rule

MedScale stays one platform, but each slice must be independently specifiable and testable. The roadmap is shallow; detailed behavior lives in each Spec Kit package. Spec preparation/research may be parallelized when it does not mutate the repository or cross an implementation gate.

Required lifecycle for each material executable spec:

```text
/speckit.constitution (repository-level / when constitution changes)
/speckit.specify
/speckit.clarify
/speckit.plan
/speckit.checklist
/speckit.tasks
/speckit.analyze
-> qualification
-> /speckit.implement  # only with explicit founder implementation authorization
-> /speckit.converge
```

## 2. Canonical V2 roadmap

| ID | Spec | Intent | Entry gate | V2 additions / exit gate |
|---|---|---|---|---|
| 000 | `medscale-constitution-source-authority` | founder decisions, frozen architecture provenance, source authority | external planning package | captures V2 source/placement/privacy rules; no product code |
| 001 | `rust-repository-speckit-bootstrap` | initialize Rust workspace, Spec Kit, governance, CI/evidence/supply-chain skeleton | explicit founder bootstrap/implementation authorization | cargo-deny/RustSec/OSV; cargo-vet/auditable/fuzz policy; dependency-direction lint; no medical functionality |
| 002 | `trusted-object-source-authority-foundation` | object classes, identity, realm/scope, proposal/promotion, effects/retry, time, text spans, process topology, IPC contracts | 001 | **single-writer Core Host decision**, tagged text coordinate systems, native/FFI hardening contract, worker supervision policy; property/serialization tests pass |
| 003 | `h0a-trusted-ingest-durability` | synthetic FHIR R4 trusted custody and canonical visibility | 002 | lexical/duplicate-key/decimal/version/validator/identity tests; blob read integrity + GC race/restore invariants; **synthetic-scope backup/restore interface proof**; no real PHI/model/network |
| 004 | `h0b-trusted-presentation-coverage` | deterministic timeline + narrow LLM-free Brief + coverage/drill-down | 003 | bounded typed resource extractors; **no general FHIRPath in H0-B**; UCUM/unit semantics where required; negative/absence/conflict/golden rebuild tests |
| 005 | `local-private-vault-encryption-recovery` | pre-PHI production encryption/key/recovery/backup/migration/log/privacy | 004 | vault defaults to local app-data, sync/remote filesystem detection, single-writer/key lifecycle, snapshot/retention/key-loss/restore proof; REAL_PHI remains separately unauthorized |
| 006 | `cli-desktop-foundation` | first useful product surface using same Rust core | 005 + product-wedge evidence | CLI authority contract; Desktop shell prototype; WebView PHI containment or reject WebView; `medscale doctor`; `PRIVACY_PROOF`; no direct UI/CLI DB bypass |
| 007 | `openmed-capability-absorption-parity` | pin OpenMed v2.2 baseline; define parity corpora; donor absorption; terminology/licensing; Saudi/Arabic benchmark program | 004 for research/spec work; capability admission depends on 005/006 as needed | parity matrix is complete/phase-scoped; v2.2 exact baseline; donor deltas all dispositioned; terminology rights track started; **no runtime required by this spec itself** |
| 008 | `local-ai-capability-fabric` | model/NER/PII/de-ID runtime fabric, signed Pack v0, offline import, worker confinement | 005 + 006 + qualified 007 benchmark/admission contracts | `008A` offline pack admission (`medscale packs install <local-path>`), `008S` per-OS worker confinement, workload-specific runtime qualification; no online download path |
| 009 | `mobile-ios-android` | native mobile surfaces over shared Rust semantics | 005 + 006 + mobile FFI qualification; AI pack features additionally require 008 | native Keychain/Keystore; no silent key sync; 16KB Android compatibility; Files/SAF sideload; pack chunk/arch requirements fed back to 008/015; privacy/platform tests |
| 010 | `documents-ocr-voice` | hostile documents, OCR, ASR/diarization, source spans | 005 + 008 | P1 worker isolation, MIME/quarantine, document breadth parity, Arabic/code-switch benchmarks, critical-number source alignment |
| 011 | `evidence-retrieval-medical-intelligence` | structured/lexical/semantic retrieval, terminology grounding, evidence sets, conflict/freshness, research privacy controls | 004 + 008 | RELEVANCE != AUTHORITY; terminology/SDC parity where scoped; reproducible retrieval/evidence benchmark |
| 012 | `mesc-artifact-integration` | admit immutable MESC artifacts; exceptional service only if justified | 008; artifact lane additionally requires explicit MESC release (optional; Spec 012 DEFERRED_BY_CANONICAL_DESIGN) | exact hashes/rights/SBOM/evaluation; no direct Python import/shared DB/key; service path remains separately sandbox-qualified |
| 013 | `fhir-smart-network-broker` | partner FHIR/SMART adapters and sole controlled online egress abstraction | 005 + 006 | Network Broker receipts/allowlist/capability rules; bypass tests; FHIR profile/integrity/conformance evidence; no uncontrolled provider client |
| 014 | `controlled-actions-nphies` | durable action intent/outbox/reconciliation; NPHIES only if product evidence selects it | 013 + workflow evidence | PENDING/SENT/CONFIRMED/FAILED/UNKNOWN; no blind retry; approval binds exact payload; terminology/profile gates for selected NPHIES workflows |
| 015 | `hf-online-pack-ecosystem` | online pack acquisition/publishing and Hugging Face distribution | 008 + 013 + mobile-format constraints from 009 planning | online path only through Network Broker; TUF/offline root + optional Sigstore attestation; chunked/per-arch/resumable packs; HF never runtime requirement |
| 016 | `durable-trusted-record` | persist full trusted-record authority object graph across process restart (Q02) | 002–006 closed; Trusted V1 review | two-process restart + failure/backup qualification; not PRIVATE_DATA_READY |
| 017 | `vault-privacy-qualification` | open-vault work/WAL wipe and privacy honesty (Q03) | 016 closed; 005 EncryptedVault | lifecycle wipe/detect; PRIVATE_DATA_READY only with measured evidence |
| 018 | `host-client-authority` | authenticated local host/client sessions and scoped capabilities (Q04) | 016–017 | sessions/leases/revocation; not remote multi-tenant |
| 019 | `record-semantics` | precision-aware time, append-only amendments, explicit identity reconciliation, missingness (Q06) | 016 + 004 contracts | READY_BASE synthetic; RELEASE_READY false |
| 020 | `fhir-interchange-qualification` | FHIR support matrix, validator evidence, loss-aware export/provenance (Q08) | 019 | CLOSED_CANONICAL READY_BASE; bounded interchange; no false conformance |
| 021 | `minimum-lovable-workflow` | restartable import-review-timeline-export-backup journey (Q07) | 016–020 | CLOSED_CANONICAL READY_BASE; disclosure append; RELEASE_READY=false |
| 022 | `release-qualification-prep` | Q05 locked builds, evidence binding, doctor honesty (prep) | 016–021; Q05 partial pins | CLOSED_CANONICAL READY_BASE prep; RELEASE_READY=false; branch protection EXTERNAL_GATES |
| 023 | `vault-open-metadata-privacy` | SQLCipher page-encrypted EncryptedVault open work (Q03 residual) | 017 closed; admission 005-sqlcipher | CLOSED_CANONICAL READY_BASE; PRIVATE_DATA_READY=false (OS key/swap/snapshot) |
| 024 | `host-os-ipc-authority` | Localhost OS IPC + strict mutating sessions (Q04 residual) | 018 closed; Spec 016 WriterLock | CLOSED_CANONICAL READY_BASE; os_ipc_qualified=true; MULTI_CLIENT_RELEASE_READY=false |
| 025 | `evidence-corpus-lifecycle` | Source-versioned local lexical evidence corpus (Q10) | 011 + 016/019 | CLOSED_CANONICAL READY_BASE; synthetic-owned; clinical quality / RELEASE_READY=false |
| 026 | `pack-signer-os-sandbox` | Pack signer/trust/anti-rollback + Linux Landlock READY_BASE (Q09) | 008 | CLOSED_CANONICAL READY_BASE; linux_measured; platform_qualified=false; sandbox gate OPEN |
| 027 | `perf-sbom-release-evidence` | Perf harness + SBOM scaffold + package checksums (Q05 remnants) | 022 closed | CLOSED_CANONICAL READY_BASE; budgets not claimed; RELEASE_READY=false |
| 028 | `os-keyring-custody` | OS keyring wrapped-DEK custody (Q03 residual) | 023 closed; admission 005-keyring | CLOSED_CANONICAL READY_BASE; os_keyring_*; PRIVATE_DATA_READY=false (swap/snapshot) |
| 029 | `macos-ci-accessibility` | macOS CI matrix + CLI/fixture accessibility honesty | 022/027 + 006/021 | CLOSED_CANONICAL READY_BASE; macos_ci_present; macos_qualified=false; no WCAG; RELEASE_READY=false |
| 030 | `windows-appcontainer-sandbox` | Windows Job Object READY_BASE (Q09 residual; AppContainer scaffold) | 026 closed | CLOSED_CANONICAL READY_BASE; windows_measured; platform_qualified=false; sandbox gate OPEN |
| 031 | `macos-seatbelt-sandbox` | macOS Seatbelt READY_BASE (Q09 residual; App Sandbox entitlements scaffold) | 026/030 closed | CLOSED_CANONICAL READY_BASE; macos_measured; platform_qualified=false; sandbox gate OPEN |
| 032 | `privacy-probes-notice-perf` | Q03 privacy probes + Q05 NOTICE inventory + perf binding | 017/023/027/028 closed | CLOSED_CANONICAL READY_BASE; rights license was undecided at closure and is now Apache-2.0; PRIVATE_DATA_READY=false; budgets not claimed |
| 033 | `windows-appcontainer-fs` | Windows AppContainer FS READY_BASE (Q09 residual) | 030 closed | CLOSED_CANONICAL READY_BASE; windows_appcontainer_fs_measured; platform_qualified=false; sandbox gate OPEN |
| 034 | `durable-outbox-restart` | Durable outbox restart fixtures (Q12 residual) | 014+016 closed | CLOSED_CANONICAL READY_BASE; outbox_restart_qualified; NPHIES gated |
| 035 | `encrypted-vault-authority-sync` | EncryptedVault authority graph sync (Q02/Q03 residual) | 016+023 closed | CLOSED_CANONICAL READY_BASE; encrypted_authority_sync_qualified; PRIVATE_DATA_READY=false |
| 036 | `mesc-synthetic-verifier` | MESC synthetic release-dir verifier (012 residual) | 012 deferred-optional | Spec 012 DEFERRED_BY_CANONICAL_DESIGN; verifier_ready_base; MESC artifact NOT_AVAILABLE with no core/release impact |
| 037 | `release-prep-transport-fail` | Required-checks + license counsel + checksum verify + transport-fail fixtures | 013/022/027/032 | CLOSED_CANONICAL READY_BASE; RELEASE_READY=false |
| 038 | `windows-appcontainer-network` | Windows AppContainer network READY_BASE (Q09 residual) | 033 closed | CLOSED_CANONICAL READY_BASE; windows_appcontainer_network_measured; LPAC scaffold; platform_qualified=false |
| 039 | `release-honesty-packets` | Entry-doc honesty + PHI readiness + signing/provenance prep packets | 022/037 | CLOSED_CANONICAL READY_BASE; no RELEASE_READY / PHI / signing credentials |
| 040 | `windows-appcontainer-lpac` | Windows AppContainer LPAC READY_BASE (Q09 residual) | 038 closed | CLOSED_CANONICAL READY_BASE; windows_appcontainer_lpac_measured; platform_qualified=false |
| 041 | `macos-app-sandbox-entitlements` | macOS App Sandbox entitlements READY_BASE (Q09 residual) | 031/040 closed | CLOSED_CANONICAL READY_BASE; entitlements measured; enforcement_measured=false; platform_qualified=false |
| 042 | `perf-delivery-plan-scale` | Delivery-plan scale perf harness (Q05 residual) | 027/032 closed | CLOSED_CANONICAL READY_BASE; budgets_claimed_met=false |
| 043 | `swap-snapshot-honesty` | Swap/snapshot/core-dump honesty classification (Q03 residual) | 032 closed | CLOSED_CANONICAL READY_BASE; PRIVATE_DATA_READY=false |
| 044 | `sandbox-composition-honesty` | Multi-OS sandbox composition honesty (Q09 residual) | 041 closed | CLOSED_CANONICAL READY_BASE; composition_inventory_present; platform_qualified=false |
| 045 | `lexical-10k-corpus` | Procedural synthetic lexical 10k corpus (Q10/Q05 residual) | 025/042 closed | CLOSED_CANONICAL READY_BASE; clinical_quality/budgets/RELEASE_READY false |
| 046 | `sbom-lock-binding` | SBOM Cargo.lock binding (Q05 residual) | 027 closed | CLOSED_CANONICAL READY_BASE; sbom_lock_bound; not full release SBOM |
| 047 | `release-dry-run-verifier` | Release dry-run + cross-verifier (Q05 residual) | 039/046 closed | CLOSED_CANONICAL READY_BASE; release_dry_run_verifier_present; RELEASE_READY=false |
| 048 | `migration-recovery-release-bar` | Migration/recovery release-bar READY_BASE (Q05 residual) | 003/005/047 closed | CLOSED_CANONICAL READY_BASE; migration_recovery_ready_base; RELEASE_READY=false |
| 049 | `package-upgrade-rollback` | Package upgrade/rollback dry-run scaffold (Q05 residual) | 047/048 closed | CLOSED_CANONICAL READY_BASE; package_upgrade_rollback_scaffold_present; RELEASE_READY=false |
| 050 | `host-perf-measurement` | Host-bound perf measurement path (Q05 residual) | 042/045 closed | CLOSED_CANONICAL READY_BASE; host_perf_measurement_path_present; budgets not claimed |
| 051 | `required-checks-sync` | REQUIRED_CHECKS live CI sync (Q05 residual) | 037/042 closed | CLOSED_CANONICAL READY_BASE; required_checks_packet_synced; post-close branch protection CONFIGURED by ruleset 23259329 |
| 052 | `linux-landlock-composition` | Linux Landlock FS+TCP+rlimit composition (Q09 residual) | 026/044 closed | CLOSED_CANONICAL READY_BASE; linux_landlock_composition_measured; platform_qualified=false |
| 053 | `linux-seccomp-composition` | Linux seccomp-bpf strict allowlist composition (Q09 residual) | 052 closed | CLOSED_CANONICAL READY_BASE; linux_seccomp_composition_measured; platform_qualified=false |
| 054 | `native-full-release-sbom` | deterministic full release SBOM qualification | 046/047 | CLOSED_CANONICAL READY_BASE; signing provenance external |
| 055 | `perf-attainment-dossier` | honest performance non-attainment dossier | 050/054 | CLOSED_CANONICAL; qualified hardware still external |
| 056 | `fixture-a11y-semantics` | fixture accessibility semantics deepening | 029 | CLOSED_CANONICAL; no WCAG/final-v0 claim |
| 057 | `release-qualification-residual-integrity` | runtime perf coverage + immutable CI action pins | 055/056 | CLOSED_CANONICAL; budgets not claimed |
| 058 | `portable-release-package` | deterministic unsigned package + real install/upgrade/rollback qualification | 057 | CLOSED_CANONICAL; three-OS package lifecycle qualified |
| 059 | `final-release-closure-audit` | terminal repo-owned release audit + external-only residual mapping | 058 | CLOSED_CANONICAL; corrective PR #101 + post-merge run 34895017496 six-check success; RELEASE_READY remains false |
| 060+ | `research-site-imaging-genomics-advanced` | deferred site/research/collab/imaging/genomics/plugin/browser/watch/vision expansion | evidence-driven later gates | no Trusted V1 implementation commitment |

## 3. Corrected critical path

```text
FOUNDATION / TRUST
000 -> 001 -> 002 -> 003 -> 004 -> 005 -> 006

EARLY PARALLEL PLANNING/RESEARCH (no product mutation before authorization)
after 004: 007 OpenMed v2.2 parity corpus + Saudi/Arabic corpus design + terminology rights track

CAPABILITY
005 + 006 + qualified 007 -> 008
008 -> 010 / 011
008 -> 012 (optional lane; artifact admit additionally requires explicit MESC release)

MOBILE
005 + 006 -> 009 base mobile surface
008 -> model-pack capability inside 009

NETWORK / ACTIONS
005 + 006 -> 013 -> 014

ONLINE ECOSYSTEM
008 + 013 (+ 009 pack-format constraints) -> 015

016 durable trusted record (Q02) after foundation
017 vault privacy (Q03) lifecycle wipe
018 host/client authority (Q04)
019 record semantics (Q06)
020 FHIR interchange qualification (Q08) CLOSED_CANONICAL READY_BASE
021 minimum lovable workflow (Q07) CLOSED_CANONICAL READY_BASE
022 release-qualification prep (Q05) CLOSED_CANONICAL READY_BASE prep; RELEASE_READY=false
023 vault open-metadata privacy (Q03 residual) CLOSED_CANONICAL READY_BASE; PRIVATE_DATA_READY=false
024 host OS IPC authority (Q04 residual) CLOSED_CANONICAL READY_BASE; os_ipc_qualified=true; MULTI_CLIENT_RELEASE_READY=false
025 evidence corpus lifecycle (Q10) CLOSED_CANONICAL READY_BASE; synthetic-lexical@version; clinical quality / RELEASE_READY=false
026 pack signer + OS sandbox (Q09) CLOSED_CANONICAL READY_BASE; linux_measured; platform_qualified=false; WORKER_OS_SANDBOX gate OPEN
027 perf harness + SBOM/checksum evidence (Q05 remnants) CLOSED_CANONICAL READY_BASE; budgets not claimed; RELEASE_READY=false
028 OS keyring custody (Q03 residual) CLOSED_CANONICAL READY_BASE; os_keyring_*; PRIVATE_DATA_READY=false (swap/snapshot)
029 macOS CI + accessibility honesty CLOSED_CANONICAL READY_BASE; macos_ci_present; macos_qualified=false; no WCAG; RELEASE_READY=false
030 Windows Job Object sandbox (Q09 residual) CLOSED_CANONICAL READY_BASE; windows_measured; AppContainer scaffold; platform_qualified=false; WORKER_OS_SANDBOX gate OPEN
031 macOS Seatbelt sandbox (Q09 residual) CLOSED_CANONICAL READY_BASE; macos_measured; App Sandbox entitlements scaffold; platform_qualified=false; WORKER_OS_SANDBOX gate OPEN
032 privacy probes + NOTICE inventory + perf binding CLOSED_CANONICAL READY_BASE; license was undecided at closure and is now Apache-2.0; budgets_claimed_met=false; PRIVATE_DATA_READY=false
033–053 residual READY_BASE closed (AppContainer FS/network/LPAC, outbox restart, EncryptedVault sync, MESC verifier, release-prep/honesty, sandbox composition, lexical 10k, SBOM/dry-run/migration/upgrade/perf path, REQUIRED_CHECKS sync, Linux Landlock composition, Linux seccomp composition)
Terminal repository-owned implementation: Trusted V1 Specs **054**–**059** and the separately promoted Desktop+CLI phase Specs **060**–**067** are closed; no promoted repository-owned implementation residual remains. Spec 012 remains optional/deferred. Remaining release blockers stay evidence-gated (qualified performance hardware, signing/provenance, macOS signed-product/App Sandbox enforcement, final assistive-technology/WCAG qualification).
```

## 2026-09-15 Desktop + CLI product-launch phase

Founder-approved final visual direction is now supplied. Trusted V1 remains canonically complete through Spec 059; this is a new product-surface phase, not a reopening of foundation closure.

```text
060 native Desktop design system + Home/Command Center
  -> 061 patient workspace + longitudinal UX
  -> 062 population insights + contextual assistant
  -> 063 workflow studio + tasks/messages
  -> 064 audit/exports/settings/integrations
  -> 065 CLI launch-quality UX + capability parity
  -> 066 Desktop+CLI hardening (shape/critique/audit/polish/harden/optimize)
  -> 067 final UI performance/accessibility product qualification
```

Mobile applications and Specs 068+ remain deferred by canonical design and require fresh promotion. Closure of Specs 060–067 does not authorize real PHI, production credentials, signing/notarization, WCAG conformance, or a release claim without evidence.

## 4. Core process topology — decided for downstream specs

Desktop/headless uses one Rust **MedScale Core Host** per open vault. It alone owns the writable canonical metadata connection and active vault key material. Desktop UI, CLI and SDK are local IPC clients. When no host exists, a CLI command may spawn/own a transient host under the same authorization path. A per-vault single-writer lease prevents two independent owners. Direct UI/CLI multi-process database opening is forbidden.

IPC uses OS-local transports and peer identity where available (named pipe on Windows, Unix-domain/local equivalent on Unix platforms), versioned messages, request capabilities, explicit deadlines/limits and no DB/key handle transfer. Mobile uses one app-owned Rust core instance; background/platform services route through the same authority facade.

## 5. Candidate initial crate topology

Avoid crate-per-noun fragmentation. Start with dependency/unsafe-boundary driven crates and promote modules only when compile-time ownership pressure justifies it:

```text
crates/
  medscale-contracts/
  medscale-core/          # authority + time + object semantics as modules initially
  medscale-storage/
  medscale-keys/
  medscale-fhir/
  medscale-projections/
  medscale-worker-protocol/
  medscale-worker-host/
  medscale-cli/

apps/
  desktop/
  ios/
  android/
workers/
  models/
  documents/
  voice/
```

`medscale-network`, `medscale-pack`, and `medscale-ffi` become separate crates when their owning specs make the boundary real. Crate promotion is by dependency/security/unsafe boundary, not by domain-name aesthetics.

## 6. Hard anti-scope rules

H0 contains no OpenMed/MESC/model/OCR/ASR/vector/agent/live network/NPHIES/action/real-PHI/browser/imaging/genomics functionality. No REST/gRPC/GraphQL service plane, plugin ecosystem, general FHIRPath engine, custom cryptography, or cloud-provider adapter is required by V2 core. No feature may bypass the Rust authority path because it is “just CLI,” “just a model,” or “just a worker.”


## 2026-09-09 evidence-backed refinement

Read [whole-product review](WHOLE_PRODUCT_REVIEW_2026-09-09.md) and
[trusted V1 delivery priorities](TRUSTED_V1_DELIVERY_PLAN.md). Historical scope closures
remain intact; next bounded planning work is tracked in BUILD_QUEUE.md. Runtime promotion
requires the owning Spec Kit lifecycle. No blanket advanced-capability promotion is implied.

## 2026-09-15 Product differentiation reopening

The founder explicitly rejected the prior product-level visual direction and model/competitive visibility. This is a fresh promotion under the existing rule that deferred Specs 068+ require founder promotion.

```text
068 product identity + IA + truthful Models/Evidence surfaces
  -> 069 real local model runtime + Hugging Face pack acquisition path
  -> 070 Model Center + runtime/operator UX
  -> 071 OpenMed evidence center + pinned comparative qualification
  -> 072 rebuilt-product performance/accessibility/release requalification
```

Specs 000–067 retain their historical canonical evidence. `MEDSCALE_IMPLEMENTATION_COMPLETE` returns to false until 068–072 close. This sequence does not authorize real PHI, bypass the Core Host authority model, or make MESC release-blocking.

## 2026-09-30 desktop presentation amendment

The founder selected staged Tauri 2 + React/TypeScript/Vite/Tailwind migration for future Desktop presentation. See `TAURI_DESKTOP_MIGRATION_DECISION_2026-09-30.md`, `PRODUCTIZATION_PROGRAM.md`, live `BUILD_QUEUE.md` and preparatory `specs/096-tauri-foundation/`. Specs 096–100 retain their product outcomes and dependency order with the new presentation medium. Spec 095 remains Slint-era brand evidence. Existing Slint, the Rust authority path, privacy gates, PHI exclusion and release limits remain in force; retirement requires a separate decision after parity.
