# Spec 071 Runtime Comparison

## MedScale observed baseline

Spec 069 executed the pinned `onnx-community/bert-base-NER-ONNX` artifact through `tract-onnx 0.22.4` on the local CPU path. Final recorded warm observation: p50 `559.899 ms`, p95 `596.100 ms` over 30 warm samples, with prepare time `738.110 ms`.

## OpenMed pinned feature evidence

OpenMed v2.2.0 documents Python MLX and Swift MLX paths on Apple Silicon, with shared artifact contracts for supported token-classification families. The pinned source proves feature presence; it does not by itself supply a matched benchmark against the MedScale Pack/model/corpus/hardware observation.

## Verdict

`NO_RUNTIME_WINNER`. There is no complete BenchmarkManifest with the same model, corpus, hardware, OS, measurement protocol, privacy observation, artifact hashes and failure behavior across both products. MedScale therefore keeps tract as the qualified portable CPU baseline and does not promote MLX, ORT, Core ML, or another accelerated runtime in Spec 071.
