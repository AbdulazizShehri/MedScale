# Spec 073 Rendered Evidence

## Capture authority

- Host: authorized macOS development host.
- Runtime: native `medscale-desktop` Slint binary built from the Spec 073 candidate working tree.
- Capture method: CoreGraphics window enumeration bound to the spawned process PID, followed by `screencapture -l <window-id>`.
- Theme/route selection: deterministic engineering-only `MEDSCALE_EVIDENCE_THEME` and `MEDSCALE_EVIDENCE_ROUTE` overrides. Product default remains OS-following.
- The screenshots are evidence of rendering only; they do not establish release readiness, WCAG conformance, qualified-hardware performance attainment, clinical authority, or real-PHI authorization.

## Final after evidence

| Surface | Theme | SHA-256 |
|---|---|---|
| Home | Light | `bb55201b061fc11f9f4677bbed4bcf5411ebc4413f87d21a28ffe40d85e1de13` |
| Home | Dark | `fb64630eca5e6a307a3a37889920bf40c1ec4b13a9dd74177622897a6af9f4eb` |
| Patients | Light | `3d1f7b35f6e901d0afe38468a5165e43f611395f65a26207b6394c8f85add175` |
| Models | Light | `363a9f7bb9143dcc0898572923b8fac63bbaf60dbb0a32e719fb865b2d2cba06` |
| Evidence | Light | `4f5658413b3cdcfb07e47bd20d48d83b9e1dbd40edec43674ae453f3bd5903ce` |
| Workflows | Light | `1f81607f5e7884a8d9df2d418ef1d040d20f71b46765944144e9c508771b206d` |
| Settings | Dark | `efe49a636e3491ee82895062614757b620d5882b58403bcdca749eb570abab44` |

`logs/render-capture-final.log` retains the final window IDs and digest output.

## Visual review

Final review result: `PASS — NO MATERIAL VISUAL FINDINGS REMAIN`.

The final review covered hierarchy, native window framing, dual-dock composition, signature-mark legibility, typography, semantic color restraint, light/dark surface layering, field rendering, overflow, clipping, status readability, workflow density, and truth-boundary copy.

Material findings discovered and corrected during the rendered loop:

1. Native text fields inherited macOS colors in Light mode. `AdaptiveLineEdit` now owns MedScale surface, text, placeholder, focus, and accessibility treatment.
2. Model Fabric did not provide enough vertical room for the truthful runtime/source explanation. Its layout was expanded rather than truncating or hiding evidence text.
3. Workflows became too compressed after the distillation pass and produced visible text overlap. Step rows and the workflow panel were expanded while retaining the quieter row treatment.
4. The `INSPECTABLE` pill on the obsidian AI panel had insufficient visual contrast in Light mode. `StatusPill` now permits explicit text color and the panel uses the navigation-light text token.
5. The original rounded `M` was too generic for the founder's recognition requirement. The final monochrome signature geometry adds the `MedScale Shelf`, preserving immediate `M` recognition at rail size without adding a medical or AI cliché.

No screenshot from an earlier failed capture attempt is treated as final evidence.
