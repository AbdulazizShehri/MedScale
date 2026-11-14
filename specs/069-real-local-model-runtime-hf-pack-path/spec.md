# Spec 069 — Real Local Model Runtime + Hugging Face Pack Path

**Status**: IN_PROGRESS
**Promoted**: 2026-09-16 after Spec 068 canonical closure
**Base**: `75b5a173a63ed2057c6ecf96ef12faf18ed5e479`

## Goal

Admit a real, bounded, local model-execution path behind MedScale Pack/Core authority boundaries and prove that a pinned Hugging Face ONNX model can be transformed, signed, admitted, and executed locally without granting model code network or authority privileges.

## Required outcomes

1. A real ONNX runtime executes model bytes; `FixtureRuntime` is no longer the only model execution path.
2. The runtime is local/offline and has no Hugging Face HTTP or hub dependency.
3. Model provenance is a signed Pack artifact: source, exact revision, original file digest, transform, task, runtime, license, and rights URI.
4. Runtime output is evidence/proposal-only and can never become `ClinicalAssertion` by confidence alone.
5. Core re-verifies Pack bytes and signed identity on every invocation; prepared execution plans may be cached only by exact signed content digest.
6. Runtime preparation never holds the shared model-cache mutex and user input is never retained in the prepared cache.
7. `REAL_PHI_MODEL_RUNTIME` remains denied; Spec 069 execution is synthetic/evidence-only.
8. A pinned external Hugging Face model must execute semantically through the same runtime boundary without vendoring weights into Git.
9. Portable runtime performance must be measured in optimized release profile and reported without claiming an accelerated-runtime win.
10. Models/Evidence product truth must stop saying `FixtureRuntime only` while continuing to state that no production clinical model is promoted.

## Explicit non-goals

- No production clinical model promotion.
- No real-PHI inference authorization.
- No product online download or Hugging Face SDK/network dependency.
- No MLX/Core ML/GPU/ORT accelerated runtime promotion.
- No claim that MedScale has model breadth or accelerated inference parity with OpenMed.
- No MESC work; Spec 012 remains untouched.
