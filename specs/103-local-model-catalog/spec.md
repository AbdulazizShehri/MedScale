# Spec 103: Local model catalog and qualified runtime adapters

**Status:** `PROMOTED` by founder directive (2026-10-08, issue #183).
**Depends on:** Specs 015, 026, 069, 070, 071 and 078. For the UI phases, the merged Tauri desktop (Specs 096–101).
**Research:** [research.md](research.md) (exact-source gap analysis against OpenMed v3.0.0 `ea920f3`).

## Goal

MedScale gets its own path: **discover → inspect → optional acquire → verify → admit → load → run → unload**. It works for task- and hardware-qualified local models, at catalog scale (thousands of entries), offline by default, under Rust Core authority.

## Requirements

- **R103-01 Catalog.** A read-only, versioned catalog imported from an exact upstream snapshot (`models.jsonl` bytes plus SHA-256 plus commit). It supports at least 10,000 rows with pagination, full-text search over id, family, task, architecture and labels, and deterministic ordering. Importing contacts no network: the snapshot file is supplied locally.
- **R103-02 Filters.** Task, language, format, architecture, size tier, parameter count, license claim, estimated disk/RAM, and device fit (CPU-only or Apple-Silicon-only formats). Estimated values are labelled `estimated`, and missing values `unknown`.
- **R103-03 Status ladder.** Every row carries exactly one status from the ladder in research §6, plus the evidence that justifies it. Listing, caching, admission, runtime compatibility, execution tests, task qualification and clinical validation are separate states. Counts are reported per status only.
- **R103-04 Offline default.** No implicit Hub contact anywhere. Acquisition (phase 3) needs explicit per-artifact consent through the existing Network Broker. It pins a 40-hex revision, checks per-file SHA-256 and size limits, and supports interruption and resume. Failures leave no partial admitted state.
- **R103-05 Rights.** Admission requires a card-level license and terms record. Rows with `other` or missing licenses stay `RIGHTS_PENDING` until reviewed. Gated or redistribution-restricted weights are never bundled.
- **R103-06 Safe formats only.** Only safetensors and ONNX are admitted. `trust_remote_code`, pickle-based weights and custom code are refused.
- **R103-07 Runtime adapters.** Extend the existing `tract` ONNX token-classification runtime to the token-classifier architectures the catalog uses where `tract` supports the operators. Each adapter declares a compatibility predicate that is checked before load. Any other runtime (ORT, GGUF, MLX) needs its own dependency admission and is out of scope until admitted.
- **R103-08 Resources.** An LRU of prepared models with configured RAM and disk budgets, explicit unload, cancellation and deterministic eviction. CPU-only is the baseline.
- **R103-09 UI.** The Models route lists the catalog with filters, per-status counts, sizes (exact or estimated), license claim and card link, runtime availability, and "local only" / "not installed" states. Installed and listed models are visually separate. No hidden download.
- **R103-10 Fleet.** Admitted, runtime-compatible models can be used as Model Fleet lanes (Spec 078 semantics: factual comparison, no ranking).
- **R103-11 Claims.** UI, docs and release notes never state a bare model count. They state status-scoped counts (for example "N listed, M admitted, K executed in tests"). No clinical-superiority or parity claim without Spec 071-style evidence.

## Non-goals

- Bulk weight download.
- Clinical validation.
- Real PHI.
- Paid providers.
- GPU acceleration claims before hardware qualification.
- Mobile apps.
- Replacing the frozen v2.2.0 comparator.

## Release gate

`RELEASE_READY=false` until #183 deliverables are qualified. Release material may only quote status-scoped counts.
