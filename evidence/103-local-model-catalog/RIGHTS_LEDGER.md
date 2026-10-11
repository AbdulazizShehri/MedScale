# Spec 103 rights ledger: executed OpenMed models

**Date:** 2026-10-09, addendum 2026-10-11. **Scope:** the models `EXECUTED_TESTED` in `specs/103-local-model-catalog/research.md` §9–10, plus (addendum) the rows in `EXECUTED_MODELS` (`crates/medscale-pack/src/compatibility.rs`) and the GLiNER export qualified for Spec 104.
**Method:** each model card `README.md` read at the pinned Hub commit (text only; no weights), the base-model card, and the declared upstream base-model licenses from Hub metadata. This is a recorded review, not legal advice. It supports local qualification use; it is **not** a redistribution clearance.

| Executed model (pinned commit) | Card SHA-256 (prefix) | Card license | Derived from | Base license (Hub) | Training data named on card |
|---|---|---|---|---|---|
| `OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-33M-v1-onnx-android` @ `54c9cc11` | `344c3a3fd0a4dd5c` | apache-2.0 | `OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-33M` @ `05659fbd` (apache-2.0) ← `intfloat/e5-small-v2` @ `ffb93f3b` | MIT | BC5CDR-Disease |
| `OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M-v1-onnx-android` @ `ad5e79cb` | `e43ec69329f881a3` | apache-2.0 | `OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M` @ `1ab9359d` (apache-2.0) ← `distilbert/distilbert-base-cased` @ `6ea81172` | Apache-2.0 | ANATOMY |
| `OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-82M-v1-onnx-android` @ `95f0bf74` | `e75bf32ef72931da` | apache-2.0 | `OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-82M` @ `724a3702` (apache-2.0) ← `distilbert/distilroberta-base` @ `fb53ab88` | Apache-2.0 | ANATOMY |
| `OpenMed/OpenMed-NER-AnatomyDetect-ModernClinical-149M-v1-onnx-android` @ `66525717` | `7a63e35a01291fe3` | apache-2.0 | `OpenMed/OpenMed-NER-AnatomyDetect-ModernClinical-149M` @ `3c0bdcc3` (apache-2.0) ← `thomas-sounack/BioClinical-ModernBERT-base` @ `c3648aa8` | MIT | ANATOMY |

All repositories were non-gated at review time.

## Addendum (2026-10-11): models executed after #191

Same method. The safetensors and MLX variants of the four models above (`OpenMed-NER-AnatomyDetect-ElectraMed-33M`, `-TinyMed-65M`, `-TinyMed-82M`, `-ModernClinical-149M`, and their `-mlx` exports) are the same OpenMed bases already listed. The new lineages are:

| Executed model (pinned commit) | Card SHA-256 (prefix) | Card license | Derived from | Base license (Hub) | Training data named on card |
|---|---|---|---|---|---|
| `OpenMed/OpenMed-PII-Dutch-SuperClinical-Small-44M-v1-onnx-android` @ `e41c8009` | `1b9bde4b087a68b2` | apache-2.0 | `OpenMed/OpenMed-PII-Dutch-SuperClinical-Small-44M-v1` @ `ee9cafad` (apache-2.0; card `d869ac67df6b8719`) ← `microsoft/deberta-v3-small` @ `a36c7390` | MIT | `ai4privacy/pii-masking-400k` (Hub license `other`) |
| `OpenMed/OpenMed-PII-Spanish-BigMed-Large-278M-v1-onnx-android` @ `da3fec60` | `ab9eecda0645b3b5` | apache-2.0 | `OpenMed/OpenMed-PII-Spanish-BigMed-Large-278M-v1` @ `2ca9bef4` (apache-2.0; card `07b9e6b0a93b38fa`) ← `FacebookAI/xlm-roberta-base` @ `e73636d4` | MIT | `ai4privacy/pii-masking-400k` (Hub license `other`) |
| `OpenMed/OpenMed-NER-AnatomyDetect-SuperClinical-141M-mlx` @ `8d9d9946` | `2e547c92300469e2` | apache-2.0 | `OpenMed/OpenMed-NER-AnatomyDetect-SuperClinical-141M` @ `3539a0dd` (apache-2.0; card `15f3929e59059f48`, "Base Model: deberta-v3-small") ← `microsoft/deberta-v3-small` @ `a36c7390` | MIT | ANATOMY |
| `OpenMed/OpenMed-ZeroShot-NER-Anatomy-Small-166M-mlx` @ `7762cb19` | `cefe3f9a9d312c17` | apache-2.0 | `OpenMed/OpenMed-ZeroShot-NER-Anatomy-Small-166M` @ `819e08e6` (apache-2.0; card `639e95b3ad9b72f3`, "Base Model: gliner_small-v2.1") ← `urchade/gliner_small-v2.1` @ `4e091416` | Apache-2.0 | ANATOMY; the GLiNER base names `urchade/pile-mistral-v0.1` |

New residuals:
- **`ai4privacy/pii-masking-400k`** (`414d0a3b`) declares license `other` on the Hub: custom terms. The terms text was **not reviewed** here. The two PII models are recorded as `DATASET_TERMS_UNVERIFIED (custom license)`.
- **`urchade/pile-mistral-v0.1`** (training data of the GLiNER base): terms **not verified**.

## Findings

1. **Weights and base models.** Every model, its OpenMed base and the upstream base model declare a permissive license (Apache-2.0 or MIT), including the models in the addendum. No gated repository appears in the weight lineage. MIT and Apache-2.0 both require notice preservation on redistribution.
2. **Training data (residual).** The cards name BC5CDR-Disease, ANATOMY and (addendum) `ai4privacy/pii-masking-400k`, whose Hub license is `other`. The GLiNER base names `urchade/pile-mistral-v0.1`. None of these dataset terms were **verified** in this review. A model trained on a dataset may carry dataset-terms questions for some uses even when the weights are permissively licensed. Recorded as `DATASET_TERMS_UNVERIFIED`.
3. **Use in MedScale.** Local, synthetic-only qualification is consistent with the declared licenses. **Not cleared:** bundling or redistributing these weights in a MedScale release, any clinical use, or any claim about the training data.

`CLINICAL_VALIDATION_STATUS=NOT_PERFORMED`. No model is `TASK_QUALIFIED`.
