# Spec 075 — Data Source Fabric + Data Workbench Foundation

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promoted:** 2026-09-19
**Base SHA:** `ae0441918296c2d1510a71061249c7e55e65d760`
**Target branch:** `spec/075-data-source-fabric`
**Dependency:** canonical MedScale through Spec 074 + merged Clinical + Research Intelligence planning packet PR #128
**Promotion authority:** `docs/planning/SPEC_075_PROMOTION.md`

## 1. Problem

MedScale can organize research work into Projects and Experiments (Spec 074), but it has no governed data-source plane. Without one, every future consumer — Analytics, cohort builder, R Workspace, Evidence/Graph work, MedAgent, Research Packs — would fetch and interpret datasets independently: separate file parsers, separate database readers, separate download logic, separate snapshot semantics, separate provenance. That produces N incompatible dataset truths, silent schema drift, unrepeatable analyses, and credential handling scattered across subsystems.

## 2. Goal

Add a local-first **Data Source Fabric + Data Workbench Foundation**: one MedScale-owned source model (source identity, credential references, discovery, preview, import/materialization, immutable snapshot lineage, schema fingerprints, refresh intent, receipts, failure states) plus one native Data Workbench for inspecting snapshots, saving views, and applying deterministic transformations.

At closure, a user can create/open a Project, import a local tabular dataset or read/snapshot a supported external data source, inspect the immutable snapshot in the native MedScale Data Workbench, create a saved view and one deterministic transformation, close/reopen MedScale, and reproduce the source → snapshot → transformation lineage without a cloud dependency for local sources.

## 3. User and operational scenarios

1. A researcher creates a local CSV source in a Project; MedScale validates, quarantines on failure, discovers schema, and materializes an immutable DataSnapshot.
2. The researcher inspects the snapshot in the Data Workbench grid with typed columns, row detail, filter, sort, group, and paging.
3. The researcher saves a Grid view and opens Form/Gallery/Kanban/Calendar/summary projections over the same snapshot without duplicating data.
4. The researcher applies a deterministic transformation (select/drop/rename/cast/filter/sort); the output is a new immutable DataSnapshot with exact lineage back to the parent.
5. The application restarts; sources, snapshots, saved views, and transformation lineage resolve identically.
6. The external source file changes; the old snapshot is byte-identical and a refresh creates a new snapshot with a refresh receipt — old snapshots are never rewritten.
7. A malformed/corrupt/partial import produces an explicit safe state (quarantine + reason), never a half-materialized snapshot.
8. A database credential is revoked mid-session; in-flight reads cancel or time out with explicit state, and no secret appears in logs or persisted artifacts.
9. A vault created before Spec 075 opens and migrates (v3 → v4) without changing old object IDs or breaking old CLI/Desktop workflows.
10. With network egress disabled, all local-source workflows remain fully useful; remote dataset adapters report explicit unavailable/denied states.
11. An interrupted/crashed snapshot write does not leave a half-materialized snapshot visible to queries.

## 4. Required outcomes

1. Bounded contracts for DataSourceManifest, DataSourceKind, capabilities, source health, SourceSchema, DataSnapshot, SnapshotPart, import/refresh receipts, SavedDataView, DataTransformation, and transformation receipts.
2. Reuse of existing `OpaqueId`, `ObjectHeader`, `DigestSha256`, realm/scope, and provenance/audit primitives; no new ID or provenance foundation.
3. Encrypted-vault persistence for sources, snapshots, snapshot parts, receipts, saved views, and transformation lineage inside the existing storage architecture (schema v3 → v4, additive).
4. Local file vertical slice: candidate → validation/quarantine → schema discovery → immutable snapshot → Project attachment → CLI inspect → restart/reopen, starting with CSV/TSV.
5. At least one evidence-selected read-only database adapter with opaque credentials, bounded discovery, immutable snapshots, timeout/cancel, schema-change state, and restart durability.
6. Remote dataset adapter foundation (Hugging Face / Kaggle) through the existing Network Broker, or an explicit canonical deferral recorded in this package.
7. Native Slint Data Workbench backed exclusively by Core: dataset/snapshot selection, typed columns, virtualized/paged rows, row detail, filter/sort/group, empty/loading/error/corrupt/partial states, provenance inspector, light/dark parity, keyboard/focus.
8. Saved and alternate views as projections over the same canonical snapshot.
9. Deterministic transformation foundation whose durable output is a new immutable DataSnapshot with replayable lineage.
10. Versioned Dataset Card plus immutable release manifest only if retained at contract freeze; annotation workflow itself stays out.
11. Migration/reopen/backup-recovery evidence from representative pre-075 vault fixtures.
12. Scale measurements (large table, pagination/virtualization) without unsupported performance claims.
13. Exact-head closure evidence mapping every acceptance criterion to proof.

