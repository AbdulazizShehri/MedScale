# Spec 068 Identity Research

**Status:** ACTIVE_EVIDENCE

## Abridge

Public Abridge marketing CSS inspected on 2026-09-15 uses the `Avantt` family across multiple weights plus a separate `Abridge Font` asset.

MedScale does not copy, vendor, or redistribute those assets. The useful reference is the craft pattern:
- strong grotesk hierarchy;
- confident whitespace;
- short clinical copy;
- workflow-first composition;
- evidence/research credibility adjacent to product claims.

Abridge's public product narrative organizes the care workflow around preparation, encounter, and post-visit work. MedScale adapts the product principle, not the visual system, into `Prepare → Understand → Act` with authority boundaries preserved.

Sources:
- https://www.abridge.com/
- https://www.abridge.com/platform/clinicians
- public CSS loaded by abridge.com on 2026-09-15
## Impeccable

Impeccable is used as a design-review discipline and external detector, not as a runtime dependency or repository hook.

Reference commands/principles adopted:
- shape before build;
- critique rendered hierarchy and generic-AI tells;
- distill unnecessary chrome;
- typeset deliberately;
- polish spacing, alignment, and copy;
- harden long/empty/error/conflict states;
- optimize measured performance.

`npx impeccable@latest --version` returned `4.1.0` on the qualification host. `npx impeccable@latest detect --json crates/medscale-desktop/ui` returned an empty result set. Because MedScale uses native Slint rather than HTML/CSS, this result is supplemental only and is not treated as proof that Slint-specific visual issues were scanned.

Sources:
- https://impeccable.style/
- https://github.com/pbakaus/impeccable

## Typography decision

MedScale selects Geist / Geist Mono as its branded target families. Upstream licensing is SIL Open Font License 1.1. Development-host fonts were installed locally for rendered review; font binaries are not yet admitted as packaged repository assets.
