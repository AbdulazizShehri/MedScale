# Candle f32 GELU latency work (2026-10-19)

Spec 103 latency follow-up to the OpenMed v3.0.0 head-to-head
([openmed-v3-head-to-head-results.md](openmed-v3-head-to-head-results.md)).
In that run, MedScale's warm per-document latency was a median 1.19x OpenMed's, and slower on 14 of 21 rows.

## Where the time goes

`crates/medscale-pack/examples/candle_latency_profile.rs layer 16` times
each op class of a BERT-base encoder (candle ops, same as candle-transformers'
`BertLayer`, 12 layers, 16 tokens). Local Windows run, 12 threads:

| Op class | Share before the patch |
|---|---|
| Linear (FFN) | 49.6% |
| Linear (Q/K/V) | 22.8% |
| GELU (erf form) | 13.1% |
| Linear (attention output) | 7.5% |
| Add + LayerNorm | 3.1% |
| Softmax, attention matmuls, reshapes | 3.9% |

- **GELU:** candle-core 0.9.1 computes f32 GELU through the scalar f64 statrs `erf` (26.3 ns per element) or libm `tanhf` (10.7 ns per element). Neither vectorizes. DistilBERT in candle-transformers uses the tanh form.
- **Matrix multiplications** dominate the rest. The gemm kernel reaches 70–190 GFLOP/s on these skinny shapes.

## Change

The vendored candle-core (`vendor/candle-core-0.9.1`, item 5 in `MEDSCALE_PATCH.md`) evaluates only the f32 CPU paths of `gelu_erf` and `gelu` with branch-free single-precision rational `erf` and `tanh`. These are the forms Eigen and XLA use for `float`. Measured cost: 1.2 and 1.5 ns per element.

Accuracy against the f64 path is enforced by `crates/medscale-pack/tests/candle_gelu_erf_accuracy.rs`:

- `gelu_erf`: within 1e-6 absolute or 4e-7 relative.
- `gelu`: within 1.5e-6 absolute or 3e-7 relative.

The vendored crate's own test suite passes.

## Same-runner A/B (CI run 39276412481)

The `perf-ab` workflow builds the profiling example twice on one runner: once against main's candle-core (A) and once against the patched one (B). It runs the two alternately for three rounds. The models use random weights with each model's real `config.json`. Inputs are 8–64 tokens; the gold documents have a median of about 16 tokens. Each figure is the median over rounds of the per-round medians.

| Runner | Model | Tokens | Before (A) ms | After (B) ms | Change |
|---|---|---|---|---|---|
| ubuntu-latest | BioMed-109M | 8 | 48.1 | 38.4 | -20% |
| ubuntu-latest | BioMed-109M | 16 | 62.3 | 49.3 | -21% |
| ubuntu-latest | BioMed-109M | 32 | 94.0 | 70.9 | -25% |
| ubuntu-latest | BioMed-109M | 64 | 171.3 | 129.0 | -25% |
| ubuntu-latest | TinyMed-135M | 8 | 23.8 | 18.7 | -21% |
| ubuntu-latest | TinyMed-135M | 16 | 31.6 | 24.6 | -22% |
| ubuntu-latest | TinyMed-135M | 32 | 50.1 | 37.0 | -26% |
| ubuntu-latest | TinyMed-135M | 64 | 89.7 | 65.2 | -27% |
| windows-latest | BioMed-109M | 8 | 81.4 | 75.0 | -8% |
| windows-latest | BioMed-109M | 16 | 96.2 | 84.7 | -12% |
| windows-latest | BioMed-109M | 32 | 131.8 | 106.0 | -20% |
| windows-latest | BioMed-109M | 64 | 198.2 | 153.2 | -23% |
| windows-latest | TinyMed-135M | 8 | 39.5 | 37.3 | -6% |
| windows-latest | TinyMed-135M | 16 | 48.9 | 44.0 | -10% |
| windows-latest | TinyMed-135M | 32 | 67.3 | 57.8 | -14% |
| windows-latest | TinyMed-135M | 64 | 100.7 | 82.1 | -18% |

