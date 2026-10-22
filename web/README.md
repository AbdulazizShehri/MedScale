# MedScale website

The public product and open-source website. It is a Next.js App Router site with TypeScript and a static export, so it needs no backend and no paid hosting. The website is separate from the MedScale desktop app and never connects to it or to any clinical system.

The design system is in [`docs/brand/WEBSITE_SYSTEM.md`](../docs/brand/WEBSITE_SYSTEM.md) and the copy rules are in [`docs/brand/MARKETING_COPY.md`](../docs/brand/MARKETING_COPY.md).

## Develop

From `web/`:

```sh
npm ci
npm run dev        # http://localhost:3000
npm test           # release-selector unit tests
npm run build      # static export to out/ (includes the prefetch payload fix)
npm run check      # verifies the export
```

`npm run check` verifies:

- every route and brand asset
- the logo is byte-identical to `assets/brand/`
- the release gate is closed
- the preview disclosures are present
- no remote scripts, stylesheets or fonts
- no installer links
- every internal link and asset resolves under the configured base path

## Configuration

| Variable | GitHub Pages | `medscale.si` |
|---|---|---|
| `NEXT_PUBLIC_SITE_BASE_PATH` | `/MedScale` | empty |
| `NEXT_PUBLIC_SITE_URL` | `https://abdulazizshehri.github.io/MedScale` (default) | `https://medscale.si` |

Both builds are verified by `npm run check`. Canonical links, Open Graph URLs, `sitemap.xml` and `robots.txt` follow `NEXT_PUBLIC_SITE_URL`.

## Deploy

`.github/workflows/website.yml` builds and checks every pull request that touches `web/`. On `main` it publishes `web/out` to GitHub Pages through the `github-pages` environment. The `preview/medscale-site-live` branch can publish a preview through the same environment. Update it only by merging, never by force-pushing.

## Cutover to medscale.si (not done)

Do these steps only after the founder authorizes the cutover.

1. In the website workflow, set `NEXT_PUBLIC_SITE_BASE_PATH` to empty and `NEXT_PUBLIC_SITE_URL` to `https://medscale.si`. Add `web/public/CNAME` containing `medscale.si`.
2. At the DNS provider, add apex `A` records for GitHub Pages (185.199.108.153, 185.199.109.153, 185.199.110.153, 185.199.111.153), the matching `AAAA` records, and a `www` CNAME to `abdulazizshehri.github.io`.
3. In the repository's Pages settings, set the custom domain to `medscale.si`, wait for the certificate, then turn on **Enforce HTTPS**.
4. Verify that `https://medscale.si/` returns 200 with the hero and that `npm run check` passes for the root build. Then check `sitemap.xml`, `robots.txt`, the favicon and the social card.
5. Leave the project Pages URL working until search engines pick up the new canonical URLs.

## Downloads

The platform cards show what is genuinely available and never invent an installer URL. `src/lib/release.ts` holds the platform-aware installer selection, adapted from CommunityFinance-CoFi's tested selector, plus a qualification gate: `QUALIFIED_RELEASE_TAGS` is empty, so the site makes no release request and links only to build instructions.

Enable direct downloads only when all of these hold:

- a GitHub Release exists with signed Windows and signed, notarized macOS installers
- release qualification has passed
- the tag has been added to `QUALIFIED_RELEASE_TAGS` in a reviewed pull request

A Linux desktop app is not available. The Linux card offers the CLI.

## Content rules

- Product images are real screenshots of the desktop app with the synthetic workspace, captured by the `desktop-e2e` journeys. Illustrations and the interactive evidence example are labelled as such.
- No invented numbers, benchmarks, testimonials, customer logos, approvals or integrations.
- Fonts are Latin subsets of the repository's OFL Inter and JetBrains Mono NL, shipped with their licenses in `public/fonts/`.
- Publishing the website does not authorize a desktop release, real patient data or any clinical use.
