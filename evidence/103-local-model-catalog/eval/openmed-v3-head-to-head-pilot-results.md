# OpenMed v3.0.0 head-to-head: pilot results (synthetic)

**Run:** `model-qualification-batch` run `39160798189` on branch `qual-batch/103-h2h-01`, 5 rows, `ubuntu-latest`. Each row ran on a single runner:
1. MedScale ran the catalog-pinned snapshot.
2. Then pinned OpenMed (PyPI `openmed==3.0.0`, `torch==2.14.1+cpu`, `transformers==5.20.1`, `analyze_text` defaults) ran the same files.

Both sides were scored by the same MedScale scorer (`entity_eval`) on the same synthetic gold sets.

**Not claimed:** this is evaluation evidence on small synthetic gold sets (40 documents each). It is not clinical validation, not a general superiority claim, and too small to settle a ranking.

| Model (revision) | Gold set | OpenMed 3.0.0 strict F1 | MedScale default strict F1 | MedScale best strict F1 (decoding) | OpenMed p50 ms | MedScale p50 ms | OpenMed cold start ms | MedScale prepare ms | OpenMed peak RSS MiB | MedScale peak RSS MiB |
|---|---|---|---|---|---|---|---|---|---|---|
| `OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-135M` (`e26dc4b`) | medscale-ner-anatomy-synthetic-en | 0.833 | 0.400 | 0.833 (word_first) | 8.4 | 24.4 | 4720 | 1023 | 625 | 831 |
| `OpenMed/OpenMed-NER-PharmaDetect-TinyMed-135M` (`954b76a`) | medscale-ner-chemical-synthetic-en | 0.979 | 0.000 | 0.979 (word_first) | 7.6 | 21.4 | 4127 | 911 | 624 | 832 |
| `OpenMed/OpenMed-PII-SuperClinical-Base-184M-v1` (`11c6dfe`) | medscale-pii-synthetic-en | 0.775 | 0.642 | 0.908 (word_first_pii_merge) | 132.3 | 177.6 | 5690 | 2832 | 967 | 1527 |
| `OpenMed/OpenMed-PII-SuperMedical-Base-125M-v1` (`5264736`) | medscale-pii-synthetic-en | 0.851 | 0.429 | 0.923 (word_first_pii_merge) | 56.3 | 100.6 | 6428 | 2866 | 851 | 987 |
| `OpenMed/OpenMed-PII-mLiteClinical-Base-135M-v1` (`3f12cf0`) | medscale-pii-synthetic-en | 0.678 | 0.467 | 0.868 (word_first_pii_merge) | 31.2 | 48.6 | 6254 | 3055 | 691 | 1072 |
## Reading

**Accuracy**
- With MedScale's optional word-level decoding plus the PII merger, MedScale scored higher strict F1 than OpenMed's own pipeline on all three PII models (+0.07 to +0.19).
- It tied OpenMed on both NER models (word-first decoding).

**MedScale's default decoding is a loss**
- `openmed_compatible` ports OpenMed's ONNX token decoder.
- It scored well below OpenMed's Python pipeline, which uses Hugging Face `simple` aggregation and sentence detection.
- So "OpenMed-compatible" matches OpenMed's ONNX decoder, not its Python default. The next step is to choose the product default from this evidence.

**Warm latency is a loss**
- Per-document p50 was 1.3–2.9× slower than OpenMed on PyTorch CPU.
- MedScale's candle path is the target for optimisation.

**Cold start is a win**
- MedScale prepares in 0.9–3.1 s; OpenMed's first call takes 4.1–6.4 s.
- The two numbers measure slightly different things:
  - OpenMed's figure is model load plus one document.
  - MedScale's figure is model preparation only (one warm document adds about 20–180 ms).

**Peak RSS is higher for MedScale (a loss as measured)**
- MedScale's process also builds and admits the signed Pack and runs five decodings in the same process.
- So its figure is an upper bound, not an inference-only measurement.

## Reproduce

Push a `qual-batch/**` branch whose `qualification/batch.json` rows carry `"openmed": "true"` and an `eval` gold set (safetensors rows only; OpenMed's PyTorch path cannot load ONNX-only snapshots).
