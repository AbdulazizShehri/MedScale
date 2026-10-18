# OpenMed v3.0.0 head-to-head: 21 models (synthetic)

**Runs:**
- `39160798189` (pilot, 5 rows, branch `qual-batch/103-h2h-01`);
- `39166911548` (17 rows, branch `qual-batch/103-h2h-02`), `ubuntu-latest`.

Each row ran on a single runner:
1. MedScale ran the catalog-pinned snapshot.
2. Then pinned OpenMed (PyPI `openmed==3.0.0`, `torch==2.14.1+cpu`, `transformers==5.20.1`, `analyze_text` defaults) ran the same files.

Both sides were scored by MedScale's `entity_eval` on the same synthetic gold sets (40 documents each).

**Not claimed:** evaluation evidence on small synthetic gold sets. It is not clinical validation and not a general superiority claim.

## Summary

- **Accuracy (strict F1, MedScale's best decoding vs OpenMed's pipeline):** 11 wins, 10 ties (within ±0.005), 0 losses over 21 models.
  - PII: higher on 10 of 10 models (+0.007 to +0.19).
  - NER: level on 9 of 11 and higher on 2.
- **MedScale default decoding (`openmed_compatible`):**
  - Level with OpenMed on the BioMed-109M and BioMed-335M rows.
  - Well below on most others (as low as 0.016 strict F1 where OpenMed scores 1.000).
  - This decoder ports OpenMed's ONNX path, not its Python default.
  - The desktop's Try a model therefore preselects word-first decoding. Choosing the Core default is the next decision.
- **Warm latency (per-document p50):**
  - MedScale faster on 6 models: the four BioClinical-108M models, PharmaDetect-BioMed-109M, and Clinical Longformer (98 ms vs 770 ms).
  - Level on 1 (BioClinicalModern).
  - Slower on 14, up to 3.2×.
  - Median ratio 1.19× (MedScale slower).
