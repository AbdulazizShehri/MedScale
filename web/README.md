# MedScale website (Next.js)

Public, responsive product and open-source website, built with Next.js App Router, TypeScript and static export. No backend or paid hosting is required. The site is intentionally distinct from the MedScale clinical application.

## Local development

Run from \`web/\`:

\`\`\`sh
npm ci
npm run dev
npm run build
npm run check
\`\`\`

\`next build\` exports static files to \`web/out/\`. A **GitHub project Pages** deployment is built with \`NEXT_PUBLIC_SITE_BASE_PATH=/MedScale\`; a future apex-domain deployment to \`medscale.si\` is built with that variable unset. Do not configure DNS or claim the domain is live until the user authorizes cutover and HTTPS verification succeeds.

## Download integrity

The download cards show actual supported engineering surfaces and **never fabricate binary URLs**. At initial launch, no signed production GitHub Releases are published; Windows and macOS direct-download buttons therefore do not exist. The Linux desktop remains deferred, though Core/CLI builds are available.

To enable future direct binaries: add a reviewed, source-controlled versioned manifest of verified signed release artifact URLs, hashes, architectures, and provenance; run release-qualification checks, then change the site content only after the GitHub Release and assets genuinely exist.

## Design and provenance

- Reuses approved first-party Logo 12 SVG masters in \`web/public/brand\`. No external fonts, tracking, marketing data, or copied product assets.
- Experience patterns inspired by high-level UX study of public medical open-source sites and Mobbin: strong product hero, inspectable example, feature discovery, platform download cards, community/documentation paths.
- All mock product panes clearly say illustration/synthetic. No fictional benchmarks, release counts, clinical approvals, model endorsements, testimonials, or patient data.
- Page claims and actual qualification gates come from root \`PRODUCT.md\`, \`README.md\`, and \`docs/qualification/EXTERNAL_GATES.md\`.
- Publication of the website is **not** authorization to release a desktop product, handle real PHI, or connect live partners.