## Head-to-head rerun with the patch (CI run 39269970221)

The same 22 rows as the original head-to-head (qual-batch `103-h2h-perf-01`) were rerun on the same runner type, pinned OpenMed 3.0.0 against MedScale. The p50 figures are warm per-document times.

| Model | OpenMed p50 ms | MedScale p50 ms | MedScale / OpenMed |
|---|---|---|---|
| OpenMed-NER-AnatomyDetect-BioClinical-108M | 140.3 | 75.3 | 0.54x |
| OpenMed-NER-AnatomyDetect-BioMed-109M | 118.7 | 72.3 | 0.61x |
| OpenMed-NER-AnatomyDetect-BioMed-335M | 387.4 | 410.3 | 1.06x |
| OpenMed-NER-AnatomyDetect-TinyMed-135M | 76.6 | 39.5 | 0.52x |
| OpenMed-NER-DiseaseDetect-BioClinical-108M | 13.5 | 33.4 | 2.48x |
| OpenMed-NER-DiseaseDetect-BioMed-109M-mlx | None | 71.3 | n/a |
| OpenMed-NER-DiseaseDetect-BioMed-335M | 388.5 | 407.6 | 1.05x |
| OpenMed-NER-DiseaseDetect-BioPatient-108M | 140.3 | 75.5 | 0.54x |
| OpenMed-NER-PharmaDetect-BioClinical-108M | 142.1 | 75.2 | 0.53x |
| OpenMed-NER-PharmaDetect-BioMed-109M | 115.6 | 71.9 | 0.62x |
| OpenMed-NER-PharmaDetect-BioMed-335M | 36.0 | 160.8 | 4.47x |
| OpenMed-NER-PharmaDetect-TinyMed-135M | 12.2 | 31.4 | 2.57x |
| OpenMed-PII-BioClinicalBERT-Base-110M-v1 | 59.6 | 83.0 | 1.39x |
| OpenMed-PII-BioClinicalModern-Base-149M-v1 | 84.4 | 79.4 | 0.94x |
| OpenMed-PII-ClinicalE5-Small-33M-v1 | 18.6 | 22.9 | 1.23x |
| OpenMed-PII-ClinicalLongformer-Base-149M-v1 | 569.7 | 37.3 | 0.07x |
| OpenMed-PII-FastClinical-Small-82M-v1 | 27.5 | 32.3 | 1.18x |
| OpenMed-PII-LiteClinical-Small-66M-v1 | 31.8 | 39.9 | 1.25x |
| OpenMed-PII-SuperClinical-Base-184M-v1 | 152.0 | 195.8 | 1.29x |
| OpenMed-PII-SuperClinical-Small-44M-v1 | 76.3 | 99.2 | 1.30x |
| OpenMed-PII-SuperMedical-Base-125M-v1 | 41.9 | 39.0 | 0.93x |
| OpenMed-PII-mLiteClinical-Base-135M-v1 | 27.0 | 32.5 | 1.20x |

Median ratio 1.06x (was 1.19x): MedScale faster on 9 rows (was 6), level on 1, slower on 11 (was 14). The MLX export is not runnable by OpenMed's PyTorch path, as before.

**Absolute times are not comparable across runs.** GitHub runners differ by up to 10x for the same OpenMed row: BioClinical-108M took 140 ms in one job and 13.5 ms in another. Only same-runner ratios and the A/B above are evidence.

## Not changed

- Matrix multiplications, about 80% of the remaining time, still use candle's gemm. Closing the rest of the gap needs faster small-matrix kernels or a different backend.
- candle-transformers' DistilBERT applies tanh-GELU where Hugging Face uses erf-GELU. That is an existing numerics difference, not introduced here.
