# Spec 007 evidence — OpenMed capability absorption / parity research

**Status**: Research-only  
**Runtime**: None required by this unit  
**REAL_PHI**: Not authorized  
**MESC mutation**: Forbidden  

## Limitations

- Does not implement Rust NER/PII/de-ID fabric (Spec 008).
- Does not import OpenMed Python runtime or wholesale-copy the OpenMed tree.
- Does not admit licensed terminology tables or model weights.
- Measurement harness ownership is Spec 008+; Spec 007 owns design/registry/dispositions.
- SURPASS claims are forbidden until exact-head BenchmarkManifest evidence exists.

## Artifacts

| Path | Purpose |
|---|---|
| `BASELINE.md` | Frozen OpenMed v2.2.0 pin |
| `PIN_VERIFICATION.md` | Pin verification record |
| `CORPUS_REGISTRY.md` | Synthetic corpus descriptors |
| `SAUDI_ARABIC_PROGRAM.md` | Arabic/Saudi trap program design |
| `BENCHMARK_MANIFEST_CHECKLIST.md` | Claim envelope checklist |
| `fixtures/` | Optional synthetic stubs only |
| `../../docs/matrices/openmed-parity-matrix-v2.2.0.json` | Full V2 parity matrix instance |
| `../../docs/matrices/openmed-absorption-dispositions.md` | Donor dispositions |
| `../../docs/matrices/terminology-rights-track.md` | Terminology rights track |
| `../../third_party/provenance/_TEMPLATE-openmed-component.md` | Provenance template |
