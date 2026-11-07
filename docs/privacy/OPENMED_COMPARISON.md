# MedScale and OpenMed: privacy capability comparison

**What this is:** a source-level comparison of OpenMed's privacy, de-identification and compliance tooling with what MedScale already has, and the plan for closing each gap. It is engineering analysis, not a legal opinion or a certification of either project.

**Sources read**
- OpenMed `maziyarpanahi/openmed@5e1eb45998700174dc8fd6a4da17198b04e9d1f8`, Apache-2.0. Read-only shallow clone; no model weights, datasets or packages were downloaded or run.
- MedScale `main@8ce211905b1216142ced36cd02a9965d9b45a941`.
- Read on 2026-11-07 by AI research agents. Every OpenMed claim below was checked against source files, not documentation alone. Nothing was executed, so behaviour is as read, not as observed.

## Summary

OpenMed is a broad Python toolkit: model-led PII detection in 44 languages, a large checksum library, k-anonymity and differential-privacy maths, DICOM and PDF redaction, and audit tooling. Several of its compliance features are thinner than their names suggest.

MedScale already has a complete deterministic Privacy Gate (Spec 079) with classification, profiles, keyed pseudonyms, receipts, a residual re-scan and fail-closed egress. Most of it is not reachable from the desktop app, and it does not model HIPAA Safe Harbor.

The plan is to extend MedScale's Privacy Gate, port a small set of pure algorithms from OpenMed with attribution, and fix the weaknesses found in OpenMed rather than copying them.

## Comparison matrix

Status values: **Have** (implemented and tested), **Hidden** (implemented, not exposed in the desktop app), **Partial**, **Missing**, **Planned** (in this programme).

| Capability | OpenMed (verified in source) | MedScale today | Gap | Choice | Tests and evidence needed | Status |
|---|---|---|---|---|---|---|
| Data classification | Labels only | `DataClass` with 4 classes, `ArtifactClassification` (`contracts/privacy_gate.rs:70,127`) | Classifying is command-line only | Expose in desktop | Desktop command tests | Hidden |
| Deterministic PII detection | Regex merger and safety sweep over model output; no regex-only fallback (`core/pii.py:592-610`) | `privacy_recognizers.rs`: names, dates, email, URL, IP, phone, identifiers, postcodes, addresses; FHIR-aware mode | 10 span kinds, not the 18 Safe Harbor categories | Add the missing kinds to `SensitiveSpanKind` | Per-category positive and negative tests | Partial → Planned |
| Model-based detection | 174 labels, about 100 national-ID validators, 44 languages | ONNX and Candle token classification; OpenMed v3 merger port (`medscale-pack/src/pii_merge.rs`) | Model recognizer receipts say "not qualified" | Keep rules as the gate; models only add recall | Leakage benchmark per category | Partial |
| Arabic | Arabic model; regex for dates, phones, addresses, postcodes; Egyptian and Moroccan IDs; **no Saudi national ID or Iqama** | Arabic speech only | No Arabic text identifiers | Write Saudi national ID and Iqama (Luhn-variant check), Gulf phones, Arabic-Indic digit folding, Hijri dates | Arabic adversarial set | Planned |
| Safe Harbor 18 categories | Category table and attestation only. Six categories have no detector. No restricted-ZIP3 list. Dates keep the year only if the caller asks | Absent | Everything | Built-in `hipaa_safe_harbor` profile: dates to year, ages over 89 aggregated, ZIP3 with the restricted list, all 18 categories, fail closed on unknown fields | Per-category leakage must be 0 on the synthetic set | Planned |
| FHIR de-identification | Generic walker sends every string through NER. **Leaves `birthDate`, all dates, ids and references unchanged** (`interop/fhir_operations.py:61-114`) | FHIR-aware recognizer; export path not privacy-gated (`facade.rs:2566`) | Export bypasses the gate | Per-resource field rules; consistent keyed pseudonyms for ids and references across a Bundle; unknown fields fail closed | Round-trip tests on US Core and IPS examples | Planned |
| HL7 v2 | Field map over PID, NK1, GT1, IN1 and others; incomplete escaping; unsalted hashing | Absent | Absent | Not in this programme; port the field table with escaping and keyed hashing when HL7 import exists | — | Deferred |
| PDF and images | Re-rasterised redaction with burned-in boxes; scanned PDFs depend on OCR | Absent | Absent | Deferred until document import exists; follow OpenMed's re-raster approach without AGPL renderers | — | Deferred |
| DICOM | PS3.15 profile table; public UID salt; audit stores unsalted hashes of names and birth dates | Absent | Absent | Deferred; if ported, key the UID remap and keep PHI hashes out of audit | — | Deferred |
| Pseudonyms | Faker seeded by an unkeyed hash (default seed 0); vault uses a home-made stream cipher | HMAC pseudonyms with the key in the KeyStore; audited re-identification | Not exposed | Keep MedScale's keyed design; expose in desktop | Re-identification audit tests | Hidden |
| Date shifting | HMAC per-patient offset, non-zero, ±365 days | `generalize()` only | No shifting | Port `stable_offset_for` with a vault-held key; label it pseudonymisation, never Safe Harbor | Interval preservation tests | Planned |
| Hijri calendar | Umm al-Qura table 1356–1450 AH, pinned by digest | Absent | Absent | Port the table converter with the digest pin | Conversion tests at both ends of the table | Planned |
| k-anonymity, l-diversity, t-closeness | k, distinct and entropy l, total-variation t; exhaustive lattice search capped at 100k nodes; pure Python | Absent | Absent | Port k, distinct/entropy l, add recursive (c,l) and EMD t-closeness for numbers | Known-answer tests | Planned |
| Prosecutor, journalist, marketer risk | `structured/reid_report.py:30-180` | Absent | Absent | Port | Known-answer tests | Planned |
| Expert Determination support | Evidence bundle titled "Not an Expert Determination"; Ed25519 attestation envelope for a named expert | Absent | Absent | Evidence package only; a field for an external expert's signed attestation, never generated by MedScale | Tests that the app cannot emit an attestation by itself | Planned |
| Leakage benchmarks | Grapheme-weighted leakage per label and language; synthetic data only; targets marked synthetic | Residual re-scan; privacy audits of installed apps (0 markers in 143 files) | No per-category metrics | Per-category leakage and over-redaction on a synthetic Arabic and English corpus, run in CI and on installed builds | Published metrics with the corpus digest | Planned |
| Receipts and evidence | HMAC-signed audit reports; Ed25519 only for expert attestations | `DeidReceipt` binds source and output digests and profile revision; bans "compliant", "safe" and "anonymi" in receipts | No report type | `DeidEvidenceReport` built on `DeidReceipt` | Tamper tests | Planned |
| Audit trail integrity | Unkeyed SHA-256 hash chain; can be rebuilt by anyone with write access | Append-only rows, no chain | Rows can be edited or lost | Keyed hash chain with a sealed head and generation counter (also readiness blocker B4) | Insert, delete, reorder and rollback tests | Planned |
| Offline enforcement | Patches Python sockets only | Default-deny network broker; refuses real-PHI egress; network idle checked on installed builds | — | Keep | Existing qualification | Have |
| Telemetry | Closed allowlist of counters | No telemetry | — | Keep none | — | Have |
| Model provenance | Pinned commit plus per-file digest; optional Sigstore; check can be skipped by an environment variable | Pinned digests; Pack trust root still a founder decision | Production trust root | Founder decision | — | Partial |
| Release gates | Licence allow and deny lists, pip-audit, SBOM, pinned Actions | `cargo-deny`, pinned Actions, installed-build qualification | No SBOM | Add a CycloneDX SBOM to release qualification | SBOM in the release packet | Planned |

