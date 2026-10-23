# MedScale website — free, local and privacy value propositions

Date: 2026-10-23. Scope: public marketing messages only, on top of the published Beta website.

## Verified product promises

| Marketing copy | Source | Boundary |
| --- | --- | --- |
| **Free to use** | Root `LICENSE` (Apache-2.0) and unsigned public Beta release links on `/download/` | This describes first-party MedScale software, **not** third-party model weights, services, downloads or hardware costs |
| **Runs on your computer** | `README.md`, `PRODUCT.md` and `apps/desktop-tauri/README.md` | Local Core, source-aware synthetic workspace and admitted local packs; live remote partner integrations are not qualified |
| **No automatic data upload** | `docs/security/TRUST_AND_PRIVACY_MODEL.md`, `crates/medscale-network/src/allowlist.rs`, and Core denied egress behavior | This means default-deny product network actions and no product telemetry/crash upload by default; it **does not** promise a permanently air-gapped device |
| **No cloud account required** | `README.md` and local Core usage | External services and optional online actions may have distinct requirements |

## Boundaries shown to visitors

- The Beta is restricted to **synthetic data**, not real patient records or clinical decisions.
- Explicit browsing, exports, and approved external integrations **can transmit data**.
- Models and services from third parties can have separate licensing or charges.
- An unsigned Beta installer is not production, clinical, patient-data, or endpoint privacy qualification.
- `PRIVATE_DATA_READY` is NOT_QUALIFIED, and `TAURI_WEBVIEW_PRIVACY_QUALIFICATION` is OPEN. Never translate default-deny broker behavior into a blanket privacy certification.

## Placement and design

- Homepage: compact hero value proposition, persistent four-column value strip, editorial Free/Local/Private section with clickable policy source, and expanded FAQ.
- Product: local-first and no-upload summary before the real application screenshots.
- Download: free-to-use and account-free claim is explicit, with unsigned Beta and synthetic-only boundary intact.
- Footer: consistent free/local/limited-privacy summary.
- `npm run check` executes `scripts/check-privacy-marketing.mjs` against static exported pages, compares marketed claims to source and rejects absolute privacy language.
- Existing branding (dark UI, orange accents, typography, icon/wordmark gap, real screenshots), source-backed catalog/performance numbers and feature directory are preserved.
- Local Next.js 16.4.2 production build, static checks, source-backed catalog and performance checks, new marketing-integrity gate, TypeScript, and 11 Node unit tests passed.
- Edge Chromium: 18/18 combinations across six pages and 390/768/1440px viewports passed, with no JS errors or horizontal overflow. Homepage, product, and download display their applicable free/local/no-auto-upload messages.
- Impeccable homepage after revision: 3 remaining advisories (2 embedded citation controls with intentionally tight padding, and the approved Inter font), matching the prior live site's final 3. New fineprint line-length advisory was repaired.
- Earlier Lighthouse performance results are not reused as a claim for this change; a fresh post-deployment field or lab measurement should be reported separately.

No competitor names, unverifiable speed/user statistics, private internal plans or regulated-compliance assertions are displayed on the website.
