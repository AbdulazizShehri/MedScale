# MedScale wordmark provenance (founder-approved 2026-10-22)

**Authority:** on 2026-10-22 the MedScale founder directed that the logo lettering change ("I do not like the font/lettering currently used for MedScale in the logo. Change it."). The founder rejected a first proposal (Inter Display 480 with ascender cuts, symbol at 2.6× cap height) as visually unbalanced. The founder then reviewed comparison sheets of four directions at three symbol ratios, in real contexts, and approved **direction A: Plus Jakarta Sans, with the symbol at 2.05× the cap height**. The three-frame symbol's geometry is unchanged.

**Supersedes for active branding:** the Logo 12 monoline wordmark in [`logo-12-approved-identity.md`](logo-12-approved-identity.md). That record and `evidence/brand-logo-12/` stay unchanged as history. Logo 12 remains the authority for the symbol.

## Evaluation

The evidence is in `evidence/brand-wordmark-2026-10-22/`.

1. **Screening:** twelve OFL families, each at weights 500 and 620: Geist, Manrope, Space Grotesk, Plus Jakarta Sans, Inter Tight, Hanken Grotesk, Schibsted Grotesk, Funnel Display, Onest, Bricolage Grotesque, Inter Display and Instrument Sans.
2. **Matrix:** four directions × three symbol heights (1.8×, 2.05× and 2.3× cap). Each was shown at a 26 px cap and at the real 12 px (header) and 9 px (sidebar) caps.
   - A: geometric (Plus Jakarta Sans 590)
   - B: contemporary (Geist 560)
   - C: editorial (Schibsted Grotesk 600)
   - D: technical (Space Grotesk 560)
3. **Contexts at real displayed sizes** for each direction, rendered at 2× device pixels:
   - white on black and black on white
   - desktop and mobile website navigation
   - the desktop app sidebar
   - GitHub README, light and dark
4. **Founder decision:** direction A at 2.05×.

Findings that set the lockup rules:

- At 2.6× the symbol dominated the word.
- At the original 26.5 px stroke the frames looked spindly beside medium-weight letters.
- At a fully weight-matched stroke the gaps between the frames closed up.
- A 1.5× lockup stroke balances the frames against the letters.

## Construction

`assets/brand/source/wordmark.py` generates the artwork, and `assets/brand/source/geometry.json` records its parameters.

- **Source outlines:** `assets/brand/fonts/PlusJakartaSans-Variable.ttf`, instanced at weight 590.
  - Upstream: `google/fonts` commit `8cd7d0de182c88592d6852c245fe48f66eef55ee`, `ofl/plusjakartasans/PlusJakartaSans[wght].ttf`.
  - SHA-256 `89b3fb38aa0d275d7a731d0d817a4f1622b316b4d7fbdedcf02ee9099ff68bc8`.
  - License `assets/brand/fonts/PlusJakartaSans-OFL.txt`, SHA-256 `995c7199cab65954f545996326755daee7b63cc6b42b06c13da1f9502ab08a99`.
- **Spacing:** the font's own GPOS pair kerning, tracking −0.018 em, and −0.012 em extra before "S" to close the d–S pair.
- **Output:** one filled path per letter, with the cap height at 180 reference px, normalized to its own bounds.
- **Horizontal lockup:**
  - The symbol is 2.05× the cap height, drawn with a 1.5× optical stroke. The geometry is unchanged.
  - The gap is 0.5× the cap height.
  - The symbol is centred on the cap height and nudged down 4% of its height, because the descender reads as weight below.
- **Vertical lockup:** the Logo 12 composition, with the symbol centred above the word and 58 reference px between the symbol and the cap height.

## Rights

Plus Jakarta Sans is © 2020 The Plus Jakarta Sans Project Authors and licensed under the SIL Open Font License 1.1. The license declares no Reserved Font Name. The OFL permits using the font to make artwork. The wordmark is vector artwork made from the font's outlines. It is not font software and does not relicense Plus Jakarta Sans. The font file is kept in the repository only as the source for the brand generator. No brand asset embeds or references it, and the website and app draw the wordmark from path data.
