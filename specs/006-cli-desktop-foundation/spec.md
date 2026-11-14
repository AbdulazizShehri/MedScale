# Feature Specification: CLI + Desktop Foundation

**Feature Branch**: `spec/006-cli-desktop-foundation`

**Created**: 2026-08-25

**Status**: Package complete; `QUALIFIED` for implementation (depends on Spec 005 `CLOSED_CANONICAL`)

**Input**: Deliver the first useful product surface—**Trusted Local Longitudinal Record**—via CLI (+ Desktop foundation) over the **same Rust Core Host / authority facade**. CLI authority contract with no privileged bypass and no direct DB open; `medscale doctor` operational report; typed `PRIVACY_PROOF` evidence artifact; Desktop thin scaffold with Tauri/WebView deferred unless privacy proof closes. Synthetic-only; REAL_PHI unauthorized; DEFAULT_DENY network; no MESC mutation. Integrate v0 UI when available under `imports/v0/` without blocking if absent.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - CLI Authority Path Only (Priority: P1)

An engineer runs MedScale CLI commands that open/create a vault, ingest synthetic data, and query timeline/Brief/coverage. Every mutating or reading operation goes through the Core Host authority facade. CLI never opens the canonical DB, never holds vault keys ambiently, and has no privileged bypass over Desktop/SDK clients.

**Why this priority**: Roadmap Spec 006 exit; MASTER_BUILD_PLAN §7/§10; GLM F-05 Core Host topology.

**Independent Test**: CLI integration suite invokes doctor/vault/ingest/presentation via facade only; negative tests prove no `rusqlite`/`EncryptedVault` open from the CLI crate itself.

**Acceptance Scenarios**:

1. **Given** Spec 005 EncryptedVault available, **When** `medscale vault create|open` runs, **Then** create/open succeeds only through Core Host lease + KeyProvider unlock envelopes.
2. **Given** an open vault, **When** `medscale ingest` loads a synthetic FHIR fixture, **Then** ingest receipts match Spec 003 semantics and no CLI process holds a durable DB connection handle.
3. **Given** CLI and a future Desktop client, **When** capability authorization is compared, **Then** CLI has no extra capabilities denied to Desktop for the same vault/scope.

---

### User Story 2 - `medscale doctor` Operational Report (Priority: P1)

`medscale doctor` reports vault location, sync/remote-root risk, key-store availability, privacy evidence freshness, filesystem/OS claim status, and pack/runtime placeholders (not_implemented until Spec 008)—without printing secrets, passphrases, or raw keys.

**Why this priority**: MASTER_BUILD_PLAN §10; replaces Spec 001 bootstrap doctor stub.

**Independent Test**: Doctor output parsed as typed `DoctorReport`; fields cover required axes; secret-marker scan of stdout fails closed (zero hits).

**Acceptance Scenarios**:

1. **Given** no vault configured, **When** `medscale doctor` runs, **Then** report states vault status, default location policy class, and claim/sync posture without inventing a vault.
2. **Given** an open or last-known vault path under claim policy, **When** doctor runs, **Then** it reports resolved vault location, sync/remote-root risk (ok|refused|unknown), key-store availability, privacy evidence freshness, and filesystem claim status.
3. **Given** known passphrase/key fixtures used elsewhere in the suite, **When** doctor stdout/stderr is captured, **Then** those secrets do not appear.

---

### User Story 3 - Longitudinal CLI Wedge (Priority: P1)

Using synthetic fixtures only, a user can create/open a vault, ingest permitted synthetic data, and print deterministic timeline, Brief, and coverage via CLI—demonstrating Trusted Local Longitudinal Record before any model or cloud account.

**Why this priority**: First product wedge (MASTER_BUILD_PLAN §7); proves Spec 003–005 value through a real surface.

**Independent Test**: End-to-end CLI script: vault create → ingest fixture → timeline/brief/coverage → digests/sections match Spec 004 golden expectations.

**Acceptance Scenarios**:

1. **Given** a fresh local app-data vault path, **When** vault create + unlock + ingest synthetic FHIR runs, **Then** ingest succeeds and sources are durable under EncryptedVault.
2. **Given** ingested assertions for a subject, **When** `medscale timeline|brief|coverage` runs, **Then** outputs are deterministic LLM-free presentations from Spec 004 rebuildable projections.
3. **Given** DEFAULT_DENY network posture, **When** the wedge script completes, **Then** no product runtime network client was invoked.

