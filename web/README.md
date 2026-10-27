# MedScale website

The public website for MedScale Beta, an open-source product in active development. It is a Next.js App Router site with TypeScript and a static export, so it needs no backend and no paid hosting. The website is separate from the MedScale desktop app and never connects to it or to any clinical system.

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

## medscale.si cutover — staged, not activated

The live website currently runs at `https://abdulazizshehri.github.io/MedScale/`.
The cutover PR changes the `main` Pages deployment to **publish the root-domain
build last**, with `NEXT_PUBLIC_SITE_BASE_PATH=""` and
`NEXT_PUBLIC_SITE_URL=https://medscale.si`. It keeps the /MedScale
build-and-check gate as a regression test.

**Do not merge the cutover PR until the actual domain switch is coordinated**:
deploying a root build to the existing project URL before configuring the
GitHub Pages custom domain can break navigation and assets.

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

### Coordinated cutover checklist

1. Confirm ownership and access to the DNS panel, and coordinate the exact
   cutover window with the website maintainer. Leave the cutover PR as a draft
   until then.
2. First register `medscale.si` in repository **Settings → Pages → Custom
   domain** to claim it with GitHub before directing DNS to GitHub, as
   GitHub's official documentation recommends. This may temporarily redirect
   the old site URL, so proceed directly to the next steps.
3. Set the apex A records and `www` CNAME above at the DNS provider.
   Check authoritative DNS resolution and allow for propagation.
4. After the GitHub Pages domain is registered and DNS is resolving to
   GitHub, merge the cutover PR through the required branch-protection checks.
   The main workflow then publishes the **root** build.
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

### Unsigned synthetic-only engineering preview (no paid developer accounts)

The desktop CI already builds and probes Windows NSIS `.exe` and macOS `.dmg` installers. A **separate and explicitly unqualified** distribution track is staged in `.github/workflows/unsigned-engineering-preview.yml`:

1. Wait for a successful **push-to-main** `tauri-preview` workflow run at the exact current main SHA, including native installer lifecycle tests.
2. Manually dispatch **Publish unsigned synthetic-only preview** from `main`, with the successful workflow run ID and the exact confirmation phrase `PUBLISH_UNSIGNED_SYNTHETIC_PREVIEW`.
3. The workflow independently checks that both sets came from the exact same main commit, verifies manifests and SHA256 digests, requires Windows/macOS installer install/launch/uninstall PASS receipts, and refuses tampered/missing artifacts or duplicate tags.
4. Only after those checks, GitHub hosts a **pre-release** containing the unsigned Windows EXE, unnotarized macOS DMG, and checksum file. Visitors can download it without signing in to GitHub. The website dynamically discovers only those exact GitHub pre-release assets, and shows no preview button until one actually exists.

**MedScale Beta is under active development. This is not the signed/public-production release channel.** `QUALIFIED_RELEASE_TAGS` stays empty, `RELEASE_READY=false`, and real PHI is forbidden. Do not distribute preview binaries for clinical decisions or real patient data.

**Windows:** Download the EXE directly; Windows Defender SmartScreen and Smart App Control may warn or block unsigned software. Do not suppress system policies programmatically. Require checksum verification and a trusted official download URL.

**macOS:** Download the DMG, copy the app to Applications, and open it. Gatekeeper can block software without Developer ID notarization. If the user has independently verified the official source and checksum, macOS may offer an explicit one-app exception under System Settings → Privacy & Security → Open Anyway. Never instruct users to disable Gatekeeper globally.

**No Microsoft or Apple paid developer membership is required to *build and host* this engineering preview**, but equally frictionless installs on every Windows/macOS device cannot be guaranteed without trusted signing. GitHub Actions artifacts are for authenticated repo users; the separate prerelease is the public-download route.

The website's actual signed/qualified production installer gate is deliberately stricter:
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
