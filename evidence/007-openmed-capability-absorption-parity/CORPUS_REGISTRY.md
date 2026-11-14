# Corpus registry (Spec 007)

**phi_class**: `synthetic_only` for all rows  
**harness_owner_spec**: `008` (measurement execution)  
**baseline_commit**: `59d9cb0a2e0ccbba8fa3d891a66d83ffaf45e837`

## Corpora

### om-ner-synthetic-en-v0

| Field | Value |
|---|---|
| capability_ids | `clinical-ner` |
| languages | `en` |
| data_class | synthetic |
| metrics | span_f1 (↑, required), precision (↑, required), recall (↑, required), latency_p50_ms (↓, placeholder) |
| anti_metrics | `raw_model_count` |
| rights_note | Synthetic MedScale fixtures only |
| status | design |
| fixtures | `fixtures/ner-en-smoke.json` |

### om-pii-synthetic-multi-v0

| Field | Value |
|---|---|
| capability_ids | `pii-detection` |
| languages | `en`, `ar` |
| trap_classes | code_switch, dialect, national_id_synthetic |
| metrics | phi_recall (↑, required), false_negative_rate (↓, required) |
| anti_metrics | `raw_model_count` |
| rights_note | Synthetic PII patterns; no real identifiers |
| status | design |
| fixtures | `fixtures/pii-multi-smoke.json` |

### om-deid-synthetic-v0

| Field | Value |
|---|---|
| capability_ids | `de-identification` |
| languages | `en` |
| metrics | span_fidelity (↑, required), transformation_record_present (↑, required), hidden_egress_count (↓, required=0) |
| anti_metrics | `raw_model_count` |
| rights_note | Synthetic only |
| status | design |

### om-multilingual-smoke-v0

| Field | Value |
|---|---|
| capability_ids | `multilingual-medical-nlp` |
| languages | multi (registry honesty) |
| metrics | supported_language_coverage (↑), unsupported_fail_closed (↑, required) |
| anti_metrics | `raw_model_count` |
| rights_note | Synthetic smoke strings |
| status | design |

### om-saudi-arabic-design-v0

| Field | Value |
|---|---|
| capability_ids | `saudi-arabic-pii-clinical-ner`, `national-id-validation` |
| languages | `ar`, `ar-SA`, `en+ar` |
| trap_classes | msa_vs_dialect, code_switch, national_id_synthetic, critical_number, rtl_entity |
| metrics | span_f1 (↑), phi_recall (↑), critical_number_error_rate (↓) |
| anti_metrics | `raw_model_count` |
| rights_note | Synthetic Arabic/PII traps; see SAUDI_ARABIC_PROGRAM.md |
| status | design |
| fixtures | `fixtures/saudi-arabic-traps-v0.json` |

### om-terminology-contract-v0

| Field | Value |
|---|---|
| capability_ids | `terminology-grounding` |
| languages | n/a |
| metrics | candidate_rank_stability (↑), abstention_correctness (↑), no_licensed_table_bytes (↑, required) |
| anti_metrics | `raw_model_count` |
| rights_note | Contract fixtures only — **no** SNOMED/LOINC/UMLS table payloads |
| status | design |

### om-unicode-offset-handoff-v0

| Field | Value |
|---|---|
| capability_ids | `unicode-offset-semantics` |
| languages | `ar`, `en` |
| metrics | offset_roundtrip_ok (↑, required), arabic_normalization_versioned (↑) |
| anti_metrics | `raw_model_count` |
| rights_note | Synthetic adversarial strings |
| status | design |

## Measurement ownership

Spec 007 closes on **designed** corpora. Spec 008 executes harnesses and may promote status to `seeded` / `frozen` / `measured` under BenchmarkManifest discipline.
