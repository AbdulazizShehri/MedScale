# MedScale website — brand spacing and complete feature marketing

Date: 2026-10-23. Scope: website only, on PR #292 over the website publication branch.

## Source-backed marketing
- The **24 desktop feature areas** are taken directly from `apps/desktop-tauri/src/App.tsx` and cross-checked with `apps/desktop-tauri/README.md`.
- Groups: Clinical (4), Research (6), Models & agents (3), Operations (5), Governance (6). The Home dashboard is **not** counted.
- Feature titles appear on the homepage and fuller per-feature descriptions appear on `/product/#all-features`.
- `web/scripts/check-marketing-feature-coverage.mjs` compares advertised labels to desktop routes during the existing website CI check, on both published pages. It fails if either side drifts.
- Copy is positioned for researchers, developers and **synthetic-data Beta testers**, not clinical adoption. The hero CTA leads to `/download/`; the adjacent CTA leads to the full feature directory.
- Preserved evidence-backed 2,266 external catalog records, 35 language tags, 5 model families, measured synthetic 280ms/32ms/20K results, GitHub installer-download reporting and their disclosures. No invented adoption, speed or clinical claims.

## Brand spacing
- The founder-approved MedScale mark and wordmark are two existing public brand assets; **no SVG geometry or font family is changed**.
- Header and footer use `mark-white.svg` and `medscale-wordmark-white.svg` with a CSS `gap: 13px` for independent spacing.
- Real Edge Chromium browser bounding-box measurements across 360px, 390px, 768px and 1440px widths measured a **13px** gap.

## Verification
- `npm run build`: PASS, all routes statically exported.
- `npm run check`: PASS (9 pages, internal links/brand/disclosures, marketing benchmarks and 24-of-24 route parity).
- `npm test`: 11/11 PASS; TypeScript check PASS.
- Playwright against locally served export: 12/12 route-viewport cases PASS for homepage, product and download across 360, 390, 768 and 1440px. All HTTP 200, no JavaScript exceptions, no horizontal overflow, mark and wordmark loaded, all 24 feature labels visible on homepage and product.
- Lighthouse mobile lab for this marketing version (single run): **performance 78/100**, accessibility **100**, best practices **100**, SEO **100**, CLS **0**. Lab results are variable and are **not** production field Core Web Vitals.
- Impeccable re-audit: homepage **3 warnings** (two in-sentence citation-button padding and the approved Inter font), product **1 warning** (the approved Inter font). Earlier live audit found 6 homepage and 8 product warnings. Detector results are advisory, not a WCAG conformance test.

## Publication guardrail
Do not claim this is deployed until the exact head merges through the website publication branch and the post-push GitHub Pages deploy run finishes successfully. Existing #293 and #294 fix a yanked Rust dependency; security tests must pass without bypass. Real-PHI, regulated medical and signed-production claims remain disallowed.
