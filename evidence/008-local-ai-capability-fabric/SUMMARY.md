# Spec 008 evidence summary

## Delivered (008A + 008S-lite)

- Offline Pack v0 admit via Core Host + `medscale packs install`
- Format deny: pickle / code_bin / onnx_custom_op
- Promotion candidate→current
- WorkerSupervisionPolicy ambient deny + confinement_profile
- FixtureRuntime Proposal-only (never ClinicalAssertion)
- Doctor `packs_runtime` axis; online_download_authorized=false

## Limitations

- No ONNX/llama/Candle DEPENDENCY admitted
- No measured NER/PII PARITY/SURPASS (needs BenchmarkManifest + Spec 007 corpora harness)
- OS PLATFORM_QUALIFIED sandbox: EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` OPEN
- No online pack acquisition (Spec 015)
