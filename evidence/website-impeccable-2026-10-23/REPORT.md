# MedScale website improvement — verified engineering evidence

Date: 2026-10-23 UTC. Product stage: **Beta / active development / synthetic-only**.

## Scope and provenance
- Starting website branch: `preview/medscale-site-live` at `ad94e1e830984dfa261c00f2f7a4a5c41b415ad9` (verified on GitHub).
- Protected `main` at `492175ad9119abc2462f1c9a90dcf99ceca29a63` at investigation time, not modified.
- Existing PR #291 at `20e985e7c62b60527651b76b6c1dc1655e3bfc2f` targets main. Its styling refinements were already present on the live website branch, so cherry-pick was correctly skipped as empty after preserving newer catalog/performance CSS.
- Deployment job #39919417663 succeeded for the **previous** live branch head `ad94e1e8`. It is **not** evidence that this improvement branch has deployed.
- No analytics, telemetry, paid service, external font, proprietary image asset, or production-signing requirement was introduced.

## Impeccable advisory results (six pages; desktop live vs rebuilt local)
| Page | Live before | Local after |
|---|---:|---:|
| / | 6 | 3 |
| /product/ | 8 | 1 |
| /models/ | 2 | 1 |
| /download/ | 1 | 1 |
| /docs/ | 1 | 1 |
| /community/ | 1 | 1 |
| TOTAL | 19 | 8 |

Baseline from earlier single-page session: 17 findings on the homepage. New live homepage baseline: 6. Rebuilt local homepage: 3. Counts reflect detector warnings, not a WCAG audit or proof of quality. Remaining 6 overused-font advisories intentionally preserved because Inter/Plus Jakarta Sans are approved branding. Two home `button.claim` inline-text padding warnings remain; further padding would damage the interactive in-sentence evidence highlighting.

## Actual changes
- Removed saturated colored screenshot-frame glow while retaining a subdued neutral elevation.
- Increased small catalog, performance caption, and download count text readability, without modifying claim numbers.
- Adjusted product-body max line length.
- Preloaded the existing JetBrains Mono font with `font-display: optional` to prevent late font layout shifts. No font family replacement.
- Pin a **verified published** unsigned Beta EXE/DMG link fallback while allowing dynamic GitHub Releases discovery; a GitHub API outage cannot remove primary download calls to action.
- Keep `QUALIFIED_RELEASE_TAGS=[]` and synthetic-only/unsigned disclosure. The static-export check allows ONLY two explicitly pinned release asset URLs.

## Accessibility, browser QA, and checks
- Next.js 16.4.2 production build: PASS. Static route/link/asset checks: PASS. Catalog metrics evidence script: PASS (2,266 externally indexed entries, 35 language tags, 5 model families). Performance metrics evidence script: PASS (p95 280ms search / 10,000 synthetic docs, p95 32ms projection / 64 events, synthetic 20K CSV rows). Unmodified source-backed interpretations.
- Node unit tests: 11/11 PASS (adds pinned Beta fallback test); `npx tsc --noEmit`: PASS; `git diff --check`: PASS.
- Real Edge Chromium via Playwright: 6 routes x 390px / 768px / 1440px = 18 viewport-route combinations, 18 HTTP 200 responses, no page JS errors or horizontal overflow. Mobile menu opens, and Escape closes it. Browser test used `prefers-reduced-motion: reduce`.
- Fail-closed API simulation: GitHub Releases API request deliberately aborted, and both direct Beta download anchors remained visible. Windows EXE and macOS DMG HTTP HEAD requests returned status 200 with `application/octet-stream`. This validates asset availability, **not successful installation on operating systems**.
- Lighthouse mobile lab via headless Edge, localhost, before mono-font fix: performance 58/100, CLS 0.282; after mono-font fix: performance 87/100, CLS 0.000, accessibility 100/100, best-practices 100/100, SEO 100/100. Refreshed local final LCP ~2,985ms, FCP ~2,034ms, TBT ~286ms. A separate live-site lab run on the historical deployed version scored performance 75/100, CLS 0.2825, LCP ~1,896ms. Lab runs vary; do not claim final LCP is faster than the prior live site. Field Core Web Vitals were not measured.

## Downloads (source: public GitHub Releases)
- Tag: `engineering-preview-19d419dd7b15` (public pre-release dated 2026-10-23).
- Windows: `MedScale-engineering-preview-windows-x64.exe`, 13,872,773 bytes.
- macOS: `MedScale-engineering-preview-macos.dmg`, 20,827,457 bytes.
- Both are unsigned/unnotarized engineering previews and limited to synthetic-only evaluation; no patient-data, medical device or clinical qualification implied.

## Release and CI boundaries
- GitHub issue #286 documents yanked `clipboard-win 5.4.1` dependency and failing cargo-deny. A website-only PR must **not** weaken cargo-deny or bypass protected branch policies to merge. This issue remains externally blocked until resolved by a separate scoped lockfile correction.
- The Beta website can be polished without declaring MedScale stable or changing application packaging.
- This report is GitHub engineering evidence, not public website marketing copy.
