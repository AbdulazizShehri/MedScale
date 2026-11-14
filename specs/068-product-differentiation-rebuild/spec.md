# Spec 068 — Product Differentiation Rebuild

**Status**: CLOSED_CANONICAL
**Promoted**: 2026-09-15 by founder after rejecting the prior product UI/positioning
**Base**: `44f8f2eabfa680bfff21851273406e194e0bd862`

## Goal

Rebuild MedScale's Desktop product identity and information architecture so the product is visibly distinct from Cohere/OpenMed and exposes its actual AI/model/evidence posture instead of hiding it behind generic healthcare UI.

## Non-negotiable outcomes

1. Replace the prior blurple/coral/lavender/pine palette with a MedScale-owned, restrained graphite/ice/signal-blue system.
2. Remove the decorative multi-color motif and generic dashboard language that made MedScale look derivative.
3. Promote `Models` and `Evidence` to first-class product routes.
4. Show the local model fabric truth visibly: admitted runtime, model source, task, hardware, trust state, benchmark state, and provenance.
5. Show OpenMed comparison as evidence states (`PROVEN`, `PARTIAL`, `NOT_YET_PROVEN`, `DEFERRED`) with exact evidence references; never use unsupported superiority copy.
6. Preserve authority boundaries: model output remains Proposal/Evaluation evidence, never ClinicalAssertion by confidence alone.
7. Preserve native Slint, keyboard/accessibility semantics, local-first/default-deny behavior, and synthetic-only product data.

## Product direction

MedScale should feel like a high-end clinical operating system: calm, dense when needed, inspectable, and precise. The visual reference is not another healthcare brand. The product language is monochrome-first with one primary signal color, strong typography hierarchy, thin structural borders, deliberate whitespace, and evidence/state data treated as first-class UI.

The Abridge product review in `evidence/068-product-differentiation-rebuild/ABRIDGE_PRODUCT_REVIEW.md` adds a composition rule: **the clinical work itself is the interface**. Home must not regress into a KPI/admin dashboard. Patient/source/evidence context is primary; system/model/runtime detail remains inspectable but visually subordinate until requested. MedScale adapts the workflow principle as `Prepare -> Understand -> Act` and does not copy Abridge branding, layouts, recording visuals, or cloud/service assumptions.

## Out of scope

- No fake model execution in this spec.
- No claim that Hugging Face runtime is already active.
- No claim that MedScale beats OpenMed on NER/PII/model breadth until measured.
- No MESC work.
- No real PHI.