---

### User Story 4 - PRIVACY_PROOF Typed Evidence (Priority: P1)

Spec 006 produces a typed `PRIVACY_PROOF` evidence artifact with attributable observations, capability/network audit notes, crash/log/cache marker scan results (CLI/host paths), vault/sync-root evidence, and explicit limitations—including that system-wide “zero packets” is not claimed and that WebView/Tauri is not privacy-qualified in this unit.

**Why this priority**: MASTER_BUILD_PLAN §14; GLM F-17; roadmap exit.

**Independent Test**: Evidence archive contains `PRIVACY_PROOF` schema instance; limitations section lists deferred WebView/Tauri and synthetic-only; doctor references evidence freshness.

**Acceptance Scenarios**:

1. **Given** CLI wedge tests + doctor + log scans complete, **When** PRIVACY_PROOF is assembled, **Then** it includes vault/sync-root evidence, key-store availability note, log/crash marker scan for CLI/Core Host, and network DEFAULT_DENY observation.
2. **Given** Tauri not admitted, **When** PRIVACY_PROOF limitations are reviewed, **Then** they explicitly state WebView/cache containment unproven and Tauri deferred.
3. **Given** REAL_PHI gate, **When** PRIVACY_PROOF is published, **Then** it does not authorize real PHI or claim counsel legal sign-off.

---

### User Story 5 - Desktop Foundation Without Weakening Privacy (Priority: P1)

Desktop foundation ships as a **thin scaffold** over the same authority facade (in-process Core Host for Spec 006). Tauri v2.11.5 remains a SOURCE_ACQUISITION **candidate only**. Because WebView PHI containment cannot be proven in the same PR as the CLI wedge, Tauri is **not admitted**; if a WebView shell cannot meet the privacy claim later, MedScale changes the shell rather than weakening privacy. v0 visual integration is deferred when no artifact exists under `imports/v0/`—recorded as an external gate, not a READY blocker.

**Why this priority**: Roadmap Desktop shell + WebView reject-or-qualify; V0_UI_INTEGRATION_CONTRACT; SOURCE Tauri row.

**Independent Test**: Desktop scaffold crate/module compiles and calls facade smoke APIs; no Tauri dependency in workspace; EXTERNAL_GATES / PRIVACY_PROOF document deferral; absence of `imports/v0/` does not fail CI.

**Acceptance Scenarios**:

1. **Given** Spec 006 implement, **When** Cargo workspace dependencies are inspected, **Then** `tauri` is absent (not admitted) and Desktop scaffold has no WebView runtime.
2. **Given** no v0 export under `imports/v0/`, **When** Spec 006 closes, **Then** CLI + host wiring still ship; FINAL_V0_UI_ARTIFACT / deferred-UI note remains; no invented final visual design.
3. **Given** a future Tauri privacy qualification attempt, **When** WebView cache/crash markers cannot be contained, **Then** policy requires safer shell—not claim weakening (documented in research/PRIVACY_PROOF).

### Edge Cases

