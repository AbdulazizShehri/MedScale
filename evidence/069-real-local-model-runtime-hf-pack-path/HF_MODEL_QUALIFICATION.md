# Spec 069 Hugging Face Model Qualification

## Pinned source

```text
repository = onnx-community/bert-base-NER-ONNX
revision = 9faa2f4a2d59b396888b318f596ff719cc893f1e
file = onnx/model_int8.onnx
license = MIT
source_sha256 = b324e829f1fad3b897f926d1a1d1372803c6d04546831a3a2ed103652b916adf
tokenizer_sha256 = 343989712a36cd8b253efeaf8baf6a08b9d2583f78e395e83832e8ee9f8d8ee1
```

The model weights are external qualification input and are **not vendored into Git**.

## Deterministic transform

The portable tract baseline requires static shape metadata. Using `onnx==1.19.0`, the qualification transform replaces graph-wide symbolic dimensions with `batch=1` and `sequence=128`. It changes shape metadata only, not model weights.

```text
batch_size replacements = 637
sequence_length replacements = 680
derived_sha256 = 183057ca3123d7bd09fda062738390b326b1298c58d0eee39f0003f5a8920f14
```

The derived model, tokenizer, labels, and provenance are signed into a synthetic qualification Pack. The Pack metadata preserves the original source digest and exact upstream revision.

## Execution proof

The ignored external qualification test must be run only when the pinned external Pack is present. It admitted the signed Pack, prepared the tract runtime, executed locally, and produced semantic NER evidence including `John -> B-PER` and `London -> B-LOC`.

The result is evidence/proposal-only and does not authorize a `ClinicalAssertion`.

## Performance honesty

The external qualification was repeated in Rust `--release` profile on the current Apple M2 development Mac after the final review hardening: bounded manifest parsing, verified-byte model/tokenizer loading, pre-execution static output-shape validation, and runtime-contract-aware prepared-cache reuse. The final pre-PR candidate observation was:

```text
prepare = 738.110 ms
warm p50 = 559.899 ms
warm p95 = 596.100 ms
30 warm samples
```

A fresh end-to-end reproduction then downloaded the same pinned Hugging Face revision again, verified source SHA-256 `b324e829f1fad3b897f926d1a1d1372803c6d04546831a3a2ed103652b916adf` and tokenizer SHA-256 `343989712a36cd8b253efeaf8baf6a08b9d2583f78e395e83832e8ee9f8d8ee1`, ran the transform with `onnx==1.19.0`, reproduced 637/680 symbolic-dimension replacements and derived SHA-256 `183057ca3123d7bd09fda062738390b326b1298c58d0eee39f0003f5a8920f14`, and rebuilt the signed Pack with content digest `805168abca14c2131c26ed20c5b46a46e8ccc759f1f78264b139f0aa86c8f93d` and the deterministic synthetic qualification signature. That fresh reproduction established deterministic source-to-Pack bytes before the final review hardening. The exact same derived model/tokenizer hashes were then executed successfully through the final review-hardened runtime above. The fresh-reproduction observation itself was:

```text
prepare = 747.781 ms
warm p50 = 587.110 ms
warm p95 = 606.937 ms
30 warm samples
```

Earlier release-profile observations before the artifact-byte/load hardening were in the same range (`warm p50` approximately 562–572 ms and `warm p95` approximately 583–602 ms). The observations are treated as a roughly 0.6 s portable CPU warm baseline, not a latency guarantee.

These measurements establish a **portable CPU baseline only**. They are not a claim that tract is the preferred interactive runtime, not an OpenMed comparison, and not cross-platform release-performance qualification. Spec 071 owns accelerated-runtime comparison (including Apple Silicon candidates) before any runtime-winner claim.

For traceability, the earlier unoptimized test-profile observation was approximately 6.29 s preparation, 4.07 s warm p50, and 4.14 s warm p95. It is retained only as development evidence and is not used as product performance evidence.