## 5. Explicit non-goals

Spec 075 MUST NOT implement:

- team collaboration, shared rooms, sync, or Hub behavior (Spec 076/084);
- MedAgent or evidence copilot (Spec 077);
- model fleet training/comparison (Spec 078);
- Privacy Gate expansion beyond using current data classification (Spec 079);
- governed web browsing or medical literature acquisition (Spec 080);
- AudioFlow, ASR, diarization, or clinical scribing (Spec 081/088);
- analytics/statistical methods, cohort semantics, or DataFusion ownership (Spec 082);
- Clinical Graph construction or knowledge/research canvas (Spec 083);
- MedScale Compute or arbitrary code execution (Spec 085);
- R Workspace or Rscript execution (Spec 086);
- community extensions or extension sandboxes (Spec 087);
- Research/Evidence Packs beyond the 075 dataset-release foundation if retained (Spec 089);
- institutional FHIR/EHR writes or clinical effects (Spec 090);
- federation (Spec 091);
- arbitrary Python/JavaScript/shell/R formula execution in transformations;
- a second database, vault, blob lifecycle, outbox, ID namespace, or authority plane;
- real PHI fixtures or any real-PHI authorization claim;
- MESC coupling of any kind.

## 6. Compatibility and reliance

- Pre-075 CLI/Desktop behavior is preserved; pre-075 vaults migrate without identity rewrite.
- Spec 074 Project/Experiment/artifact-ref APIs are reused for Project attachment; 075 adds no parallel project system.
- Existing Network Broker allowlist/transport semantics govern all remote dataset access; no new product runtime network path.
- Existing secret/keystore conventions govern credential references; credentials themselves never enter snapshots, receipts, logs, or backups as plaintext.

## 7. Missing-domain answers (frozen at T075-01)

- **Identity:** dataset rows have no patient-identity semantics in 075; row identity is positional/content digest within a snapshot. No cross-source entity merge is performed or implied.
- **Time:** source observation time, ingestion time, and snapshot creation time are recorded separately where the source provides them; repository metadata uses existing system-time/audit conventions, not `MedicalTime`.
- **Rights:** every source records license/rights metadata and a rights state; redistribution-limited sources carry explicit non-redistribution markers. No licensed dataset content enters the repository.
- **Privacy:** sources carry data classification; purpose/retention/egress follow current vault policy. No new egress is introduced; remote adapters inherit broker data-class rules.
- **Provenance:** every snapshot binds exact source identity + revision (digest/version/files), transform identity + parameters, engine/tool versions, and reviewer state where review exists.
- **Failure:** denied, unavailable, timeout/cancelled, partial, stale, conflicting, unknown, corrupt, and unsupported-version states are distinct typed states end to end.
- **Recovery:** restart, migration, backup, rollback via pre-migration backup restore, interrupted-operation fail-closed, and late-external-result reconciliation rules are defined in `migration.md` and tested.

## 8. Acceptance criteria

The fourteen frozen acceptance requirements are listed in `docs/planning/SPEC_075_PROMOTION.md` ("Frozen acceptance requirements") and mapped to proof in `plan.md` T075-10 and the `evidence/075-data-source-fabric/` packet. They are not restated here to avoid divergence; the promotion document is authoritative for acceptance wording.
