# MedScale wordmark provenance (founder amendment, 2026-10-22)

**Authority:** on 2026-10-22 the MedScale founder directed that the lettering of "MedScale" in the logo be changed ("I do not like the font/lettering currently used for MedScale in the logo. Change it."). This is an explicit amendment to the Logo 12 rule that locked the wordmark. The three-frame symbol is unchanged.

**Supersedes for active branding:** the monoline wordmark described in [`logo-12-approved-identity.md`](logo-12-approved-identity.md). That record, its IoU measurements and `evidence/brand-logo-12/` stay unchanged as history. Logo 12 remains the authority for the symbol.

## Concepts evaluated

Each concept was rendered as white on black and black on white, in a website header at desktop and mobile sizes, in a desktop sidebar, and at README size (16 px symbol). The comparison sheets are in `evidence/brand-wordmark-2026-10-22/`.

| Concept | Source | Result |
|---|---|---|
| A. Precision | Inter Display (opsz 32), weight 460–600, tight tracking | **Selected at weight 480**, with the ascender cut below |
| B. Instrument | Instrument Sans Medium/SemiBold | Runner-up. Good character, but wider, and the family is a superseded Spec 073 asset outside the active type system |
| C. Editorial | Source Serif 4 Regular/Semibold | Rejected. Elegant, but it pulls away from the geometric symbol and loses clarity at 16 px |
| Weight study | Inter Display 600 | Rejected. It overpowers the symbol's thin uniform stroke |

## Construction

`assets/brand/source/wordmark.py` generates the artwork, and `assets/brand/source/geometry.json` records its parameters:

- **Source outlines:** `assets/brand/fonts/InterVariable.ttf`, the admitted Inter v4.1 file (SHA-256 recorded in `095-native-typography.md`), instanced at opsz 32 and weight 480.
- **Spacing:** tracking −0.020 em, plus hand spacing of −0.008 em before "e" and −0.010 em before "c".
- **Bespoke detail:** the flat ascender tops of "d" and "l" are cut at slope 0.4542, which is the symbol's frame top slope (`ts` = −0.4542). The word rises to the right like the frames. At small sizes the cut disappears and the word stays clean.
- **Output:** one filled path per letter, scaled so the cap height is 180 reference px and normalized to its own bounds.

## Rights

Inter is © The Inter Project Authors and licensed under the SIL Open Font License 1.1 (`assets/brand/fonts/Inter-OFL.txt`). The OFL permits using the font to make artwork. The wordmark is vector artwork made with the font, with modified outlines; it is not font software and does not relicense Inter. No font file is embedded in or referenced by any brand asset, and the website and app draw the wordmark from path data without loading a font.
