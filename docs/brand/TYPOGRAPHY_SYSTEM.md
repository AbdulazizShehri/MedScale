# MedScale typography system

**Status:** CANONICAL_SPEC_095. Native source: ui/theme.slint.

Inter UI/wordmark and JetBrains Mono NL exact metadata. Inter's text optical size, tabular numerals and differentiated forms fit clinical density and the reference. NL keeps operators/IDs free of code ligatures. Candidate evaluation: specs/095-brand-foundation/research.md.

| Role/token | Family | Weight | Size | Line height | Tracking | Usage |
|---|---|---:|---:|---:|---:|---|
| Display/type-display | Inter | 600 | 32px | 40px | -0.5px | Editorial/brand, sparing in product |
| Page/type-page | Inter | 600 | 28px | 36px | -0.3px | Route title |
| Section/type-section | Inter | 600 | 18px | 26px | -0.2px | Pane heading |
| Card/type-subsection | Inter | 600 | 14px | 22px | 0 | Semantic group heading |
| Body/type-body | Inter | 400 | 14px | 22px | 0 | Clinical/research reading |
| Compact body/type-row | Inter | 400 | 13px | 20px | 0 | Dense tables/lists |
| Label/type-control | Inter | 500 | 13px | 20px | 0 | Controls/navigation |
| Caption/type-caption | Inter | 400 | 12px | 18px | 0 | Supporting metadata |
| Micro/type-micro | Inter | 500 | 11px | 16px | 0.8px uppercase | Section labels |
| Data/type-code | JetBrains Mono NL | 400 | 12px | 20px | 0 | Numeric/exact values |
| Evidence metadata/type-code | JetBrains Mono NL | 400 | 12px | 20px | 0 | IDs/hashes/time/operators |

Slint Text uses measured metrics without a CSS line-height property. Allocate this rhythm in row/layout geometry; no native line-height property is claimed. Compact preserves text size. Numeric columns use mono/tabular alignment; names/narrative stay in Inter. Oversized headings must not displace clinical information.

Only unchanged Inter Variable Roman and JetBrains Mono NL Regular are added, explicitly required/permitted by Spec 095 for offline deterministic rendering. SIL OFL licenses, source hashes and attribution accompany native imports and checksummed portable redistribution. No extra weights, italics, webfonts or font service. Instrument Sans and Source Serif 4 remain historical Spec 073 assets outside the active scale.
