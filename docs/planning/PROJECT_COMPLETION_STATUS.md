# MedScale Project Completion Status

## Current program — 2026-09-29

CURRENT_PROGRAM = MEDSCALE_IDENTITY_AND_DESKTOP_PRODUCTIZATION
CURRENT_PROGRAM_STATUS = IMPLEMENTATION_IN_PROGRESS
RESEARCH_OS_SPECS_068_094 = CLOSED_CANONICAL_FOR_AUTHORIZED_SCOPE
NEXT_PROMOTED_SPEC = 095
LAUNCH_UI_FROZEN = FALSE

The founder request promotes [095–100](PRODUCTIZATION_PROGRAM.md). Base 1e2b7d94, CI 36480424698 (6/6). Queue/closure supersede summaries ending 073/075. Drafts 124/125 stay separate. Release/privacy/multi-client readiness remains false and PHI remains unauthorized.

## Spec 103 / 104 / 105 model execution — 2026-10-13 (updated after #223–#230 and batches 01–03)

```text
SPEC_103 = IMPLEMENTATION_IN_PROGRESS (repository-owned work done; remaining items are external)
SPEC_104_NATIVE_SAFETENSORS_RUNTIME = CLOSED_CANONICAL (#198; extended by #203, #208)
EXECUTED_TESTED_ARCHITECTURES = bert, distilbert, roberta, modernbert, deberta-v2, xlm-roberta, eurobert, qwen (int8 export), longformer (PyTorch rows, exact local-window path)
EXECUTED_TESTED_ZERO_SHOT = GLiNER uni-encoder span (OpenMed ZeroShot MLX exports, Small and Medium)
RUNTIMES = tract ONNX (CPU, integrity-checked plan cache); candle safetensors and OpenMed MLX exports (CPU; Apple GPU via Metal); candle GLiNER zero-shot; candle text generation (qwen2, mistral, llama; synthetic only)
EXECUTED_TESTED_CATALOG_ROWS = 931 (row-level recorded runs; batch harness, batches 01–26)
EXPECTED_RUNNABLE_CATALOG_ROWS = 797 more (1728 of 2266 executed or expected; architecture-level expectation, not per-row tests)
ESTIMATED_OVER_BOUND_ROWS = 83 (even fp16 exceeds the unchanged 1 GiB bound)
TASK_QUALIFIED_MODELS = 0
CLINICALLY_VALIDATED_MODELS = 0
RELEASE_READY = FALSE
```

The founder's issue #183 priority, OpenMed model-execution parity, is tracked in [`specs/103-local-model-catalog/CAPABILITY_MATRIX.md`](../../specs/103-local-model-catalog/CAPABILITY_MATRIX.md).

The full lifecycle runs with real OpenMed v3.0.0 models on free GitHub runners. Every stage below is backed by recorded runs:

| Stage | Implementation |
|---|---|
| Discover | catalog with a Core capability |
| Acquire | live governed acquisition through the product CLI, at the catalog-pinned revision, with byte-range resume |
| Verify | catalog hash and per-file digests |
| Admit | signed Pack |
| Load / run | tract ONNX; candle safetensors and MLX exports; Apple GPU via Metal |
| Compare | Model Fleet across runtimes |
| Unload | residency pool |

Remaining, each with its cause:
- the Models UI and the desktop parts of #182: blocked by F096-T01;
- GLiNER zero-shot: the 91 PyTorch rows ship pickle only, and the Base and Large MLX exports exceed the 1 GiB Pack bound (the Small and Medium MLX exports run);
- Clinical Longformer: tract prepares the export after the omitted-`Pad` rewrite but panics in `Gather` (contained); candle has no Longformer encoder;
- privacy-filter: 1.49–2.80 GB, over the 1 GiB Pack bound;
- XLM-R NER exports: defective upstream tokenizers;
- PyTorch-format and MLX XLM-R: over the 1 GiB Pack bound, and raising it is a founder decision;
- quantized MLX: not supported;
- ONNX Runtime: evaluated, not admitted;
- Android and WebGPU: MedScale has no such product surfaces;
- generative and multimodal models: not started (outside token classification; 1.3–20.6 GB);
- the first DeBERTa-v2 prepare on tract per machine is non-interactive (about 11–13 minutes); later prepares take about 3 s from the plan cache, and the candle path prepares the same model in about 3 s;
- task qualification and clinical validation: not performed.

See [external gates](EXTERNAL_GATES.md).

## Historical completion status through Spec 073

