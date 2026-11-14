# Feature Specification: FHIR / SMART / Network Broker

**Feature Branch**: `spec/013-fhir-smart-network-broker`

**Created**: 2026-08-25

**Status**: Package complete; `QUALIFIED` for implementation (depends on Spec 005 + Spec 006 `CLOSED_CANONICAL`)

**Input**: Deliver the **sole product online egress abstraction**—the Network Broker—with explicit destination / purpose / data-class / authorization / receipt on every egress. Provide partner FHIR/SMART **adapter interfaces** (stub + fixture receipts in this unit). Prove bypass impossibility, FHIR profile/integrity/conformance evidence attachment, and fail-closed allowlist behavior. No uncontrolled provider/HTTP client outside the broker. Product runtime remains DEFAULT_DENY except through the broker. Synthetic-only / H0-adjacent fixture posture; REAL_PHI unauthorized; no MESC mutation.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Sole Egress Through Network Broker (Priority: P1)

An engineer (or later CLI/Desktop/worker/HF client) that needs online product egress must call the Core Host Network Broker capability. Every request declares destination, purpose, data-class, and authorization context. Successful or denied attempts produce durable receipts. No worker, provider SDK, or ad-hoc HTTP client may open sockets for product purposes outside the broker.

**Why this priority**: MASTER_BUILD_PLAN §12; SPECKIT_MASTER_ROADMAP Spec 013 exit; constitution DEFAULT_DENY until broker.

**Independent Test**: Architecture/dependency + capability suites prove product crates cannot invoke the admitted HTTP client except via broker; unknown destination denied; receipt written.

**Acceptance Scenarios**:

1. **Given** an empty or fixture allowlist, **When** a broker request targets an unknown destination, **Then** the call is denied fail-closed and a durable denial receipt is recorded.
2. **Given** an allowlisted synthetic destination + authorized purpose/data-class, **When** broker executes (fixture transport), **Then** a durable success/attempt receipt binds destination/purpose/data-class/authorization and response digest class.
3. **Given** a product client crate (CLI/worker stub), **When** dependency/architecture tests run, **Then** no direct `ureq`/`TcpStream` product egress path exists outside the broker module.

---

### User Story 2 - Fail-Closed Allowlist + Authorization (Priority: P1)

The broker maintains an explicit destination allowlist (host/port/scheme/path-prefix policy as contracted). Unknown destinations are denied. Purpose and data-class must match allowlist rows. Missing or insufficient authorization fails closed. DEFAULT_DENY remains the posture when the broker is not invoked.

**Why this priority**: Roadmap receipts/allowlist/capability rules; privacy minimum disclosure.

**Independent Test**: Matrix of allowlist miss, purpose mismatch, data-class mismatch, unauthorized capability → all Deny + receipt; no network bytes on Deny path when transport is live-stubbed.

**Acceptance Scenarios**:

1. **Given** allowlisted host but wrong purpose, **When** broker is invoked, **Then** Deny with typed error; no transport send.
2. **Given** allowlisted host + matching purpose but data-class not permitted, **When** broker is invoked, **Then** Deny; receipt records data-class refusal.
3. **Given** no Network Broker capability in the caller's authority scope, **When** egress is attempted, **Then** `AuthorityError::Unauthorized` (or equivalent); no ambient bypass.

---

### User Story 3 - Durable Broker Receipts (Priority: P1)

Every broker decision (Allow→attempt, Deny) produces durable audit/evidence using Spec 002 **ActionAuditRecord** and/or **EvaluationRecord** patterns: evidence-only evaluations for conformance/profile checks; audit/action records for egress attempts with payload digests and effect markers appropriate to this unit (full PENDING→SENT→CONFIRMED machine remains Spec 014).

**Why this priority**: Roadmap receipts; continuity with Spec 002 object classes; feeds Spec 014/015.

**Independent Test**: After fixture broker calls, DurableStore (or EncryptedVault path via Core Host) contains ActionAuditRecord and EvaluationRecord instances with required fields; receipts rebuildable by id.

**Acceptance Scenarios**:

