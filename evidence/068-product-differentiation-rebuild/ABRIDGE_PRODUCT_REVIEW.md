# Abridge Product Review — Spec 068

**Reviewed:** 2026-09-15
**Source class:** Product/pattern evidence only; Abridge is not runtime authority for MedScale.
**Primary source:** https://www.abridge.com/

## Why this review exists

Abridge demonstrates a stronger clinician-facing product composition than the rejected MedScale dashboard direction. The useful lesson is not its branding or exact layouts; it is that the clinical work itself is the interface.

Current Abridge product materials organize the clinician experience around the care encounter: preparation before the visit, intelligence during the conversation, and reviewable documentation/actions after the visit. Its product framing consistently reduces visible process and keeps source-grounded work close to the clinician.

## Observed product principles

- Workflow first, dashboard second: the encounter and resulting work dominate the interface.
- Low chrome: the product avoids making metrics, configuration, or infrastructure the primary visual object.
- Conversation/source traceability is visible rather than buried in settings.
- Generated clinical output is presented as reviewable draft work, not silent authority.
- Before / during / after states create a clear temporal model of what the clinician should do next.
- Primary actions are few and obvious; secondary system detail stays inspectable rather than constantly visible.
## Visual observations relevant to MedScale

Abridge uses warm brand accents and light clinical surfaces, but the stronger design lesson is hierarchy: the active encounter or note is visually dominant, controls stay quiet, and transcript/source context is close to the generated output. Historical product screenshots also show timestamps, highlighted clinical terms, playback controls, and source-oriented review instead of a KPI-heavy landing page.

MedScale must not copy Abridge's palette, recording waveform, typography, or layouts. The product should translate the structural lessons into its own identity:

1. The central work surface owns visual priority.
2. Patient/source/evidence context appears before system metrics.
3. AI state is inspectable at the point of use.
4. Model/runtime details are one click away, not disguised as generic assistant capability.
5. Review and authority boundaries are visible in the workflow itself.
6. Infrastructure truth remains available without turning Home into an operator dashboard.

## Spec 068 disposition

**ADOPT:** workflow-first composition, reduced chrome, source-linked review, explicit temporal/authority progression.
**ADAPT:** Abridge's before/during/after encounter model into MedScale's `Prepare → Understand → Act` evidence/authority flow.
**REJECT:** copying Abridge branding, remote-service assumptions, ambient-scribe-first product identity, or any claim that its cloud architecture is MedScale authority.

The resulting MedScale Home must read as a clinical workspace, not an admin dashboard. Model Center and Evidence Center remain first-class because MedScale's differentiator is inspectable local intelligence plus authority/provenance, not an invisible AI service.