```text
STATUS = REPOSITORY_IMPLEMENTATION_COMPLETE_PENDING_EXTERNAL_GATES
MEDSCALE_TRUSTED_V1_IMPLEMENTATION_COMPLETE = TRUE
MEDSCALE_DESKTOP_CLI_PRODUCT_PHASE_COMPLETE = TRUE
MEDSCALE_IMPLEMENTATION_COMPLETE = TRUE
MEDSCALE_RELEASE_READY = FALSE
PRIVATE_DATA_READY = FALSE
MULTI_CLIENT_RELEASE_READY = FALSE
REAL_PHI_AUTHORIZED = FALSE
KNOWN_REPOSITORY_OWNED_TRUSTED_V1_RESIDUALS = 0
KNOWN_REPOSITORY_OWNED_DESKTOP_CLI_RESIDUALS = 0
NEXT_PROMOTED_SPEC = NONE
```

Repository-owned MedScale implementation is canonically complete through Spec 073. The founder-promoted bounded cross-surface identity pass closed after native Desktop implementation, safe human-facing CLI identity, documentation-only Web reference, exact-head qualification, protected merge, and post-main verification. Specs 068–073 remain `CLOSED_CANONICAL`; no repository-owned implementation unit remains promoted. Mobile and all 074+ advanced work remain deferred unless freshly promoted by a new canonical decision.

This implementation closure does not imply distribution/release qualification. `RELEASE_READY=false`, `PRIVATE_DATA_READY=false`, `MULTI_CLIENT_RELEASE_READY=false`, `REAL_PHI_AUTHORIZED=false`, qualified-hardware budget attainment is unproven, production signing/notarization is not granted, final macOS signed-product/App Sandbox enforcement remains external, and WCAG/assistive-technology qualification remains externally unmeasured. MESC is a separate project/repository and is excluded from MedScale completion and release calculations. Historical MedScale Specs 012/036 remain provenance only.

## 2026-09-15 founder product-bar reopening

The founder rejected the prior Desktop visual direction, the invisibility of real model/runtime state, and the absence of an in-product evidence-backed OpenMed comparison. Specs 068–072 are freshly promoted to rebuild product identity, admit a real local model runtime/HF pack path, expose Model Center, expose OpenMed Evidence Center, and requalify the resulting release candidate. Specs 000–067 remain canonically closed as historical implementation evidence; this new sequence raises the product bar rather than rewriting those closures.

## Spec 068 canonical closure

Spec 068 final head `1119378ec312c52a5bf9c5dd24a26005b1fd418c` passed all six required jobs in pull-request run `35004533213`. PR #113 merged normally as `75b5a173a63ed2057c6ecf96ef12faf18ed5e479`, and post-merge main run `35031392283` passed all six required jobs. Product identity, Models/Evidence visibility, rendered design evidence, and the Abridge/Impeccable-informed design system are therefore canonical historical evidence.

## Spec 070 canonical closure

Spec 070 final head `9cf2037bad29adbba034836347927834c345c1ba` passed all six required jobs in pull-request run `35065471973`. PR #115 merged normally as `129b63d0d93e8fc14fa0dcd786d95b1ac4697e68`, and post-merge main run `35066357815` passed all six required jobs. Model Center is therefore canonically closed; OpenMed comparative claim qualification remains owned by Spec 071.

## Spec 071 canonical closure

Spec 071 final head `fc8e7092cfe622db4ae0a77922be587989ec376a` passed all six required jobs in pull-request run `35073913261`. PR #116 merged normally as `e2031bdef918980cc184aed811f381e41f5dfb8b`, and post-merge main run `35075937209` passed all six required jobs. The pinned OpenMed v2.2.0 comparator, complete 39-row fail-closed claim ledger, and native Evidence Center are therefore canonical. Product requalification remains owned by Spec 072.


## Spec 072 canonical closure

Spec 072 final head `02e343ff11fbe914a0b312d4e8235552d9b89917` passed all six required jobs in pull-request run `35089753209`. PR #117 merged normally as `f97637e7e9435be8972cc908ced8354be0f7b24e`, and post-merge main run `35091230970` passed all six required jobs. Rebuilt-product routes, Model Center and Evidence Center truth boundaries, accessibility/performance external action packets, five-class release residual accounting, and MESC project separation are therefore canonical. Product identity polish remained owned by Spec 073.


## Spec 073 canonical closure

Spec 073 final head `3ea26b297ef973ea6d0b992e50c99facfdab55c7` passed all six required jobs in pull-request run `35174991802`. PR #119 merged normally as `f9587c3ed66609b6e2a7c23bf2c45df6a67c8f2e`, and post-merge main run `35175649969` passed all six required jobs. The founder-approved monochrome signature mark with MedScale Shelf, adaptive Light/Dark tokens, embedded typography, icon grammar, dual-dock shell, safe CLI identity, and documentation-only Web reference are therefore canonical. Specs 068–073 remain `CLOSED_CANONICAL`. No repository-owned MedScale implementation residual remains promoted.
