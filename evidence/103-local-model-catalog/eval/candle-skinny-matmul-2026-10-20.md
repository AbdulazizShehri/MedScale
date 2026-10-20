# Candle few-row f32 matmul kernel (2026-10-20)

This is the second Spec 103 latency step after the f32 GELU change
([candle-gelu-latency-2026-10-19.md](candle-gelu-latency-2026-10-19.md)).
At short token counts a `Linear` multiply is bound by streaming the weight.
candle's gemm packs weight panels for every tile.

The vendored candle-core now serves f32 matmuls with at most 64 lhs rows against a transposed contiguous weight, which is the `Linear` layout. Its kernel reads each block of four weight rows once and reuses it for every lhs row: AVX2+FMA, 2x4 register blocking, rayon over the weight rows. It runs only on x86_64 CPUs with AVX2 and FMA, checked at run time. Every other CPU stays on gemm (see [macOS](#macos)). `MEDSCALE_PATCH.md` item 6 has the details.

## Same-runner A/B, balanced (perf-ab run 39360947078)

- **Variants:** A is main's candle-core, B is this branch's, and C is a byte-identical copy of A.
- **Order:** six rounds cover every order of A, B and C, so no variant is favoured by running first or last.
- **Models:** random weights with real `config.json` files (OpenMed BioMed-109M is BERT-base; TinyMed-135M is DistilBERT).
- **Figures:** medians over rounds of per-round medians.
- **Noise floor:** the **C vs A column** measures it. It stays within ±2% on Ubuntu and Windows. On macOS, where A, B and C now run the same code, it is ±11%.

| Runner | Model | Tokens | A ms | B ms | B vs A | C (copy of A) vs A |
|---|---|---|---|---|---|---|
| macos-latest | BioMed-109M | 8 | 46.1 | 51.9 | +13% | +11% |
| macos-latest | BioMed-109M | 16 | 62.2 | 59.7 | -4% | +2% |
| macos-latest | BioMed-109M | 32 | 88.6 | 79.5 | -10% | +3% |
| macos-latest | BioMed-109M | 64 | 149.8 | 116.7 | -22% | +5% |
| macos-latest | TinyMed-135M | 8 | 24.7 | 22.0 | -11% | -0% |
| macos-latest | TinyMed-135M | 16 | 33.6 | 34.9 | +4% | -7% |
| macos-latest | TinyMed-135M | 32 | 49.7 | 53.1 | +7% | -8% |
| macos-latest | TinyMed-135M | 64 | 67.8 | 77.2 | +14% | +11% |
| ubuntu-latest | BioMed-109M | 8 | 62.6 | 20.8 | -67% | -0% |
| ubuntu-latest | BioMed-109M | 16 | 78.0 | 34.9 | -55% | +0% |
| ubuntu-latest | BioMed-109M | 32 | 96.8 | 58.8 | -39% | +1% |
| ubuntu-latest | BioMed-109M | 64 | 150.9 | 121.9 | -19% | +0% |
| ubuntu-latest | TinyMed-135M | 8 | 31.1 | 9.7 | -69% | +0% |
| ubuntu-latest | TinyMed-135M | 16 | 38.3 | 16.9 | -56% | -0% |
| ubuntu-latest | TinyMed-135M | 32 | 50.4 | 30.5 | -39% | +1% |
| ubuntu-latest | TinyMed-135M | 64 | 75.7 | 60.6 | -20% | +0% |
| windows-latest | BioMed-109M | 8 | 33.1 | 17.9 | -46% | +1% |
| windows-latest | BioMed-109M | 16 | 37.9 | 22.0 | -42% | -2% |
| windows-latest | BioMed-109M | 32 | 51.5 | 36.1 | -30% | -2% |
| windows-latest | BioMed-109M | 64 | 81.5 | 70.9 | -13% | -1% |
| windows-latest | TinyMed-135M | 8 | 16.5 | 8.9 | -46% | +1% |
| windows-latest | TinyMed-135M | 16 | 21.2 | 13.4 | -37% | -0% |
| windows-latest | TinyMed-135M | 32 | 30.0 | 22.4 | -25% | -0% |
| windows-latest | TinyMed-135M | 64 | 43.1 | 35.6 | -17% | +2% |

**Earlier unbalanced A/B runs are not used as evidence.** Within every round A ran before B. On macOS, where A and B ran identical code, B still measured 16–38% faster, which exposed an order bias.

## Head-to-head with OpenMed 3.0.0 (run 39356478281)

The same 22 rows as the original head-to-head were run on ubuntu-latest (x86_64) with the kernel. In each job, MedScale and pinned OpenMed run on the same runner. The p50 figures are warm per-document times.

| Model | OpenMed p50 ms | MedScale p50 ms | MedScale / OpenMed |
|---|---|---|---|
| OpenMed-NER-AnatomyDetect-BioClinical-108M | 12.3 | 21.1 | 1.72x |
| OpenMed-NER-AnatomyDetect-BioMed-109M | 117.4 | 29.3 | 0.25x |
| OpenMed-NER-AnatomyDetect-BioMed-335M | 393.0 | 97.0 | 0.25x |
| OpenMed-NER-AnatomyDetect-TinyMed-135M | 13.1 | 16.0 | 1.21x |
| OpenMed-NER-DiseaseDetect-BioClinical-108M | 75.6 | 39.1 | 0.52x |
| OpenMed-NER-DiseaseDetect-BioMed-109M-mlx | None | 32.7 | n/a |
| OpenMed-NER-DiseaseDetect-BioMed-335M | 47.7 | 94.4 | 1.98x |
| OpenMed-NER-DiseaseDetect-BioPatient-108M | 13.2 | 21.1 | 1.60x |
| OpenMed-NER-PharmaDetect-BioClinical-108M | 14.8 | 30.6 | 2.07x |
| OpenMed-NER-PharmaDetect-BioMed-109M | 63.6 | 32.4 | 0.51x |
| OpenMed-NER-PharmaDetect-BioMed-335M | 394.3 | 97.0 | 0.25x |
| OpenMed-NER-PharmaDetect-TinyMed-135M | 75.5 | 18.1 | 0.24x |
| OpenMed-PII-BioClinicalBERT-Base-110M-v1 | 61.5 | 43.6 | 0.71x |
| OpenMed-PII-BioClinicalModern-Base-149M-v1 | 68.8 | 56.2 | 0.82x |
| OpenMed-PII-ClinicalE5-Small-33M-v1 | 20.1 | 13.8 | 0.69x |
| OpenMed-PII-ClinicalLongformer-Base-149M-v1 | 807.7 | 36.1 | 0.04x |
| OpenMed-PII-FastClinical-Small-82M-v1 | 31.0 | 19.1 | 0.62x |
| OpenMed-PII-LiteClinical-Small-66M-v1 | 30.4 | 19.8 | 0.65x |
| OpenMed-PII-SuperClinical-Base-184M-v1 | 132.3 | 134.5 | 1.02x |
| OpenMed-PII-SuperClinical-Small-44M-v1 | 75.2 | 77.3 | 1.03x |
| OpenMed-PII-SuperMedical-Base-125M-v1 | 42.2 | 25.3 | 0.60x |
| OpenMed-PII-mLiteClinical-Base-135M-v1 | 32.2 | 19.4 | 0.60x |

**Across the 22 rows, the median MedScale/OpenMed ratio is 0.65x. MedScale is faster on 14 rows, level (within ±5%) on 2, and slower on 5.**

The progression of the median ratio:
- before the latency work: 1.19x
- after the f32 GELU change: 1.06x
- with this kernel: **0.65x**

MedScale is slower only where OpenMed landed on a runner on which PyTorch is unusually fast (about 12–15 ms). The same OpenMed model takes about 117 ms on other runners. **Absolute times differ across GitHub runners, so only same-job ratios count.** The MLX export (row 13) is not runnable by OpenMed's PyTorch path, as before.

## Accuracy

- `crates/medscale-pack/tests/candle_skinny_matmul_accuracy.rs`:
  - `Linear` on 1–64 tokens matches an f64 reference within 2e-6·sqrt(k).
  - The fast path agrees with gemm within 1e-4.
  - These tests run in the workspace suite on every CI platform.
- The unit test in `vendor/candle-core-0.9.1/src/cpu_backend/skinny.rs` and the vendored crate's own test suite pass.

## macOS

A portable (non-SIMD-intrinsic) version of the loop measured 2.4–7.7x **slower** than gemm's NEON kernels on Apple Silicon (perf-ab run 39357885288). The kernel is therefore x86-only, and Apple Silicon keeps gemm. The balanced run confirms macOS is unchanged within its noise floor.
