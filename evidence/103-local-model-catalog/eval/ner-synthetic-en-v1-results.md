# Synthetic biomedical NER gold sets: accuracy results

Gold sets: `ner-{disease,chemical,anatomy}-synthetic-en-v1.json` (40 synthetic English sentences each, CC0, single type, distractor mentions of the other types; `scripts/evaluation/generate_ner_gold.py`). Runs: NER accuracy batch 0 (run 39138333094) for DiseaseDetect and PharmaDetect, batch 1 (run 39145303218) for AnatomyDetect after the anatomy convention was aligned with the models (laterality included; see the generator docstring). 21 models per family, sampled across sizes and runtimes.

**Evaluation evidence on small synthetic sets; not clinical validation.**

## Mean strict / overlap F1 by family and decoding

| Family | Models | openmed_default | word_first | word_average | word_first_pii_merge |
|---|---|---|---|---|---|
| AnatomyDetect | 21 | 0.803 / 0.830 | 0.847 / 0.866 | 0.846 / 0.865 | 0.847 / 0.866 |
| DiseaseDetect | 21 | 0.500 / 0.655 | 0.756 / 0.780 | 0.756 / 0.780 | 0.756 / 0.780 |
| PharmaDetect | 21 | 0.290 / 0.584 | 0.992 / 0.992 | 0.992 / 0.992 | 0.992 / 0.992 |

## Per model (strict F1)

