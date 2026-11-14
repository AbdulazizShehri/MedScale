# Saudi / Arabic benchmark program (Spec 007)

| Field | Value |
|---|---|
| program_id | `saudi-arabic-pii-ner-v0` |
| baseline_commit | `59d9cb0a2e0ccbba8fa3d891a66d83ffaf45e837` |
| corpus_ids | `om-saudi-arabic-design-v0` |
| phi_policy | `synthetic_only` |
| measurement_owner_spec | `008` |
| surpass_policy | `forbidden_until_exact_head_benchmark` |
| status | `designed` |

## Trap classes (≥4)

1. **msa_vs_dialect** — MSA vs Gulf/Saudi dialect entity spelling  
2. **code_switch** — English↔Arabic medical/PII mixing in one span  
3. **national_id_synthetic** — synthetic national-ID shaped tokens (never real IDs)  
4. **critical_number** — doses, labs, dates where digit error is high-cost  
5. **rtl_entity** — RTL/LTR boundary and bidirectional mark traps  

## Metrics

| name | higher_is_better | required_for_parity |
|---|---|---|
| span_f1 | true | true |
| phi_recall | true | true |
| critical_number_error_rate | false | true |
| latency_p50_ms | false | false (008 fills) |

## Surpass gate

No SURPASS claim vs OpenMed floor until Spec 008 produces BenchmarkManifest with exact OpenMed pin commit, MedScale commit, corpus hash, and harness revision. Design completion here does **not** authorize superiority language.
