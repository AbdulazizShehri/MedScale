# Feature Specification: Local AI Capability Fabric

**Feature Branch**: `spec/008-local-ai-capability-fabric`  
**Created**: 2026-08-25  
**Status**: Package QUALIFIED for implementation (005+006+007 CLOSED)  
**Input**: Offline Pack v0 admission, fail-closed format gates, worker ambient-deny confinement proofs, workload FixtureRuntime (no preferred engine), doctor packs axis. No online download, no REAL_PHI, no OpenMed runtime, no ClinicalAssertion from model output.

## User Scenarios & Testing

### User Story 1 - Offline Pack Install (Priority: P1) — 008A

Operator installs a content-addressed Pack from a local path via Core Host (`medscale packs install <local-path>`). Manifest digests/rights/SBOM refs verified. No network.

**Acceptance**: Happy path admits fixture pack; missing rights/digest mismatch denied; CLI uses facade only.

### User Story 2 - Fail-Closed Format Admission (Priority: P1)

Packs declaring pickle, code-bearing `.bin`, or unsigned ONNX custom-ops are denied.

### User Story 3 - Staged Promotion Semantics (Priority: P1)

Pack states `candidate` / `current` / `last-green` / `canary` with fail-closed promote rules (no silent current without candidate).

### User Story 4 - Worker Ambient Deny (Priority: P1) — 008S-lite

Workers receive no ambient canonical DB, master keys, unrestricted FS, network, secrets, or authority. Explicit grants only. OS PLATFORM_QUALIFIED sandbox remains evidence follow-on (EXTERNAL_GATES / limitations).

### User Story 5 - Fixture Runtime Adapter (Priority: P1)

Workload-specific `PackRuntimeAdapter` with FixtureOnly adapter producing Proposal/Evaluation evidence_only—never ClinicalAssertion. No ONNX/llama admitted in this unit.

## Requirements

- FR-001 PackManifestV0 with content digests, rights, SBOM, runtime reqs, promotion state  
- FR-002 Offline local-path install only  
- FR-003 Format deny matrix (pickle / code-bin / unsigned custom-op)  
- FR-004 Promotion state machine skeleton  
- FR-005 WorkerSupervisionPolicy ambient deny + confinement profile id  
- FR-006 Doctor packs_runtime axis  
- FR-007 No HF/online download path; no OpenMed Cargo dep; no REAL_PHI  
- FR-008 Model/worker output ≠ ClinicalAssertion  

## Success Criteria

- SC-001 `medscale packs install` works on fixture pack offline  
- SC-002 Format deny matrix PASS  
- SC-003 Worker ambient deny tests PASS  
- SC-004 Doctor reports packs axis  
- SC-005 Zero product online pack acquisition; no PARITY claim without BenchmarkManifest  

## Anti-scope

Online packs (015), real engine FFI, measured NER/PII PARITY, MESC (012), OCR/ASR (010), mobile sideload (009), REAL_PHI.