1. **Given** a denied request, **When** receipts are queried, **Then** an ActionAuditRecord (kind Audit) records actor, action, destination/purpose/data-class summary, and denial reason digest—no PHI required.
2. **Given** an allowed fixture transport attempt, **When** receipts are queried, **Then** ActionAuditRecord binds request/response digests and outcome; EvaluationRecord may attach profile/conformance evidence with `evidence_only = true`.
3. **Given** receipt persistence, **When** vault/store restarts, **Then** receipts remain durable and queryable via authority facade.

---

### User Story 4 - Partner FHIR / SMART Adapter Interface (Priority: P1)

MedScale exposes typed partner FHIR and SMART **adapter interfaces** behind the broker. This unit ships **stubs + fixture receipts** (synthetic FHIR R4 4.0.1 interchange bytes, synthetic SMART discovery/token fixture envelopes). Live partner EHR/SMART endpoints remain EXTERNAL_GATES; production credentials remain NOT_GRANTED.

**Why this priority**: Roadmap partner FHIR/SMART adapters; Inferno/SMART as EXTERNAL_CONFORMANCE evidence only.

**Independent Test**: Adapter suite drives stub SMART authorize/token/refresh and FHIR read/search against fixtures through broker; live URL paths refused without gate.

**Acceptance Scenarios**:

1. **Given** SMART stub adapter + fixture well-known/token responses, **When** adapter runs through broker, **Then** fixture receipts are durable and no live partner host is contacted.
2. **Given** FHIR partner adapter + synthetic Bundle/Patient fixtures, **When** retrieve/search is invoked via broker, **Then** returned bytes are treated as interchange; profile/integrity EvaluationRecords attach as evidence only.
3. **Given** REAL_PHI / PRODUCTION_CREDENTIALS / PARTNER endpoint gates closed-not-granted, **When** adapter is asked for live partner mode, **Then** it refuses fail-closed and documents EXTERNAL_GATES.

---

### User Story 5 - FHIR Profile / Integrity / Conformance Evidence (Priority: P1)

Brokered FHIR interchange paths can attach profile validation, integrity (digest/byte identity), and conformance evidence as EvaluationRecords. Validators/Inferno remain evidence oracles—not ClinicalAssertion authority. Synthetic fixtures only in this unit.

**Why this priority**: Roadmap FHIR profile/integrity/conformance evidence; Spec 003 validator evidence-only continuity.

**Independent Test**: Fixture FHIR resource with known profile outcome → EvaluationRecord; tampered bytes fail integrity check; Inferno/SMART kit referenced as EXTERNAL_CONFORMANCE_PASS candidate evidence class, not product authority.

**Acceptance Scenarios**:

1. **Given** synthetic FHIR fixture + profile oracle/fixture result, **When** attached post-broker retrieve or pre-send, **Then** EvaluationRecord stores evidence_only outcome.
2. **Given** bytes whose digest mismatches declared integrity, **When** integrity check runs, **Then** fail-closed; no ClinicalAssertion creation.
3. **Given** conformance harness evidence (fixture or optional offline Inferno report digest), **When** archived, **Then** claim scope is EXTERNAL_CONFORMANCE evidence—not runtime requirement for every CI job.

### Edge Cases

