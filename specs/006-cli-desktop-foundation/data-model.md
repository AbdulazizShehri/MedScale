# Data Model: Spec 006 CLI + Desktop Foundation

**Date**: 2026-08-25  
**Status**: Planning-canonical for Spec 006 implement  
**Persistence**: No new durable clinical object classes. Presentation remains Spec 004 Projection bodies. Spec 006 adds **operational / evidence** types consumed by CLI, doctor, and evidence archives. Synthetic-only.

## Principles

1. Spec 002–005 class invariants remain in force (Proposal≠Assertion; Projection≠truth; EncryptedVault production path).
2. CLI/Desktop hold **no** ambient DurableStore connection or vault DEK.
3. Doctor and PRIVACY_PROOF types are non-authoritative operational evidence—not ClinicalAssertions.
4. REAL_PHI unauthorized; fixtures synthetic.
5. DEFAULT_DENY network; no MESC objects.

## Inherited (read-only consumption)

| Source | Types used by CLI wedge |
|---|---|
| Spec 002 | AuthorityRequest/Response, Capability, VaultLease, AuthorityError |
| Spec 003 | IngestOutcome / IngestReceipt, claim path semantics |
| Spec 004 | SubjectTimelineV1, SubjectBriefV1, SubjectCoverageV1 |
| Spec 005 | EncryptedVaultDescriptor, KeyProvider status, VaultLocationPolicy |

## New Spec 006 entities

### DoctorReport

| Field | Required | Description |
|---|---|---|
| `product_name` | yes | e.g. MedScale |
| `version` | yes | CoreFacade / crate version |
| `local_only` | yes | bool; product egress posture |
| `network_posture` | yes | `DefaultDeny` |
| `vault` | yes | `DoctorVaultSection` |
| `sync_remote_risk` | yes | enum status + notes |
| `key_store` | yes | availability enum + backend label (non-secret) |
| `privacy_evidence` | yes | freshness + artifact id/path |
| `filesystem_claim` | yes | claim status / OS notes |
| `packs_runtime` | yes | placeholder until 008 |
| `network_broker` | yes | placeholder until 013 |
| `limitations` | yes | short string list |

### DoctorVaultSection

| Field | Required | Description |
|---|---|---|
| `status` | yes | `NotConfigured` \| `Configured` \| `Open` \| `Error` |
| `root_path` | optional | claim-scoped path when known |
| `root_class` | optional | e.g. `LocalAppData` |
| `vault_id` | optional | opaque id |
| `encryption_profile` | optional | from Spec 005 when open/configured |

### SyncRemoteRisk

```text
Ok | Refused | Unknown | NotConfigured
```

Plus optional human-readable `detail` (no secrets).

### KeyStoreAvailability

```text
Available | Unavailable | Mock | Unknown
```

Backend label examples: `windows-native-keyring-store`, `linux-keyutils`, `mock`—never key material.

### PrivacyEvidenceFreshness

| Field | Required | Description |
|---|---|---|
| `status` | yes | `Fresh` \| `Stale` \| `Missing` |
| `artifact_id` | optional | PRIVACY_PROOF id |
| `observed_at` | optional | ISO-8601 / MedicalTime as documented |
| `path` | optional | evidence path relative to repo/evidence root |

### PrivacyProof

| Field | Required | Description |
|---|---|---|
| `schema_id` | yes | e.g. `medscale.privacy_proof.v1` |
| `spec_id` | yes | `006` |
| `generated_at` | yes | |
| `platform` | yes | OS / arch |
| `claim_scope` | yes | attributable boundary description |
| `network_observation` | yes | expected empty product egress |
| `dependency_capability_audit` | yes | notes / hashes |
| `log_crash_marker_scan` | yes | results for CLI/Core Host |
| `webview_cache_scan` | yes | `NotApplicable` when Tauri deferred |
| `vault_sync_root_evidence` | yes | references doctor/claim tests |
| `limitations` | yes | list including synthetic-only, no Tauri, no REAL_PHI, no system-wide zero packets |
| `related_doctor_report_digest` | optional | |

### CliSession

| Field | Required | Description |
|---|---|---|
| `session_id` | yes | opaque |
| `vault_id` | optional | |
| `lease_id` | optional | when vault open |
| `host_mode` | yes | `InProcessTransient` (006) |

Not persisted across processes as authority; may be process-local only.

### DesktopScaffoldStatus

| Field | Required | Description |
|---|---|---|
| `shell_kind` | yes | `ScaffoldNoWebView` |
| `tauri_admitted` | yes | false for Spec 006 |
| `facade_reachable` | yes | bool smoke |
| `v0_integrated` | yes | bool; false if deferred |

### LongitudinalWedgeReceipt

| Field | Required | Description |
|---|---|---|
| `vault_id` | yes | |
| `ingest_receipt_ids` | yes | |
| `timeline_digest` | optional | |
| `brief_digest` | optional | |
| `coverage_digest` | optional | |
| `offline` | yes | must be true |
| `synthetic_only` | yes | must be true |

## Serialization

- Doctor: human text (default) + `--json` using serde.
- PRIVACY_PROOF: JSON evidence file under `evidence/006-cli-desktop-foundation/`.
- Presentation CLI: human text or `--json` emitting Spec 004 bodies.

## Non-entities (forbidden)

- Raw DEK / passphrase / recovery codes as doctor fields
- Direct SQL connection handles in CLI session structs
- WebView cookie/cache identifiers as “privacy PASS” without scan
- REAL_PHI markers
