# Spec 069 Hugging Face Qualification Reproduction

This workflow is a developer/release-qualification procedure. It is **not** product online acquisition and does not weaken the `HF_ONLINE_PACK_DISTRIBUTION` gate.

## 1. Acquire the pinned external files

Acquire from the exact Hugging Face revision:

```text
repository = onnx-community/bert-base-NER-ONNX
revision = 9faa2f4a2d59b396888b318f596ff719cc893f1e
model file = onnx/model_int8.onnx
tokenizer file = tokenizer.json
```

Before transformation, verify:

```text
model SHA-256 = b324e829f1fad3b897f926d1a1d1372803c6d04546831a3a2ed103652b916adf
tokenizer SHA-256 = 343989712a36cd8b253efeaf8baf6a08b9d2583f78e395e83832e8ee9f8d8ee1
license = MIT
```

## 2. Specialize shape metadata deterministically

Use Python with exactly `onnx==1.19.0`:

```bash
python -m venv .tmp-onnx-069
.tmp-onnx-069/bin/pip install 'onnx==1.19.0'
.tmp-onnx-069/bin/python scripts/specialize-hf-bert-ner-069.py \
  --source-model /path/to/model_int8.onnx \
  --source-tokenizer /path/to/tokenizer.json \
  --output-dir /path/to/medscale-hf-pack
```

Expected transform evidence:

```text
batch_size replacements = 637
sequence_length replacements = 680
derived model SHA-256 = 183057ca3123d7bd09fda062738390b326b1298c58d0eee39f0003f5a8920f14
```

## 3. Build the signed synthetic qualification Pack

```bash
cargo run -p medscale-pack --example sign_hf_pack_069 -- /path/to/medscale-hf-pack
```

The signer is the repository synthetic Pack trust root used only for fixtures and qualification. It is not a production signing identity.

## 4. Execute the external qualification

```bash
MEDSCALE_HF_PACK_DIR=/path/to/medscale-hf-pack \
  cargo test --release -p medscale-pack --test hf_runtime_external_069 -- --ignored --nocapture
```

The test verifies signed Pack admission, local model preparation, semantic PERSON/LOCATION output, evidence-only authority, exact source provenance, and warm-run timing. Model weights remain outside Git.