- Broker invoked with empty body but PHI-class data-class label: still requires allowlist match; synthetic-only enforcement refuses RealPhi class.
- Allowlist entry expired / disabled: treated as unknown → Deny.
- Transport timeout on allowed fixture/live-stub: receipt records FAILED/UNKNOWN-class outcome marker without blind retry (014 owns full machine; 013 must not invent silent retry loops).
- Worker requests broker capability: denied unless explicitly capability-granted; workers still receive no ambient network.
- Concurrent broker calls: serialized or lease-scoped per Core Host rules; receipts distinct.
- Doctor network_broker section: updates from Spec 006 placeholder to implemented status reflecting allowlist presence + DEFAULT_DENY-except-broker posture.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a Network Broker as the **sole** product online egress abstraction; all product runtime network I/O MUST pass through it.
- **FR-002**: Every broker request MUST declare destination, purpose, data-class, and authorization context; every decision MUST yield a durable receipt.
- **FR-003**: Broker MUST use a fail-closed destination allowlist; unknown destinations MUST be denied with no transport send.
- **FR-004**: Purpose and data-class MUST match allowlist policy or the request MUST be denied.
- **FR-005**: Product crates (CLI, Desktop, workers, future HF clients) MUST NOT embed uncontrolled HTTP/provider clients that bypass the broker.
- **FR-006**: Durable receipts MUST follow Spec 002 ActionAuditRecord and EvaluationRecord patterns (evidence_only for evaluations).
- **FR-007**: System MUST provide partner FHIR and SMART adapter interfaces; Spec 013 MAY satisfy them with stubs + fixture receipts.
- **FR-008**: Live partner FHIR/SMART endpoints and production credentials MUST remain EXTERNAL_GATES; Spec 013 MUST NOT require them to close.
- **FR-009**: FHIR profile, integrity, and conformance outcomes MUST attach as EvaluationRecord evidence only—never as ClinicalAssertion authority.
- **FR-010**: Spec 013 MUST remain synthetic/fixture-only for clinical content; REAL_PHI remains NOT_AUTHORIZED.
- **FR-011**: Spec 013 MUST NOT mutate MESC, import MESC runtime, or treat OpenMed/model output as ClinicalAssertion.
- **FR-012**: Admitted HTTP client MUST be pinned, prefer rustls, and remain minimal; TLS via rustls only (no native-tls product path).
- **FR-013**: DEFAULT_DENY remains the product posture except for broker-mediated, allowlisted, authorized egress.
- **FR-014**: Spec 013 MUST NOT implement Spec 014 full external-action state machine or NPHIES product workflows; it MAY record attempt outcomes compatible with later 014 binding.
- **FR-015**: `medscale doctor` network_broker axis MUST report broker availability/allowlist posture (no longer NotImplemented) without emitting secrets or tokens.

### Key Entities

- **NetworkBrokerRequest**: Destination, purpose, data-class, authorization, payload digest/class, correlation id.
- **NetworkBrokerDecision**: Allow | Deny + reason codes.
- **NetworkBrokerReceipt**: Durable binding of request decision/outcome via ActionAuditRecord (+ optional EvaluationRecord evidence).
- **EgressAllowlistEntry**: Fail-closed policy row (destination pattern, purposes, data-classes, enabled).
- **FhirPartnerAdapter**: Typed interface for partner FHIR interchange via broker.
- **SmartAuthAdapter**: Typed SMART stub/adapter interface (discovery/token fixture path in 013).
- **FhirConformanceEvidence**: EvaluationRecord payload for profile/integrity/conformance.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Bypass suite: 100% of attempted product egress outside broker fails closed or is architecturally impossible (0 admitted direct HTTP client uses in product crates outside broker).
- **SC-002**: Allowlist suite: unknown destination, purpose mismatch, and data-class mismatch each produce Deny + durable receipt; 0 transport sends on Deny.
- **SC-003**: Receipt suite: every Allow and Deny path yields queryable ActionAuditRecord; EvaluationRecords used for profile/conformance carry `evidence_only = true`.
- **SC-004**: SMART/FHIR adapter suite completes against fixtures with durable receipts and zero live partner hosts contacted.
- **SC-005**: Integrity/profile evidence suite: tampered bytes fail; valid fixture attaches EvaluationRecord without ClinicalAssertion auto-create.
- **SC-006**: Doctor reports network_broker as implemented/available with DEFAULT_DENY-except-broker posture; no tokens/secrets in output.
- **SC-007**: Deliverable introduces no REAL_PHI authorization, no MESC mutation, no uncontrolled provider client, and no requirement for live partner endpoints to close Spec 013.

## Assumptions

- Spec 005 and Spec 006 are `CLOSED_CANONICAL` (EncryptedVault, Core Host facade, CLI/Desktop clients, doctor placeholder for network_broker).
- Spec 002 object classes (EvaluationRecord, ActionAuditRecord) and Spec 003 FHIR interchange + evidence-only validator rules remain binding.
- FHIR R4 4.0.1 remains interchange baseline—not canonical DB.
- Inferno core v1.4.2 / SMART test kit remain EXTERNAL_CONFORMANCE / REFERENCE_ONLY per SOURCE_ACQUISITION.
- Ordinary pins recorded in `research.md` (ureq + rustls path); admission before first dependency line.
- Spec 014 owns durable external-action PENDING→SENT→CONFIRMED/FAILED/UNKNOWN machine and NPHIES workflows.
- Spec 015 will consume this broker for online pack/HF acquisition—no HF client in 013.
- Workers remain without ambient network; any future worker egress is capability-mediated through broker only.
