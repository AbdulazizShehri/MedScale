# Multilingual synthetic PII gold sets: accuracy results

Gold sets: `pii-synthetic-{nl,fr,de,es,it,pt}-v1.json` (40 synthetic sentences each, CC0, natively phrased, localized names and date formats; `scripts/evaluation/generate_pii_gold.py`). Runs: multilingual accuracy batches 0 (rerun, run 39129491295), 1 (run 39136265009) and 2 (run 39136270332). Scoring: `medscale_pack::entity_eval` (strict and overlap, one-to-one).

**Evaluation evidence on small synthetic sets; not clinical validation.**

The multilingual OpenMed PII models use the ai4privacy-style label schema (`FIRSTNAME`, `LASTNAME`, `DATEOFBIRTH`, `PHONE`, ...), mapped by the shared label map. That schema has **no medical record number label** and the models do not tag age in this phrasing, so ID and AGE recall is limited by the schema, not only by the model. The three `QwenMed-XLarge-600M` rows in batch 2 hit the 90-minute job limit (five decodings × 40 documents on a 600M decoder on CPU) and are not scored.

## Mean strict F1 by language and decoding

| Language | Rows | openmed_default | word_first | word_average | pii_merge | word_first_pii_merge |
|---|---|---|---|---|---|---|
| Dutch | 27 | 0.175 | 0.378 | 0.294 | 0.390 | 0.603 |
| French | 26 | 0.334 | 0.619 | 0.595 | 0.441 | 0.742 |
| German | 27 | 0.359 | 0.592 | 0.582 | 0.435 | 0.678 |
| Italian | 25 | 0.340 | 0.558 | 0.537 | 0.469 | 0.696 |
| Portuguese | 32 | 0.452 | 0.710 | 0.698 | 0.562 | 0.833 |
| Spanish | 26 | 0.222 | 0.382 | 0.335 | 0.319 | 0.487 |
| **all** | 163 | **0.318** | **0.545** | **0.513** | **0.440** | **0.678** |

## Findings

- The ranking matches English: `word_first_pii_merge` has the highest mean strict F1 in every language; the OpenMed-compatible decoding is lowest.
- Exports of one model agree across runtimes: in 53 of 54 groups with more than one export (ONNX on tract, safetensors and MLX on candle) the scores are identical. The exception is `Portuguese-LiteClinical-Small-66M-v1`: 0.866 on ONNX against 0.855 on safetensors and MLX, a small numerical divergence between runtimes on one model, recorded here rather than averaged away.
- Spanish is lowest: its models split `3 de mayo de 1975` at the connector `de` into three DATE entities (DATE strict: 8 TP, 28 FP, 16 FN on `Spanish-SuperClinical-Small-44M`), while the Portuguese models keep `12 de março de 1980` whole.

## Per model (strict F1)