| Model | Revision | openmed_default | word_first | word_average | word_first_pii_merge |
|---|---|---|---|---|---|
| OpenMed/OpenMed-NER-AnatomyDetect-BioPatient-108M-mlx | `a20addf2` | 0.889 | 0.909 | 0.909 | 0.909 |
| OpenMed/OpenMed-NER-AnatomyDetect-PubMed-109M | `10d8d3a7` | 0.889 | 0.889 | 0.889 | 0.889 |
| OpenMed/OpenMed-NER-AnatomyDetect-SuperClinical-184M-mlx | `b576ce4d` | 0.889 | 0.889 | 0.889 | 0.889 |
| OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-33M-v1-onnx-android | `1736f530` | 0.851 | 0.889 | 0.851 | 0.889 |
| OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M | `1ab9359d` | 0.851 | 0.889 | 0.889 | 0.889 |
| OpenMed/OpenMed-NER-AnatomyDetect-BioClinical-108M | `f6445838` | 0.833 | 0.870 | 0.889 | 0.870 |
| OpenMed/OpenMed-NER-AnatomyDetect-SuperClinical-141M-mlx | `8d9d9946` | 0.870 | 0.870 | 0.870 | 0.870 |
| OpenMed/OpenMed-NER-AnatomyDetect-PubMed-v2-109M-mlx | `948c10f3` | 0.870 | 0.870 | 0.870 | 0.870 |
| OpenMed/OpenMed-NER-AnatomyDetect-PubMed-335M | `34e40f30` | 0.870 | 0.870 | 0.870 | 0.870 |
| OpenMed/OpenMed-NER-AnatomyDetect-BioMed-109M | `49bedb87` | 0.870 | 0.870 | 0.870 | 0.870 |
| OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-82M | `724a3702` | 0.769 | 0.870 | 0.870 | 0.870 |
| OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-109M-mlx | `b4f14991` | 0.800 | 0.851 | 0.851 | 0.851 |
| OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-335M-v1-onnx-android | `edddd939` | 0.800 | 0.851 | 0.851 | 0.851 |
| OpenMed/OpenMed-NER-AnatomyDetect-BioMed-335M | `a227a43c` | 0.851 | 0.851 | 0.851 | 0.851 |
| OpenMed/OpenMed-NER-AnatomyDetect-SuperClinical-434M-v1-onnx-android | `6597a66a` | 0.851 | 0.851 | 0.851 | 0.851 |
| OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-66M | `bdb016bf` | 0.784 | 0.851 | 0.851 | 0.851 |
| OpenMed/OpenMed-NER-AnatomyDetect-ModernMed-149M | `147280de` | 0.844 | 0.844 | 0.844 | 0.844 |
| OpenMed/OpenMed-NER-AnatomyDetect-ModernMed-395M-v1-onnx-android | `2433e278` | 0.833 | 0.833 | 0.833 | 0.833 |
| OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-135M | `e26dc4bb` | 0.400 | 0.833 | 0.833 | 0.833 |
| OpenMed/OpenMed-NER-AnatomyDetect-SuperMedical-125M-v1-onnx-android | `e6fa5868` | 0.741 | 0.833 | 0.833 | 0.833 |
| OpenMed/OpenMed-NER-AnatomyDetect-ModernClinical-149M-v1-onnx-android | `66525717` | 0.511 | 0.511 | 0.511 | 0.511 |
| OpenMed/OpenMed-NER-DiseaseDetect-BioClinical-108M | `f25fd4b7` | 0.179 | 0.824 | 0.824 | 0.824 |
| OpenMed/OpenMed-NER-DiseaseDetect-BioClinical-108M-v1-onnx-android | `b4ef0b1b` | 0.179 | 0.824 | 0.824 | 0.824 |
| OpenMed/OpenMed-NER-DiseaseDetect-BioMed-335M | `a62e2a2d` | 0.824 | 0.824 | 0.824 | 0.824 |
| OpenMed/OpenMed-NER-DiseaseDetect-PubMed-335M | `f7564900` | 0.824 | 0.824 | 0.824 | 0.824 |
| OpenMed/OpenMed-NER-DiseaseDetect-PubMed-v2-109M | `114b383b` | 0.824 | 0.824 | 0.824 | 0.824 |
| OpenMed/OpenMed-NER-DiseaseDetect-PubMed-v2-109M-onnx-android | `6e8fbd36` | 0.824 | 0.824 | 0.824 | 0.824 |
| OpenMed/OpenMed-NER-DiseaseDetect-SuperClinical-184M | `1d72b9a9` | 0.824 | 0.824 | 0.824 | 0.824 |
| OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-109M-mlx | `811f16c0` | 0.265 | 0.812 | 0.812 | 0.812 |
| OpenMed/OpenMed-NER-DiseaseDetect-BioMed-109M-mlx | `ae5ca73d` | 0.812 | 0.812 | 0.812 | 0.812 |
| OpenMed/OpenMed-NER-DiseaseDetect-BioPatient-108M-v1-onnx-android | `2f9bfa5c` | 0.175 | 0.800 | 0.800 | 0.800 |
| OpenMed/OpenMed-NER-DiseaseDetect-BioPatient-108M | `3c6d265a` | 0.175 | 0.800 | 0.800 | 0.800 |
| OpenMed/OpenMed-NER-DiseaseDetect-MultiMed-335M-v1-onnx-android | `c2d29278` | 0.260 | 0.789 | 0.789 | 0.789 |
| OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-335M | `83c597b1` | 0.245 | 0.783 | 0.783 | 0.783 |
| OpenMed/OpenMed-NER-DiseaseDetect-SuperClinical-141M-mlx | `7d255023` | 0.778 | 0.778 | 0.778 | 0.778 |
| OpenMed/OpenMed-NER-DiseaseDetect-ModernClinical-149M-v1-onnx-android | `a26333c4` | 0.641 | 0.771 | 0.771 | 0.771 |
| OpenMed/OpenMed-NER-DiseaseDetect-PubMed-109M-mlx | `a8e13027` | 0.747 | 0.747 | 0.747 | 0.747 |
| OpenMed/OpenMed-NER-DiseaseDetect-ModernClinical-395M-v1-onnx-android | `1aee5e93` | 0.585 | 0.703 | 0.703 | 0.703 |
| OpenMed/OpenMed-NER-DiseaseDetect-ModernMed-395M-v1-onnx-android | `3dc6f62c` | 0.517 | 0.617 | 0.617 | 0.617 |
| OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-33M-v1-onnx-android | `54c9cc11` | 0.214 | 0.602 | 0.602 | 0.602 |
| OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-33M | `05659fbd` | 0.214 | 0.602 | 0.602 | 0.602 |
| OpenMed/OpenMed-NER-DiseaseDetect-ModernMed-149M-v1-onnx-android | `916ce444` | 0.392 | 0.494 | 0.494 | 0.494 |
| OpenMed/OpenMed-NER-PharmaDetect-BioClinical-108M | `c0a8d5eb` | 0.016 | 1.000 | 1.000 | 1.000 |
| OpenMed/OpenMed-NER-PharmaDetect-BioPatient-108M-mlx | `93e7b248` | 0.016 | 1.000 | 1.000 | 1.000 |
| OpenMed/OpenMed-NER-PharmaDetect-ElectraMed-33M-v1-onnx-android | `5650513f` | 0.018 | 1.000 | 1.000 | 1.000 |
| OpenMed/OpenMed-NER-PharmaDetect-ElectraMed-335M-v1-onnx-android | `410b701f` | 0.018 | 1.000 | 1.000 | 1.000 |
| OpenMed/OpenMed-NER-PharmaDetect-ModernClinical-149M-v1-onnx-android | `57d4406e` | 0.081 | 1.000 | 1.000 | 1.000 |
| OpenMed/OpenMed-NER-PharmaDetect-ModernMed-149M | `889deecf` | 0.081 | 1.000 | 1.000 | 1.000 |
| OpenMed/OpenMed-NER-PharmaDetect-SuperClinical-141M-mlx | `64f1c1ae` | 0.920 | 1.000 | 1.000 | 1.000 |
| OpenMed/OpenMed-NER-PharmaDetect-PubMed-109M | `f272908d` | 0.603 | 1.000 | 1.000 | 1.000 |
| OpenMed/OpenMed-NER-PharmaDetect-PubMed-v2-109M-mlx | `75f83920` | 0.603 | 1.000 | 1.000 | 1.000 |
| OpenMed/OpenMed-NER-PharmaDetect-SuperClinical-184M-mlx | `8962b62e` | 0.920 | 1.000 | 1.000 | 1.000 |
| OpenMed/OpenMed-NER-PharmaDetect-SuperClinical-434M-v1-onnx-android | `c4184e3f` | 0.920 | 1.000 | 1.000 | 1.000 |
| OpenMed/OpenMed-NER-PharmaDetect-SuperMedical-125M-v1-onnx-android | `92f93769` | 0.038 | 1.000 | 1.000 | 1.000 |
| OpenMed/OpenMed-NER-PharmaDetect-TinyMed-82M | `9e706a03` | 0.038 | 1.000 | 1.000 | 1.000 |
| OpenMed/OpenMed-NER-PharmaDetect-ModernMed-395M-v1-onnx-android | `14cfad6d` | 0.078 | 0.980 | 0.980 | 0.980 |
| OpenMed/OpenMed-NER-PharmaDetect-BioMed-109M | `8e510d68` | 0.581 | 0.979 | 0.979 | 0.979 |
| OpenMed/OpenMed-NER-PharmaDetect-BioMed-335M | `e2b5518c` | 0.581 | 0.979 | 0.979 | 0.979 |
| OpenMed/OpenMed-NER-PharmaDetect-ElectraMed-109M-mlx | `62d7fe28` | 0.000 | 0.979 | 0.979 | 0.979 |
| OpenMed/OpenMed-NER-PharmaDetect-PubMed-335M | `93b6b35f` | 0.581 | 0.979 | 0.979 | 0.979 |
| OpenMed/OpenMed-NER-PharmaDetect-TinyMed-135M | `954b76a0` | 0.000 | 0.979 | 0.979 | 0.979 |
| OpenMed/OpenMed-NER-PharmaDetect-TinyMed-65M | `fc9dabae` | 0.000 | 0.979 | 0.979 | 0.979 |
| OpenMed/OpenMed-NER-PharmaDetect-TinyMed-66M | `ccdd774c` | 0.000 | 0.979 | 0.979 | 0.979 |