## Where MedScale should do better than OpenMed

- **Rules decide, models assist.** Safe Harbor removal must not depend on a model noticing a value. Structured fields are handled by fixed rules; free text is handled by rules plus optional models, and anything unrecognised blocks the export.
- **Dates and ids in FHIR are de-identified,** not passed through.
- **Every hash that touches an identifier is keyed.** That covers pseudonyms, surrogates, audit fingerprints and UID remaps.
- **Evidence is signed with Ed25519,** so a reader can check a report without being able to forge one.
- **The audit chain is keyed and its head sealed,** so rebuilding it is detectable.
- **Saudi identifiers are covered,** which OpenMed lacks.
- **Qualification runs on the exact installed build,** not only on unit tests.

## Licence and provenance ledger

| Item | Source | Licence | Use in MedScale | Attribution |
|---|---|---|---|---|
| OpenMed SDK source | `maziyarpanahi/openmed@5e1eb45` | Apache-2.0 | Algorithms ported to Rust: date-shift offset, calendar converters and Umm al-Qura table, k/l/t metrics, prosecutor/journalist/marketer risk, checksum validators, leakage accounting | Required: NOTICE entry and a header in each ported file naming the source file and commit |
| OpenMed v3.0.0 PII merger | `ea920f36` | Apache-2.0 | Already ported (`medscale-pack/src/pii_merge.rs`, `pii_patterns.rs`) | Check the existing NOTICE entry covers it |
| OpenMed model catalog | `third_party/medscale-model-catalog-v3` | Per-model licences | Data only; no weights shipped | Per-row licence field |
| OpenMed models | Hugging Face | Varies by model | **Not adopted.** No weights downloaded | — |
| Umm al-Qura table | OpenMed, sourced from hijridate 2.6.0 | hijridate is believed MIT; confirm before porting | Port with the digest pin | Name both sources |
| OpenMed GPL extras (extract-msg, sdcMicro) | Optional extras | GPL | **Not used** | — |
| ARX | External program | Apache-2.0 | Not used | — |

Before any port lands, the ported file's header and `docs/legal/NOTICE_INVENTORY.md` must name the OpenMed file and commit.

## What this does not establish

- That MedScale meets HIPAA, GDPR, Saudi PDPL or the EU AI Act. Those need legal review and, for HIPAA Expert Determination, a qualified expert.
- That any de-identification here is safe for real patient data. `REAL_PHI_AUTHORIZATION` stays `NOT_AUTHORIZED` until the conditions in `docs/beta/REAL_DATA_READINESS.md` are met.
- That OpenMed's behaviour matches what was read; nothing was executed.