- **Cold start:** MedScale prepares faster on every row (OpenMed's figure includes one document).
- **Peak RSS:** higher for MedScale as measured. Its process also builds and admits the signed Pack and runs five decodings, so its figure is an upper bound.
- **Not runnable by OpenMed's PyTorch path:** `OpenMed-NER-DiseaseDetect-BioMed-109M-mlx` (an MLX export). MedScale ran it; OpenMed's Python pipeline cannot load MLX weights on Linux.

## Rows

| Model (revision) | Gold set | OpenMed 3.0.0 strict F1 | MedScale default strict F1 | MedScale best strict F1 (decoding) | OpenMed p50 ms | MedScale p50 ms | OpenMed cold start ms | MedScale prepare ms | OpenMed peak RSS MiB | MedScale peak RSS MiB |
|---|---|---|---|---|---|---|---|---|---|---|
| `OpenMed/OpenMed-NER-AnatomyDetect-BioClinical-108M` (`f644583`) | medscale-ner-anatomy-synthetic-en | 0.870 | 0.833 | 0.889 (word_average) | 140.0 | 90.1 | 6351 | 1276 | 639 | 640 |
| `OpenMed/OpenMed-NER-AnatomyDetect-BioMed-109M` (`49bedb8`) | medscale-ner-anatomy-synthetic-en | 0.870 | 0.870 | 0.870 (openmed_default) | 11.6 | 36.5 | 3776 | 686 | 664 | 646 |
| `OpenMed/OpenMed-NER-AnatomyDetect-BioMed-335M` (`a227a43`) | medscale-ner-anatomy-synthetic-en | 0.851 | 0.851 | 0.851 (openmed_default) | 387.5 | 439.4 | 7336 | 3797 | 1056 | 1940 |
| `OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-135M` (`e26dc4b`) | medscale-ner-anatomy-synthetic-en | 0.833 | 0.400 | 0.833 (word_first) | 8.4 | 24.4 | 4720 | 1023 | 625 | 831 |
| `OpenMed/OpenMed-NER-DiseaseDetect-BioClinical-108M` (`f25fd4b`) | medscale-ner-disease-synthetic-en | 0.824 | 0.179 | 0.824 (word_first) | 140.2 | 90.5 | 6510 | 1290 | 639 | 640 |
| `OpenMed/OpenMed-NER-DiseaseDetect-BioMed-335M` (`a62e2a2`) | medscale-ner-disease-synthetic-en | 0.824 | 0.824 | 0.824 (openmed_default) | 389.0 | 445.0 | 6658 | 3816 | 1056 | 1940 |
| `OpenMed/OpenMed-NER-DiseaseDetect-BioPatient-108M` (`3c6d265`) | medscale-ner-disease-synthetic-en | 0.800 | 0.175 | 0.800 (word_first) | 139.9 | 88.7 | 6300 | 1278 | 639 | 640 |
| `OpenMed/OpenMed-NER-PharmaDetect-BioClinical-108M` (`c0a8d5e`) | medscale-ner-chemical-synthetic-en | 1.000 | 0.016 | 1.000 (word_first) | 141.2 | 90.1 | 6496 | 1280 | 639 | 640 |
| `OpenMed/OpenMed-NER-PharmaDetect-BioMed-109M` (`8e510d6`) | medscale-ner-chemical-synthetic-en | 0.979 | 0.581 | 0.979 (word_first) | 119.9 | 83.5 | 6775 | 1299 | 640 | 646 |
| `OpenMed/OpenMed-NER-PharmaDetect-BioMed-335M` (`e2b5518`) | medscale-ner-chemical-synthetic-en | 0.979 | 0.581 | 0.979 (word_first) | 386.6 | 439.0 | 6811 | 3805 | 1056 | 1940 |
| `OpenMed/OpenMed-NER-PharmaDetect-TinyMed-135M` (`954b76a`) | medscale-ner-chemical-synthetic-en | 0.979 | 0.000 | 0.979 (word_first) | 7.6 | 21.4 | 4127 | 911 | 624 | 832 |
| `OpenMed/OpenMed-PII-BioClinicalBERT-Base-110M-v1` (`87c4b15`) | medscale-pii-synthetic-en | 0.742 | 0.193 | 0.896 (word_first_pii_merge) | 61.7 | 104.9 | 6178 | 2430 | 822 | 840 |
| `OpenMed/OpenMed-PII-BioClinicalModern-Base-149M-v1` (`732b1c5`) | medscale-pii-synthetic-en | 0.749 | 0.296 | 0.867 (word_first_pii_merge) | 68.0 | 67.1 | 5340 | 2280 | 957 | 1183 |
| `OpenMed/OpenMed-PII-ClinicalE5-Small-33M-v1` (`2ea9471`) | medscale-pii-synthetic-en | 0.900 | 0.662 | 0.907 (word_average) | 21.5 | 28.7 | 6517 | 806 | 572 | 271 |
| `OpenMed/OpenMed-PII-ClinicalLongformer-Base-149M-v1` (`ade4412`) | medscale-pii-synthetic-en | 0.908 | 0.448 | 0.923 (word_first_pii_merge) | 769.9 | 97.9 | 7127 | 3355 | 910 | 1151 |
| `OpenMed/OpenMed-PII-FastClinical-Small-82M-v1` (`85d5bba`) | medscale-pii-synthetic-en | 0.916 | 0.495 | 0.951 (word_average) | 23.4 | 27.7 | 5076 | 1194 | 688 | 661 |
| `OpenMed/OpenMed-PII-LiteClinical-Small-66M-v1` (`a6c3e20`) | medscale-pii-synthetic-en | 0.796 | 0.604 | 0.859 (word_first_pii_merge) | 27.2 | 38.0 | 5276 | 986 | 662 | 525 |
| `OpenMed/OpenMed-PII-SuperClinical-Base-184M-v1` (`11c6dfe`) | medscale-pii-synthetic-en | 0.775 | 0.642 | 0.908 (word_first_pii_merge) | 132.3 | 177.6 | 5690 | 2832 | 967 | 1527 |
| `OpenMed/OpenMed-PII-SuperClinical-Small-44M-v1` (`a2360d3`) | medscale-pii-synthetic-en | 0.892 | 0.816 | 0.966 (word_first) | 77.1 | 108.1 | 7197 | 3365 | 804 | 1203 |
| `OpenMed/OpenMed-PII-SuperMedical-Base-125M-v1` (`5264736`) | medscale-pii-synthetic-en | 0.851 | 0.429 | 0.923 (word_first_pii_merge) | 56.3 | 100.6 | 6428 | 2866 | 851 | 987 |
| `OpenMed/OpenMed-PII-mLiteClinical-Base-135M-v1` (`3f12cf0`) | medscale-pii-synthetic-en | 0.678 | 0.467 | 0.868 (word_first_pii_merge) | 31.2 | 48.6 | 6254 | 3055 | 691 | 1072 |