- Doctor with corrupted privacy evidence path: reports `stale|missing` freshness; does not invent PASS.
- CLI command with missing lease / locked vault: typed error; fail closed; no plaintext DB fallback.
- Ingest of non-synthetic path labeled real PHI: refused / out of scope; REAL_PHI remains NOT_AUTHORIZED.
- Concurrent second CLI writer on same vault: lease refuse (Spec 005/002).
- Desktop scaffold invoked without vault: smoke status only; no ambient open.
- PHI-looking synthetic strings in CLI stdout: allowed only for synthetic fixtures; default deny for future PHI-to-stdout remains policy (high-friction opt-in later claim scope).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: CLI MUST invoke Core Host / authority facade for vault, ingest, presentation, and doctor data collection; MUST NOT open canonical DurableStore/BlobStore/SQLCipher directly.
- **FR-002**: CLI MUST NOT have privileged bypass relative to Desktop/SDK clients for the same capability set on a vault/scope.
- **FR-003**: System MUST provide product-grade `medscale doctor` reporting vault location, sync/remote-root risk, key-store availability, privacy evidence freshness, filesystem/OS claim status, and pack/runtime placeholder state.
- **FR-004**: CLI MUST support vault create/open (EncryptedVault via Spec 005), synthetic ingest, and timeline/Brief/coverage commands sufficient for the Trusted Local Longitudinal Record wedge.
- **FR-005**: Spec 006 MUST produce a typed `PRIVACY_PROOF` evidence artifact with claim scope and explicit limitations (no system-wide zero-packet claim).
- **FR-006**: Tauri/WebView MUST NOT be admitted as a DEPENDENCY unless WebView PHI containment/privacy proof passes; Spec 006 default is **defer Tauri** and ship Desktop thin scaffold + PRIVACY_PROOF limitations.
- **FR-007**: If/when WebView cannot meet privacy claims, MedScale MUST change the desktop shell rather than weaken privacy invariants.
- **FR-008**: v0 UI integration MUST follow `V0_UI_INTEGRATION_CONTRACT.md` via `imports/v0/` when an artifact exists; absence MUST NOT block Spec 006 closeout (record EXTERNAL_GATES / deferred note).
- **FR-009**: Spec 006 MUST remain synthetic-only; REAL_PHI remains EXTERNAL_GATES NOT_AUTHORIZED.
- **FR-010**: Spec 006 MUST NOT introduce product runtime network egress (DEFAULT_DENY), MESC mutation/runtime, OpenMed runtime, models/packs as product path, or OCR/ASR.
- **FR-011**: Doctor and CLI error/help output MUST NOT emit passphrases, recovery codes, DEKs, or SQLCipher secrets.
- **FR-012**: CLI argument parsing SHOULD use pinned `clap`; fallible command orchestration MAY use pinned `anyhow` at the CLI edge only—authority errors remain typed `AuthorityError` envelopes from core/contracts.
- **FR-013**: Spec 006 MAY use in-process Core Host (CLI owns transient host) as the product IPC path for this unit; multi-process local IPC socket remains a later hardening option without reopening authority contracts.
- **FR-014**: Desktop foundation MUST share the same authority facade types/capabilities as CLI (scaffold smoke sufficient when Tauri deferred).

### Key Entities

- **DoctorReport**: Typed operational report (vault, sync risk, key-store, privacy freshness, FS claim, packs placeholder).
- **PrivacyProof**: Typed evidence artifact with observations, scans, claim scope, limitations.
- **CliSession**: Transient host ownership + vault lease context for a CLI invocation.
- **DesktopScaffold**: Non-WebView thin host/UI placeholder calling the same facade.
- **AuthorityFacadeClient**: Shared client path used by CLI and Desktop (in-process for 006).
- **LongitudinalWedgeReceipt**: End-to-end synthetic vault→ingest→presentation evidence record.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: CLI authority suite: 100% of vault/ingest/presentation commands exercise facade envelopes; 0 direct DB opens from `medscale-cli`.
- **SC-002**: Doctor suite: required report axes present and parseable; secret-marker scan of doctor output = 0 hits.
- **SC-003**: Longitudinal wedge suite: vault create → synthetic ingest → timeline/brief/coverage completes offline with deterministic Spec 004-compatible outputs.
- **SC-004**: PRIVACY_PROOF artifact archived with limitations listing synthetic-only, DEFAULT_DENY, no Tauri/WebView admission, no REAL_PHI.
- **SC-005**: Workspace has no admitted `tauri` dependency; Desktop scaffold builds without WebView.
- **SC-006**: Missing `imports/v0/` does not fail Spec 006 tests; deferred UI gate documented.
- **SC-007**: Deliverable introduces no real PHI, product network clients, MESC mutation, models/packs product path, or WebView privacy PASS claim.

## Assumptions

- Spec 005 is `CLOSED_CANONICAL` (EncryptedVault, KeyProvider, claim paths, recovery).
- Spec 004 presentation capabilities (`GetTimeline` / `GetBrief` / `GetCoverage`) remain the presentation contract.
- Spec 002/003 authority envelopes, lease, and ingest semantics remain binding.
- Spec 001 CLI bootstrap (`doctor` stub, `--version`) is replaced/extended—not a parallel CLI.
- Ordinary pins follow `IMPLEMENTATION_DECISION_DEFAULTS.md` and are recorded in `research.md`.
- Product runtime network remains DEFAULT_DENY by absence.
- FINAL_V0_UI_ARTIFACT remains user-supplied when ready; Cursor does not invent final visual design.
- Mobile shells are Spec 009; Spec 006 Desktop is foundation only.
