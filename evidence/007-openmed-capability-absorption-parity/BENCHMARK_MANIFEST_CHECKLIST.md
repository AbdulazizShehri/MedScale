# BenchmarkManifest checklist (Spec 007)

Mirror of `specs/007-openmed-capability-absorption-parity/contracts/benchmark-manifest.md`.

Before any `PARITY` / `SURPASS` / `ABSORB_PATTERN` claim language in evidence:

| Field | Required |
|---|---|
| manifest_id | yes |
| claim_type | yes (`PARITY` \| `SURPASS` \| `ABSORB_PATTERN` \| `WAIVER`) |
| capability_id | yes |
| openmed_commit | yes — must equal pin for floor claims |
| medscale_commit | yes |
| artifact_hashes | yes |
| corpus_hash | yes |
| rights | yes |
| hardware_os_runtime | yes |
| quality_metrics | yes |
| latency_cold_start_throughput | optional |
| memory | optional |
| network_privacy_observation | yes |
| failure_behavior | yes |
| limitations | yes |
| harness_revision | yes |
| recorded_at | yes |

## Forbidden claim shortcuts

- Using **raw model count** as a parity metric  
- Comparing to an **unpinned** OpenMed tip as the floor  
- Claiming **SURPASS** without exact-head BenchmarkManifest  
- Treating design-only Spec 007 corpora as measured parity  

## Cross-links (SURPASS_PENDING)

| capability_id | Note |
|---|---|
| privacy-observability | Spec 006 `PRIVACY_PROOF` is baseline; surpass needs cross-platform release evidence |
| local-offline-useful-operation | Offline proof + broker DEFAULT_DENY; surpass pending scripted multi-surface evidence |
| saudi-arabic-pii-clinical-ner | Program designed; measure in 008 |
| cli | Spec 006 CLI wedge closed; surpass pending comparative harness |
| cross-surface-consistency | CLI/Desktop present; mobile still Spec 009 |
