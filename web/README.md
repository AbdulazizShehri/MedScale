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

## Build-only website qualification — no publication authorization

After protected PR #372, `.github/workflows/website.yml` runs tests and
checks both static exports for matching pull requests and `main` pushes.
It has **no Pages upload/deploy job and no Pages write permissions**.
The `preview/medscale-site-live` branch is not a publishing trigger in this
workflow. A successful build **does not** deploy a website.

Existing public Pages content may remain online from earlier deployments;
build-only CI does not take that content offline or prove which version a
visitor receives. No new website deployment, domain cutover, or installer
distribution is authorized. Restoring deployment requires a separate,
explicit owner decision and a new protected, qualified PR.

## Historical medscale.si cutover plan — NOT AUTHORIZED

The following DNS and migration notes are historical planning material, not
instructions to execute now. Do not change DNS, GitHub Pages settings,
deployment workflows, or the published site based on this README.
Verify the current domain and deployment state independently before any
future authorized change.

### DNS to configure at the provider

The authoritative nameservers checked on 2026-10-23 were
`artemis.dns-parking.com` and `hermes.dns-parking.com` (Hostinger DNS).
Verify the active nameservers before making changes.

| Type | Host | Destination |
| --- | --- | --- |
| A | `@` | `185.199.108.153` |
| A | `@` | `185.199.109.153` |
| A | `@` | `185.199.110.153` |
| A | `@` | `185.199.111.153` |
| CNAME | `www` | `abdulazizshehri.github.io` |

Optional `AAAA` records: `2606:50c0:8000::153`,
`2606:50c0:8001::153`, `2606:50c0:8002::153`,
`2606:50c0:8003::153`. Never add duplicate/conflicting
apex A/AAAA or `www` A/AAAA records. Remove the existing parked apex
`A` value only when ready to point the website to GitHub Pages. Preserve
MX, SPF, DKIM, DMARC, and unrelated TXT records (especially email).
Do not change nameservers or use wildcard DNS records.

### Historical coordinated cutover checklist (do not execute without authorization)

1. Confirm ownership and access to the DNS panel, and coordinate the exact
   cutover window with the website maintainer. Leave the cutover PR as a draft
   until then.
2. First register `medscale.si` in repository **Settings → Pages → Custom
   domain** to claim it with GitHub before directing DNS to GitHub, as
   GitHub's official documentation recommends. This may temporarily redirect
   the old site URL, so proceed directly to the next steps.
3. Set the apex A records and `www` CNAME above at the DNS provider.
   Check authoritative DNS resolution and allow for propagation.
4. Only after separate explicit owner authorization, verified DNS/domain
   prerequisites, and a newly qualified protected PR that deliberately
   restores deployment could a root build be published. The current workflow
   remains build-only.
5. Verify `https://medscale.si/`, `/product/`, `/models/`, `/download/`,
   `/docs/`, and `/community/`, along with JS chunks, fonts, image assets,
   `sitemap.xml`, `robots.txt`, the icon and social preview. Confirm no
   links rely on the old `/MedScale/` base path.
6. After the Pages HTTPS certificate is issued, enable **Enforce HTTPS**
   and verify `www.medscale.si` redirects to the apex domain. GitHub Pages
   may take up to 24 hours to finish DNS propagation and certificate issuance.

For **custom GitHub Actions Pages deployment**, a repository `CNAME`
file is not required and is ignored; the site domain is configured in GitHub
Pages settings. The original `github.io/MedScale` URL is normally redirected
to the configured custom domain by GitHub Pages.

References:
- https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site
- https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/verifying-your-custom-domain-for-github-pages

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