| Model | Revision | openmed_default | word_first | word_average | pii_merge | word_first_pii_merge |
|---|---|---|---|---|---|---|
| OpenMed/OpenMed-PII-Dutch-SuperClinical-Small-44M-v1-onnx-android | `e41c8009` | 0.483 | 0.648 | 0.634 | 0.661 | 0.831 |
| OpenMed/OpenMed-PII-Dutch-SuperClinical-Small-44M-v1 | `ee9cafad` | 0.483 | 0.648 | 0.634 | 0.661 | 0.831 |
| OpenMed/OpenMed-PII-Dutch-mSuperClinical-Large-279M-v1-onnx-android | `b07d15f7` | 0.403 | 0.597 | 0.569 | 0.618 | 0.824 |
| OpenMed/OpenMed-PII-Dutch-SuperClinical-Base-184M-v1 | `9cd3176b` | 0.337 | 0.556 | 0.544 | 0.590 | 0.791 |
| OpenMed/OpenMed-PII-Dutch-SuperClinical-Base-184M-v1-onnx-android | `a7bf385f` | 0.337 | 0.556 | 0.544 | 0.590 | 0.791 |
| OpenMed/OpenMed-PII-Dutch-SuperClinical-Large-434M-v1-onnx-android | `9bbf624d` | 0.267 | 0.469 | 0.462 | 0.529 | 0.701 |
| OpenMed/OpenMed-PII-Dutch-mLiteClinical-Base-135M-v1 | `5159368e` | 0.233 | 0.408 | 0.377 | 0.461 | 0.660 |
| OpenMed/OpenMed-PII-Dutch-mLiteClinical-Base-135M-v1-onnx-android | `8fa91cec` | 0.233 | 0.408 | 0.377 | 0.461 | 0.660 |
| OpenMed/OpenMed-PII-Dutch-LiteClinical-Small-66M-v1-onnx-android | `5f068313` | 0.067 | 0.359 | 0.159 | 0.323 | 0.628 |
| OpenMed/OpenMed-PII-Dutch-LiteClinical-Small-66M-v1 | `0f6930e3` | 0.067 | 0.359 | 0.159 | 0.323 | 0.628 |
| OpenMed/OpenMed-PII-Dutch-ClinicalLongformer-Base-149M-v1 | `a1237b36` | 0.215 | 0.457 | 0.284 | 0.349 | 0.626 |
| OpenMed/OpenMed-PII-Dutch-SuperMedical-Base-125M-v1 | `8920d465` | 0.198 | 0.393 | 0.374 | 0.389 | 0.610 |
| OpenMed/OpenMed-PII-Dutch-SuperMedical-Base-125M-v1-onnx-android | `b33e9db7` | 0.198 | 0.393 | 0.374 | 0.389 | 0.610 |
| OpenMed/OpenMed-PII-Dutch-BigMed-Large-278M-v1-onnx-android | `5811d0ca` | 0.216 | 0.409 | 0.394 | 0.413 | 0.610 |
| OpenMed/OpenMed-PII-Dutch-FastClinical-Small-82M-v1 | `cfb4de5f` | 0.189 | 0.389 | 0.308 | 0.356 | 0.586 |
| OpenMed/OpenMed-PII-Dutch-FastClinical-Small-82M-v1-onnx-android | `e3179696` | 0.189 | 0.389 | 0.308 | 0.356 | 0.586 |
| OpenMed/OpenMed-PII-Dutch-QwenMed-XLarge-600M-v1-onnx-android | `b23b67ba` | 0.107 | 0.367 | 0.359 | 0.283 | 0.575 |
| OpenMed/OpenMed-PII-Dutch-BiomedBERT-Large-340M-v1-onnx-android | `435e8d2f` | 0.071 | 0.342 | 0.296 | 0.258 | 0.574 |
| OpenMed/OpenMed-PII-Dutch-EuroMed-Large-210M-v1-onnx-android | `5dc33894` | 0.105 | 0.323 | 0.186 | 0.347 | 0.563 |
| OpenMed/OpenMed-PII-Dutch-BioClinicalModern-Large-395M-v1-onnx-android | `f5f172ed` | 0.090 | 0.335 | 0.119 | 0.293 | 0.528 |
| OpenMed/OpenMed-PII-Dutch-SuperMedical-Large-355M-v1-onnx-android | `19b9df02` | 0.086 | 0.291 | 0.222 | 0.283 | 0.515 |
| OpenMed/OpenMed-PII-Dutch-BioClinicalBERT-Base-110M-v1 | `973bfed6` | 0.068 | 0.251 | 0.092 | 0.322 | 0.508 |
| OpenMed/OpenMed-PII-Dutch-BioClinicalBERT-Base-110M-v1-onnx-android | `a78102a6` | 0.068 | 0.251 | 0.092 | 0.322 | 0.508 |
| OpenMed/OpenMed-PII-Dutch-BioClinicalModern-Base-149M-v1 | `d21ec33f` | 0.010 | 0.208 | 0.037 | 0.246 | 0.443 |
| OpenMed/OpenMed-PII-Dutch-BioClinicalModern-Base-149M-v1-onnx-android | `583fa3a7` | 0.010 | 0.208 | 0.037 | 0.246 | 0.443 |
| OpenMed/OpenMed-PII-Dutch-ClinicalE5-Small-33M-v1 | `ee25778d` | 0.000 | 0.097 | 0.000 | 0.225 | 0.328 |
| OpenMed/OpenMed-PII-Dutch-ClinicalE5-Small-33M-v1-onnx-android | `27ff9e2c` | 0.000 | 0.097 | 0.000 | 0.225 | 0.328 |
| OpenMed/OpenMed-PII-French-mSuperClinical-Large-279M-v1-onnx-android | `612d6a11` | 0.456 | 0.844 | 0.844 | 0.536 | 0.935 |
| OpenMed/OpenMed-PII-French-SuperClinical-Large-434M-v1-onnx-android | `8d2db771` | 0.551 | 0.812 | 0.727 | 0.660 | 0.875 |
| OpenMed/OpenMed-PII-French-SuperClinical-Small-44M-v1-onnx-android | `de66f964` | 0.552 | 0.743 | 0.728 | 0.674 | 0.875 |
| OpenMed/OpenMed-PII-French-SuperClinical-Small-44M-v1 | `cc0bb568` | 0.552 | 0.743 | 0.728 | 0.674 | 0.875 |
| OpenMed/OpenMed-PII-French-mLiteClinical-Base-135M-v1 | `515f4e55` | 0.521 | 0.643 | 0.643 | 0.716 | 0.853 |
| OpenMed/OpenMed-PII-French-mLiteClinical-Base-135M-v1-onnx-android | `5fcf2676` | 0.521 | 0.643 | 0.643 | 0.716 | 0.853 |
| OpenMed/OpenMed-PII-French-BigMed-Large-278M-v1-onnx-android | `d41dd08c` | 0.314 | 0.784 | 0.784 | 0.360 | 0.833 |
| OpenMed/OpenMed-PII-French-ClinicalLongformer-Base-149M-v1 | `ce00777b` | 0.219 | 0.585 | 0.612 | 0.351 | 0.791 |
| OpenMed/OpenMed-PII-French-SuperClinical-Base-184M-v1-onnx-android | `5f1dd294` | 0.441 | 0.649 | 0.649 | 0.557 | 0.788 |
| OpenMed/OpenMed-PII-French-SuperClinical-Base-184M-v1 | `7464a44a` | 0.441 | 0.649 | 0.649 | 0.557 | 0.788 |
| OpenMed/OpenMed-PII-French-BiomedBERT-Large-340M-v1-onnx-android | `442c5672` | 0.179 | 0.629 | 0.626 | 0.263 | 0.755 |
| OpenMed/OpenMed-PII-French-LiteClinical-Small-66M-v1 | `d068f2d8` | 0.455 | 0.620 | 0.607 | 0.577 | 0.747 |
| OpenMed/OpenMed-PII-French-LiteClinical-Small-66M-v1-onnx-android | `d3b631fe` | 0.455 | 0.620 | 0.607 | 0.577 | 0.747 |
| OpenMed/OpenMed-PII-French-ClinicalE5-Small-33M-v1 | `9b4b6504` | 0.424 | 0.626 | 0.610 | 0.526 | 0.744 |
| OpenMed/OpenMed-PII-French-ClinicalE5-Small-33M-v1-onnx-android | `1e212566` | 0.424 | 0.626 | 0.610 | 0.526 | 0.744 |
| OpenMed/OpenMed-PII-French-SuperMedical-Large-355M-v1-onnx-android | `ea641599` | 0.301 | 0.652 | 0.626 | 0.383 | 0.742 |
| OpenMed/OpenMed-PII-French-FastClinical-Small-82M-v1 | `04d79bfe` | 0.190 | 0.542 | 0.527 | 0.314 | 0.727 |
| OpenMed/OpenMed-PII-French-FastClinical-Small-82M-v1-onnx-android | `07af2f29` | 0.190 | 0.542 | 0.527 | 0.314 | 0.727 |
| OpenMed/OpenMed-PII-French-EuroMed-Large-210M-v1-onnx-android | `c6348a95` | 0.286 | 0.642 | 0.558 | 0.355 | 0.711 |
| OpenMed/OpenMed-PII-French-BioClinicalBERT-Base-110M-v1 | `11f77a4a` | 0.139 | 0.565 | 0.535 | 0.236 | 0.672 |
| OpenMed/OpenMed-PII-French-BioClinicalBERT-Base-110M-v1-onnx-android | `afe1521c` | 0.139 | 0.565 | 0.535 | 0.236 | 0.672 |
| OpenMed/OpenMed-PII-French-SuperMedical-Base-125M-v1-onnx-android | `79e32d47` | 0.257 | 0.599 | 0.632 | 0.305 | 0.656 |
| OpenMed/OpenMed-PII-French-SuperMedical-Base-125M-v1 | `9931d98e` | 0.257 | 0.599 | 0.632 | 0.305 | 0.656 |
| OpenMed/OpenMed-PII-French-BioClinicalModern-Large-395M-v1-onnx-android | `783ff5c2` | 0.188 | 0.484 | 0.420 | 0.278 | 0.584 |
| OpenMed/OpenMed-PII-French-BioClinicalModern-Base-149M-v1-onnx-android | `4a131943` | 0.122 | 0.341 | 0.200 | 0.237 | 0.468 |
| OpenMed/OpenMed-PII-French-BioClinicalModern-Base-149M-v1 | `561a0eb2` | 0.122 | 0.341 | 0.200 | 0.237 | 0.468 |
| OpenMed/OpenMed-PII-German-mSuperClinical-Large-279M-v1-onnx-android | `a5c1abb5` | 0.674 | 0.875 | 0.875 | 0.781 | 0.968 |
| OpenMed/OpenMed-PII-German-ClinicalLongformer-Base-149M-v1 | `7f4248dc` | 0.352 | 0.648 | 0.622 | 0.454 | 0.794 |
| OpenMed/OpenMed-PII-German-BigMed-Large-278M-v1-onnx-android | `88620316` | 0.467 | 0.683 | 0.683 | 0.587 | 0.788 |
| OpenMed/OpenMed-PII-German-BiomedBERT-Large-340M-v1-onnx-android | `b4a09628` | 0.128 | 0.618 | 0.620 | 0.190 | 0.767 |
| OpenMed/OpenMed-PII-German-LiteClinical-Small-66M-v1 | `92de5d8d` | 0.479 | 0.646 | 0.629 | 0.551 | 0.731 |
| OpenMed/OpenMed-PII-German-LiteClinical-Small-66M-v1-onnx-android | `eabb097e` | 0.479 | 0.646 | 0.629 | 0.551 | 0.731 |
| OpenMed/OpenMed-PII-German-mLiteClinical-Base-135M-v1-onnx-android | `14e374f7` | 0.535 | 0.595 | 0.595 | 0.669 | 0.727 |
| OpenMed/OpenMed-PII-German-mLiteClinical-Base-135M-v1 | `e38aed6b` | 0.535 | 0.595 | 0.595 | 0.669 | 0.727 |
| OpenMed/OpenMed-PII-German-SuperClinical-Base-184M-v1-onnx-android | `10e62736` | 0.442 | 0.591 | 0.591 | 0.556 | 0.722 |
| OpenMed/OpenMed-PII-German-SuperClinical-Base-184M-v1 | `3c3f1b0a` | 0.442 | 0.591 | 0.591 | 0.556 | 0.722 |
| OpenMed/OpenMed-PII-German-ClinicalE5-Small-33M-v1 | `e7bda7ff` | 0.500 | 0.624 | 0.614 | 0.590 | 0.714 |
| OpenMed/OpenMed-PII-German-ClinicalE5-Small-33M-v1-onnx-android | `e37d973e` | 0.500 | 0.624 | 0.614 | 0.590 | 0.714 |
| OpenMed/OpenMed-PII-German-SuperClinical-Large-434M-v1-onnx-android | `6deef138` | 0.492 | 0.647 | 0.647 | 0.505 | 0.706 |
| OpenMed/OpenMed-PII-German-SuperClinical-Small-44M-v1 | `b7145b13` | 0.425 | 0.545 | 0.545 | 0.532 | 0.660 |
| OpenMed/OpenMed-PII-German-SuperClinical-Small-44M-v1-onnx-android | `f6a0d9ab` | 0.425 | 0.545 | 0.545 | 0.532 | 0.660 |
| OpenMed/OpenMed-PII-German-BioClinicalBERT-Base-110M-v1 | `2fc7c9eb` | 0.045 | 0.588 | 0.545 | 0.093 | 0.651 |
| OpenMed/OpenMed-PII-German-BioClinicalBERT-Base-110M-v1-onnx-android | `48fa77fb` | 0.045 | 0.588 | 0.545 | 0.093 | 0.651 |
| OpenMed/OpenMed-PII-German-EuroMed-Large-210M-v1-onnx-android | `eda2df4a` | 0.350 | 0.587 | 0.571 | 0.411 | 0.643 |
| OpenMed/OpenMed-PII-German-BioClinicalModern-Large-395M-v1-onnx-android | `ea8ed480` | 0.208 | 0.560 | 0.569 | 0.262 | 0.618 |
| OpenMed/OpenMed-PII-German-BioClinicalModern-Base-149M-v1 | `43de4f2f` | 0.181 | 0.560 | 0.518 | 0.238 | 0.615 |
| OpenMed/OpenMed-PII-German-BioClinicalModern-Base-149M-v1-onnx-android | `ba5a95ed` | 0.180 | 0.560 | 0.518 | 0.237 | 0.615 |
| OpenMed/OpenMed-PII-German-SuperMedical-Base-125M-v1 | `1d5ac702` | 0.312 | 0.593 | 0.593 | 0.328 | 0.610 |
| OpenMed/OpenMed-PII-German-SuperMedical-Base-125M-v1-onnx-android | `494d3c99` | 0.312 | 0.593 | 0.593 | 0.328 | 0.610 |
| OpenMed/OpenMed-PII-German-SuperMedical-Large-355M-v1-onnx-android | `bc992a72` | 0.429 | 0.588 | 0.588 | 0.495 | 0.606 |
| OpenMed/OpenMed-PII-German-FastClinical-Small-82M-v1 | `cf8654cc` | 0.275 | 0.498 | 0.506 | 0.352 | 0.595 |
| OpenMed/OpenMed-PII-German-FastClinical-Small-82M-v1-onnx-android | `26bec9f1` | 0.275 | 0.498 | 0.506 | 0.352 | 0.595 |
| OpenMed/OpenMed-PII-German-QwenMed-XLarge-600M-v1-onnx-android | `23602509` | 0.206 | 0.310 | 0.258 | 0.252 | 0.356 |
| OpenMed/OpenMed-PII-Italian-LiteClinical-Small-66M-v1-onnx-android | `ec892ebb` | 0.558 | 0.732 | 0.660 | 0.672 | 0.845 |
| OpenMed/OpenMed-PII-Italian-LiteClinical-Small-66M-v1 | `69f8c5c1` | 0.558 | 0.732 | 0.660 | 0.672 | 0.845 |
| OpenMed/OpenMed-PII-Italian-SuperClinical-Small-44M-v1 | `6b47b973` | 0.595 | 0.686 | 0.725 | 0.727 | 0.812 |
| OpenMed/OpenMed-PII-Italian-SuperClinical-Small-44M-v1-onnx-android | `200be8fe` | 0.595 | 0.686 | 0.725 | 0.727 | 0.812 |
| OpenMed/OpenMed-PII-Italian-BioClinicalModern-Large-395M-v1-onnx-android | `8f3c2042` | 0.219 | 0.684 | 0.543 | 0.322 | 0.808 |
| OpenMed/OpenMed-PII-Italian-SuperClinical-Large-434M-v1-onnx-android | `2bfdd0f8` | 0.647 | 0.708 | 0.664 | 0.788 | 0.788 |
| OpenMed/OpenMed-PII-Italian-BigMed-Large-278M-v1-onnx-android | `b9aa7ae2` | 0.371 | 0.582 | 0.582 | 0.551 | 0.774 |
| OpenMed/OpenMed-PII-Italian-ClinicalLongformer-Base-149M-v1 | `2c664f6b` | 0.261 | 0.578 | 0.573 | 0.403 | 0.772 |
| OpenMed/OpenMed-PII-Italian-ClinicalE5-Small-33M-v1 | `4a96ad80` | 0.434 | 0.579 | 0.574 | 0.631 | 0.756 |
| OpenMed/OpenMed-PII-Italian-ClinicalE5-Small-33M-v1-onnx-android | `1f0146e3` | 0.434 | 0.579 | 0.574 | 0.631 | 0.756 |
| OpenMed/OpenMed-PII-Italian-SuperClinical-Base-184M-v1-onnx-android | `5d5f09f4` | 0.538 | 0.639 | 0.648 | 0.667 | 0.754 |
| OpenMed/OpenMed-PII-Italian-SuperClinical-Base-184M-v1 | `e0a1a417` | 0.538 | 0.639 | 0.648 | 0.667 | 0.754 |
| OpenMed/OpenMed-PII-Italian-mLiteClinical-Base-135M-v1 | `0ce2c464` | 0.500 | 0.526 | 0.526 | 0.701 | 0.722 |
| OpenMed/OpenMed-PII-Italian-SuperMedical-Large-355M-v1-onnx-android | `f528e4a0` | 0.241 | 0.584 | 0.584 | 0.360 | 0.722 |
| OpenMed/OpenMed-PII-Italian-mLiteClinical-Base-135M-v1-onnx-android | `9d6a9514` | 0.500 | 0.526 | 0.526 | 0.701 | 0.722 |
| OpenMed/OpenMed-PII-Italian-SuperMedical-Base-125M-v1 | `7129b53a` | 0.173 | 0.556 | 0.556 | 0.296 | 0.718 |
| OpenMed/OpenMed-PII-Italian-SuperMedical-Base-125M-v1-onnx-android | `f2b7b81a` | 0.173 | 0.556 | 0.556 | 0.296 | 0.718 |
| OpenMed/OpenMed-PII-Italian-FastClinical-Small-82M-v1-onnx-android | `05cef784` | 0.220 | 0.603 | 0.619 | 0.285 | 0.694 |
| OpenMed/OpenMed-PII-Italian-FastClinical-Small-82M-v1 | `60e841c6` | 0.220 | 0.603 | 0.619 | 0.285 | 0.694 |
| OpenMed/OpenMed-PII-Italian-BiomedBERT-Large-340M-v1-onnx-android | `65444cef` | 0.113 | 0.455 | 0.454 | 0.242 | 0.632 |
| OpenMed/OpenMed-PII-Italian-EuroMed-Large-210M-v1-onnx-android | `a93f28b5` | 0.260 | 0.468 | 0.430 | 0.346 | 0.566 |
| OpenMed/OpenMed-PII-Italian-BioClinicalBERT-Base-110M-v1 | `11a61abb` | 0.065 | 0.402 | 0.361 | 0.164 | 0.523 |
| OpenMed/OpenMed-PII-Italian-BioClinicalBERT-Base-110M-v1-onnx-android | `fe87e32b` | 0.065 | 0.402 | 0.361 | 0.164 | 0.523 |
| OpenMed/OpenMed-PII-Italian-BioClinicalModern-Base-149M-v1-onnx-android | `91770ca6` | 0.105 | 0.217 | 0.132 | 0.217 | 0.341 |
| OpenMed/OpenMed-PII-Italian-BioClinicalModern-Base-149M-v1 | `6909afda` | 0.105 | 0.217 | 0.132 | 0.217 | 0.341 |
| OpenMed/OpenMed-PII-Portuguese-SuperClinical-Small-44M-v1-mlx | `994f1f06` | 0.595 | 0.796 | 0.796 | 0.743 | 0.964 |
| OpenMed/OpenMed-PII-Portuguese-SuperClinical-Small-44M-v1-onnx-android | `1ae9008e` | 0.595 | 0.796 | 0.796 | 0.743 | 0.964 |
| OpenMed/OpenMed-PII-Portuguese-SuperClinical-Small-44M-v1 | `e74186eb` | 0.595 | 0.796 | 0.796 | 0.743 | 0.964 |
| OpenMed/OpenMed-PII-Portuguese-SuperMedical-Base-125M-v1-onnx-android | `4582e590` | 0.483 | 0.864 | 0.877 | 0.566 | 0.964 |
| OpenMed/OpenMed-PII-Portuguese-SuperMedical-Base-125M-v1 | `c021f988` | 0.483 | 0.864 | 0.877 | 0.566 | 0.964 |
| OpenMed/OpenMed-PII-Portuguese-SuperMedical-Base-125M-v1-mlx | `793441fc` | 0.483 | 0.864 | 0.877 | 0.566 | 0.964 |
| OpenMed/OpenMed-PII-Portuguese-FastClinical-Small-82M-v1-mlx | `e7a899a9` | 0.484 | 0.835 | 0.835 | 0.579 | 0.955 |
| OpenMed/OpenMed-PII-Portuguese-FastClinical-Small-82M-v1 | `a6243988` | 0.484 | 0.835 | 0.835 | 0.579 | 0.955 |
| OpenMed/OpenMed-PII-Portuguese-FastClinical-Small-82M-v1-onnx-android | `88d930b0` | 0.484 | 0.835 | 0.835 | 0.579 | 0.955 |
| OpenMed/OpenMed-PII-Portuguese-SuperClinical-Large-434M-v1-onnx-android | `c84a1a21` | 0.740 | 0.866 | 0.844 | 0.845 | 0.954 |
| OpenMed/OpenMed-PII-Portuguese-SuperClinical-Base-184M-v1 | `99a2e9ca` | 0.696 | 0.761 | 0.761 | 0.865 | 0.921 |
| OpenMed/OpenMed-PII-Portuguese-SuperClinical-Base-184M-v1-onnx-android | `a124aa9f` | 0.696 | 0.761 | 0.761 | 0.865 | 0.921 |
| OpenMed/OpenMed-PII-Portuguese-SuperClinical-Base-184M-v1-mlx | `78a83a82` | 0.696 | 0.761 | 0.761 | 0.865 | 0.921 |
| OpenMed/OpenMed-PII-Portuguese-BioClinicalBERT-Base-110M-v1-onnx-android | `cf72fb62` | 0.283 | 0.751 | 0.717 | 0.375 | 0.873 |
| OpenMed/OpenMed-PII-Portuguese-BioClinicalBERT-Base-110M-v1 | `3252a888` | 0.283 | 0.751 | 0.717 | 0.375 | 0.873 |
| OpenMed/OpenMed-PII-Portuguese-BioClinicalBERT-Base-110M-v1-mlx | `d8aec713` | 0.283 | 0.751 | 0.717 | 0.375 | 0.873 |
| OpenMed/OpenMed-PII-Portuguese-mLiteClinical-Base-135M-v1 | `ee0a56f5` | 0.636 | 0.718 | 0.718 | 0.794 | 0.871 |
| OpenMed/OpenMed-PII-Portuguese-mLiteClinical-Base-135M-v1-mlx | `b671bcb2` | 0.636 | 0.718 | 0.718 | 0.794 | 0.871 |
| OpenMed/OpenMed-PII-Portuguese-mLiteClinical-Base-135M-v1-onnx-android | `f8f28b3f` | 0.636 | 0.718 | 0.718 | 0.794 | 0.871 |
| OpenMed/OpenMed-PII-Portuguese-LiteClinical-Small-66M-v1-onnx-android | `d774b32f` | 0.510 | 0.731 | 0.672 | 0.647 | 0.866 |
| OpenMed/OpenMed-PII-Portuguese-SuperMedical-Large-355M-v1-onnx-android | `4b7a1cc6` | 0.361 | 0.734 | 0.734 | 0.453 | 0.866 |
| OpenMed/OpenMed-PII-Portuguese-LiteClinical-Small-66M-v1-mlx | `4cf8809c` | 0.502 | 0.721 | 0.662 | 0.637 | 0.855 |
| OpenMed/OpenMed-PII-Portuguese-LiteClinical-Small-66M-v1 | `e37c55aa` | 0.502 | 0.721 | 0.662 | 0.637 | 0.855 |
| OpenMed/OpenMed-PII-Portuguese-BiomedBERT-Large-340M-v1-onnx-android | `98ff95df` | 0.240 | 0.687 | 0.650 | 0.320 | 0.832 |
| OpenMed/OpenMed-PII-Portuguese-BigMed-Large-278M-v1-onnx-android | `725b64fc` | 0.455 | 0.681 | 0.671 | 0.606 | 0.830 |
| OpenMed/OpenMed-PII-Portuguese-ClinicalLongformer-Base-149M-v1 | `d1745e7d` | 0.224 | 0.569 | 0.545 | 0.313 | 0.707 |
| OpenMed/OpenMed-PII-Portuguese-ClinicalE5-Small-33M-v1-onnx-android | `06995205` | 0.367 | 0.512 | 0.544 | 0.432 | 0.577 |
| OpenMed/OpenMed-PII-Portuguese-ClinicalE5-Small-33M-v1-mlx | `d4786398` | 0.367 | 0.512 | 0.544 | 0.432 | 0.577 |
| OpenMed/OpenMed-PII-Portuguese-ClinicalE5-Small-33M-v1 | `01a1677d` | 0.367 | 0.512 | 0.544 | 0.432 | 0.577 |
| OpenMed/OpenMed-PII-Portuguese-BioClinicalModern-Large-395M-v1-onnx-android | `ca2761ea` | 0.087 | 0.438 | 0.373 | 0.136 | 0.508 |
| OpenMed/OpenMed-PII-Portuguese-BioClinicalModern-Base-149M-v1-onnx-android | `e6289a36` | 0.105 | 0.424 | 0.392 | 0.167 | 0.502 |
| OpenMed/OpenMed-PII-Portuguese-BioClinicalModern-Base-149M-v1 | `11566ad6` | 0.105 | 0.424 | 0.392 | 0.167 | 0.502 |
| OpenMed/OpenMed-PII-Spanish-SuperClinical-Base-184M-v1 | `b5c39baf` | 0.494 | 0.693 | 0.672 | 0.638 | 0.852 |
| OpenMed/OpenMed-PII-Spanish-SuperClinical-Base-184M-v1-onnx-android | `4354153e` | 0.494 | 0.693 | 0.672 | 0.638 | 0.852 |
| OpenMed/OpenMed-PII-Spanish-SuperClinical-Small-44M-v1-onnx-android | `fbeafaea` | 0.511 | 0.632 | 0.639 | 0.606 | 0.718 |
| OpenMed/OpenMed-PII-Spanish-SuperClinical-Small-44M-v1 | `9fb17321` | 0.511 | 0.632 | 0.639 | 0.606 | 0.718 |
| OpenMed/OpenMed-PII-Spanish-ClinicalLongformer-Base-149M-v1 | `0e85d0c3` | 0.221 | 0.494 | 0.419 | 0.333 | 0.645 |
| OpenMed/OpenMed-PII-Spanish-BigMed-Large-278M-v1-onnx-android | `da3fec60` | 0.386 | 0.545 | 0.495 | 0.491 | 0.642 |
| OpenMed/OpenMed-PII-Spanish-SuperClinical-Large-434M-v1-onnx-android | `1281b747` | 0.371 | 0.527 | 0.502 | 0.458 | 0.606 |
| OpenMed/OpenMed-PII-Spanish-BioClinicalBERT-Base-110M-v1-onnx-android | `bd16dfef` | 0.068 | 0.441 | 0.343 | 0.137 | 0.545 |
| OpenMed/OpenMed-PII-Spanish-BioClinicalBERT-Base-110M-v1 | `4ec9e7ec` | 0.068 | 0.441 | 0.343 | 0.137 | 0.545 |
| OpenMed/OpenMed-PII-Spanish-FastClinical-Small-82M-v1 | `ba243978` | 0.164 | 0.403 | 0.389 | 0.243 | 0.513 |
| OpenMed/OpenMed-PII-Spanish-FastClinical-Small-82M-v1-onnx-android | `89071ea7` | 0.164 | 0.403 | 0.389 | 0.243 | 0.513 |
| OpenMed/OpenMed-PII-Spanish-SuperMedical-Large-355M-v1-onnx-android | `361d92b8` | 0.250 | 0.370 | 0.291 | 0.375 | 0.500 |
| OpenMed/OpenMed-PII-Spanish-SuperMedical-Base-125M-v1 | `2a903c4b` | 0.217 | 0.340 | 0.253 | 0.335 | 0.469 |
| OpenMed/OpenMed-PII-Spanish-SuperMedical-Base-125M-v1-onnx-android | `a4e1d5fd` | 0.217 | 0.340 | 0.253 | 0.335 | 0.469 |
| OpenMed/OpenMed-PII-Spanish-BiomedBERT-Large-340M-v1-onnx-android | `c4dd668c` | 0.000 | 0.345 | 0.328 | 0.056 | 0.463 |
| OpenMed/OpenMed-PII-Spanish-EuroMed-Large-210M-v1-onnx-android | `ec1daddd` | 0.177 | 0.371 | 0.259 | 0.251 | 0.449 |
| OpenMed/OpenMed-PII-Spanish-LiteClinical-Small-66M-v1 | `55ecc0b3` | 0.257 | 0.328 | 0.303 | 0.371 | 0.434 |
| OpenMed/OpenMed-PII-Spanish-LiteClinical-Small-66M-v1-onnx-android | `39f3315e` | 0.258 | 0.328 | 0.303 | 0.372 | 0.434 |
| OpenMed/OpenMed-PII-Spanish-mLiteClinical-Base-135M-v1-onnx-android | `13e8006e` | 0.290 | 0.300 | 0.300 | 0.423 | 0.423 |
| OpenMed/OpenMed-PII-Spanish-mLiteClinical-Base-135M-v1 | `4e74267c` | 0.290 | 0.300 | 0.300 | 0.423 | 0.423 |
| OpenMed/OpenMed-PII-Spanish-QwenMed-XLarge-600M-v1-onnx-android | `e4c4c8aa` | 0.083 | 0.238 | 0.122 | 0.158 | 0.308 |
| OpenMed/OpenMed-PII-Spanish-BioClinicalModern-Base-149M-v1-onnx-android | `71f9730b` | 0.009 | 0.164 | 0.042 | 0.085 | 0.242 |
| OpenMed/OpenMed-PII-Spanish-BioClinicalModern-Base-149M-v1 | `c0e535b8` | 0.009 | 0.164 | 0.042 | 0.085 | 0.242 |
| OpenMed/OpenMed-PII-Spanish-BioClinicalModern-Large-395M-v1-onnx-android | `4fe297bc` | 0.006 | 0.165 | 0.132 | 0.057 | 0.222 |
| OpenMed/OpenMed-PII-Spanish-ClinicalE5-Small-33M-v1 | `82d8692d` | 0.132 | 0.132 | 0.140 | 0.220 | 0.220 |
| OpenMed/OpenMed-PII-Spanish-ClinicalE5-Small-33M-v1-onnx-android | `0d0332e7` | 0.132 | 0.132 | 0.140 | 0.220 | 0.220 |
