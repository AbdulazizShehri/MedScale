# Spec 070 — Model Center + Runtime UX

**Status**: CLOSED_CANONICAL
**Base**: `99017b9b22400e5b6d35c217bea02c0602c5649e`

## Goal

Turn the existing Models surface into a truthful operator-facing Model Center backed by Core Pack authority, without making qualification references look installed or granting Desktop direct runtime authority.

## Required outcomes

1. Session-admitted Pack inventory is read through Core authority (`PacksList`), not hard-coded as installed state.
2. A local signed Pack can be admitted from Model Center only through `CliSession`/Core authority; refusal reasons remain visible and fail closed.
3. Every admitted Pack row exposes pack/version identity, content digest, runtime requirements, local device boundary, trust root, rights/provenance reference, benchmark links, and promotion state.
4. Pinned Hugging Face qualification evidence is visually and semantically separate from session-admitted inventory.
5. Empty inventory is explicit; Pack registry restart persistence is not claimed because prior durable contracts define Pack state as process-lifetime.
6. The top-level product runtime badge no longer says `FIXTURE`; product copy reflects the real portable ONNX runtime proven by Spec 069.
7. Model Center never turns model output into clinical authority, never enables real-PHI inference, never performs online Hugging Face acquisition, and never promotes a Pack automatically.
8. Model Center remains keyboard/accessibility readable and exposes status through text, not color alone.
9. Desktop smoke/performance modes stay deterministic and do not open a model session.
10. Local qualification, exact-range review, exact-head CI, protected merge, and post-main verification are required before closure.

## Explicit non-goals

- No restart-persistent Pack registry; Spec 016 explicitly keeps packs process-lifetime.
- No accelerated-runtime selection or Apple Silicon winner claim; Spec 071 owns comparison.
- No OpenMed comparative verdict expansion; Spec 071 owns comparative evidence.
- No production clinical model promotion, real-PHI authorization, online model download, or MESC work.
