# Whole-Platform Qualification Matrix — Spec 092

Status vocabulary (never upgraded without evidence):

- `PROVEN` — repository-owned behavior exercised by tests that ran green in
  exact-head CI on Linux, Windows and macOS, with synthetic data only.
- `IMPLEMENTED` — code exists and is tested for some paths, but the claim in
  the row is broader than what the tests prove.
- `PARTIAL` — some of the row is proven; the rest is recorded as a gap.
- `MISSING` — repository-owned work not done.
- `DEFERRED` — deliberately out of the current release profile by canonical
  decision.
- `EXTERNAL` — needs a human/external action listed in
  `docs/planning/EXTERNAL_GATES.md`; never a repository PASS.
- `N/A` — not applicable to this profile.

Release profile qualified here: **local Personal/Lab, synthetic data, CLI
through Core, Linux/Windows/macOS hosted CI runners.** Nothing here
qualifies real PHI, production release, signed packages or rendered
Desktop accessibility.

| # | Area | Status | Evidence / gap |
|---|---|---|---|
| 1 | Core authority: one authority path for all surfaces | PROVEN | Every spec's Core tests go through `CliSession -> CoreFacade::dispatch`; `whole_platform_092` crosses ten planes in one vault |
| 2 | Project isolation | PROVEN | Cross-project refusals in 085-091 tests (snapshots, extensions, huddles, packs, adapters, federation) |
| 3 | Storage integrity (column/body checks, fail-closed reads) | PROVEN | Per-spec tampered-row tests; `whole_platform_092` runs every verifier after restore |
| 4 | Migration v1 -> v20 (additive, journaled) | PROVEN | Per-version rewind tests (`migration_v*_is_additive`), older-backup restores |
| 5 | Backup / restore with tamper refusal | PROVEN (after fix #160) | Per-spec tamper suites; `whole_platform_092` round trip across all planes, then new work in the restored vault. Backups written before #160 carry no id sequences (residual: new ids of a used kind collide and fail closed) |
| 6 | Restart / recovery of interrupted work | PROVEN | Compute `running -> interrupted`; adapter `sent -> unknown`; restart through Core in `whole_platform_092` |
| 7 | Upgrade / rollback of domain objects | PROVEN | Extensions (re-consent, rollback), Research Packs (non-destructive migration), adapters (config rollback) |
| 8 | Product upgrade / rollback between MedScale releases | PARTIAL | Storage forward-migration proven; downgrade of an upgraded vault is not supported (forward-only schema) |
| 9 | Corruption handling | PROVEN | Digest-checked blobs and outputs; corrupt inputs end `corrupt`/refused in 085-091 |
| 10 | Cancellation | PARTIAL | Compute in-process cancel proven; CLI runs are synchronous; no cancel for other long operations |
| 11 | Resource ceilings | PARTIAL | Byte/row/time bounds proven per plane; no OS memory/CPU quotas (085 residual) |
| 12 | Privacy classification and egress policy | PROVEN | 079 gate; `local_phi` never leaves via adapters (090) or federation (091); unclassified data defaults to `local_phi` |
| 13 | Encryption at rest | IMPLEMENTED | Encrypted vault (SQLCipher) exists and is tested; encryption is not PHI authorization; OS key custody gate open |
| 14 | Private-data readiness | EXTERNAL | `OS_KEYRING_SWAP_SNAPSHOT_PRIVATE_DATA`, `REAL_PHI_AUTHORIZATION` |
| 15 | Network default-deny / no hidden cloud fallback | PROVEN | Institutional transport defaults to unavailable (090); R workspace has no network path (086); federation is out of band (091); cloud ASR route refused (081) |
| 16 | Model boundaries | IMPLEMENTED | Model Fleet / MedAgent specs 077-078 tested; gated model terms not accepted (`GATED_MODEL_TERMS`) |
| 17 | Hub | PARTIAL | In-process and local IPC proven (084); no network transport or multi-machine run |
| 18 | Compute isolation | PARTIAL | Closed job kinds in a separate worker with measured OS mechanisms; `platform_qualified=false` (`WORKER_OS_SANDBOX_PLATFORM_QUALIFIED`) |
| 19 | R Workspace | PARTIAL | Staging, launch, explicit publish proven; managed R execution `NOT_ADMITTED`; external IDE not confined |
| 20 | Extensions | PARTIAL | Declarative extensions proven end to end; executable extensions `NOT_ADMITTED` (no WASM runtime admission) |
| 21 | Audio / huddles | PARTIAL | Consent, labeling, retention, deletion proven with the fixture engine; real ASR, capture, TTS absent; medical ASR quality `UNMEASURED` |
| 22 | Evidence / research semantics | PROVEN | Research Pack evidence axes with explicit `unknown` (089); knowledge retrieval (083) |
| 23 | Clinical correctness of any assessment | N/A | Not claimed; synthetic fixtures only |
| 24 | Institutional adapters | PARTIAL | Full write lifecycle proven against an in-process store; no real institutional transport (`PRODUCTION_CREDENTIALS`, network admission) |
| 25 | Federation | PARTIAL | Two-institution synthetic exchange proven; no federated analysis; identity secret not recoverable from backup (by design) |
| 26 | Performance budgets on qualified hardware | EXTERNAL | `QUALIFIED_RELEASE_PERFORMANCE_HARDWARE`; hosted CI perf job is a feasibility signal only |
| 27 | Accessibility (rendered UI, assistive technology) | EXTERNAL | `FINAL_V0_UI_ACCESSIBILITY_QUALIFICATION` |
| 28 | CLI / Desktop parity for Research OS planes | PARTIAL (after Specs 093-094) | Was `MISSING` at the Spec 092 closure. Spec 093 adds a Desktop Research OS route over all eight planes through Core with explicit empty/unavailable states; Spec 094 adds reversible row actions (Compute cancel, extension enable/disable, adapter suspend/resume). Terminal and data-moving actions remain CLI-only |
| 29 | Supply chain (cargo-deny, SBOM, dependency policy) | PROVEN | `cargo-deny` and supply-chain jobs green on every exact head; no dependency added in 085-091 |
| 30 | Package integrity (portable packages, checksums) | PROVEN | Portable package qualification step green in CI (058) |
| 31 | Signed packages / notarization | EXTERNAL | `DESKTOP_RELEASE_SIGNING_PROVENANCE`, `MACOS_SIGNED_PRODUCT_QUALIFICATION` |
| 32 | Local-first useful core | PROVEN | All campaign steps run with no account, no remote model, no network |
| 33 | Platform-specific behavior | PARTIAL | Linux/Windows/macOS hosted CI green; some link-safety tests Unix-only; Windows link check length-based (086) |
| 34 | Terminology content | EXTERNAL | SNOMED/LOINC/ICD/ATC/UMLS license gates |
| 35 | Partner EHR / NPHIES / SMART live | EXTERNAL | `PARTNER_EHR_NPHIES_ENDPOINT`, `SMART_LIVE_PARTNER_AUTHORIZATION` |

## Defects found by the campaign

- **Id sequences lost on restore** (all planes allocating from
  `store_state`): after a restore, the first new object of an already-used
  kind collided with a restored row and was refused. Found by
  `whole_platform_092` (federation receipt after restore); fixed forward in
  #160 (`fix(storage): carry id sequences through backup and restore`) with
  the regression test `id_sequences_restore`.
- **Nondeterministic leak check in `privacy_gate_079`**: the corpus
  leak assertion matched short values (the postal code `10115`) inside
  unrelated hex digests, ids or timestamps of persisted rows, so the test
  failed by chance (macOS, Spec 092 exact-head run `36359133877`). No value
  leaked. Fixed forward in this spec: a match now counts only when not
  embedded in a longer alphanumeric run, with the unit test
  `value_matching_ignores_digest_collisions_but_finds_leaks`. The failed
  run is recorded, not re-run into a pass.

Terminal values (this spec cannot set any of them to true):

```text
RELEASE_READY=false
PRIVATE_DATA_READY=false
PLATFORM_QUALIFIED=false
CLINICAL_VALIDATION_STATUS=NOT_PERFORMED
REGULATORY_STATUS=NOT_PERFORMED
```
