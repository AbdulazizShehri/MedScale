# MedScale website system

**Status:** active for `web/` (2026-10-22). The website implements it in `web/src/app/globals.css`. Copy rules are in [`MARKETING_COPY.md`](MARKETING_COPY.md).

## Shared brand contract vs. website decisions

| Area | Shared MedScale contract (desktop, CLI, website) | Website-only decision |
|---|---|---|
| Logo | The 2026-11-05 symbol plus the 2026-10-22 wordmark, from `assets/brand/` only. Monochrome, never recoloured, never retyped. | Header lockup at 18 px high; footer at 17 px; closing mark at 40 px. No opacity or effects on the logo. |
| Color | Black and warm white. Sky blue `#70B8C7` for interaction and support; orange `#FB905A` for attention, review, conflict and preview state. Status never relies on color alone. | Near-black page `#08090A` with three raised surfaces. No glows or gradients: surfaces are flat and hairlines separate them. |
| Type | Inter and JetBrains Mono NL, unchanged OFL files from `assets/brand/fonts/`. | Latin WOFF2 subsets of the same files, self-hosted, with the OFL texts in `/fonts/`. No font service. |
| Voice | `VOICE_AND_COPY.md`. Evidence first. No clinical claims. | Two-tone headlines: a gray lead-in, then a white statement. |
| Imagery | Real product screenshots use the synthetic workspace only. | Screens sit in a single hairline with one radius, unaltered: no window chrome, overlays or fades. A caption names them as real screenshots. |

## Tokens

| Token | Value | Use |
|---|---|---|
| `--bg` / `--bg-1` / `--bg-2` / `--bg-3` | `#08090A` / `#0C0D0F` / `#121316` / `#191A1D` | page, cards, raised panels, icon tiles |
| `--ink` / `--ink-2` / `--ink-3` | `#F4F4F3` / `#A4A7AC` / `#8B8F96` | primary, body, meta. `--ink-3` is at least 5.3:1 on every surface. |
| `--line` / `--line-2` / `--line-3` | white at 7.5% / 13% / 22% | dividers, panel borders (`--panel-line`), hover borders |
| `--blue`, `--orange` | `#70B8C7`, `#FB905A` | above 8:1 on `--bg-2` |
| Radius | 14 px for panels, 10 px for anything nested in a panel, 6 px for chips, and 999 px for pills and buttons | |
| Spacing | 4 px base. `--section-y` `clamp(72px, 9vw, 120px)` around sections, `--head-y` `clamp(40px, 5vw, 64px)` from a section head to its content, `--stack` 20 px inside a block of copy, `--stack-lg` 24 px in hero copy and between blocks, `--pad` 24 px inside panels. Gutter is `clamp(20px, 4vw, 40px)`. Shell maximum is 1200 px. | |

## Layout grid

Every multi-column layout sits on one 12-column grid with a 24 px gap, defined by `--col` and `--span-*` in `globals.css`. Don't add a new column ratio.

- **One left axis.** Headings, body text, panels and footers start on the shell's left edge. Nothing is centred at section level.
- **Text on the axis, not inside a tint.** Eyebrows have no leading mark. Tour tabs and model states are marked by a rule above them, so their labels start on column lines. The first header nav label sits on column line 3.
- **Splits:** text and media use 5 + 7 columns, always with the copy on the left; section heads and the evidence panel use 7 + 5; the tour uses 3 + 9; the footer uses 6 + 2 + 2 + 2. The evidence panel's right-hand text lands on column line 8, under the section head's paragraph.
- **Repeats:** three-up rows span 4 columns each and four-up rows span 3. Cards, pillars, states, strip items and tour points use `grid-template-rows: subgrid`, so titles and text share lines across a row even when one title wraps.
- **Tables:** column text lands on column lines (3 + 4 + 5 on Models, 3 + 2 + 2 + 5 on Download). Table cells ignore `calc()` with percentages, so these widths use container units.
- **Buttons in cards:** the button comes last in every platform card, including one that shows a checksum, so buttons line up across a row.
- **Baselines:** in a split section head, the headline's last line and the right column's last line share a baseline (`align-items: last baseline`).
- **Column caps go inside `.shell`.** Never put `max-width: var(--span-*)` or any narrower width on `.shell` itself: the percentage resolves against the full-bleed section and the shell's auto margins centre it off the axis.
- **Panels:** one radius (14 px) and 24 px padding. In a row of cards, buttons line up along the bottom.
- **Rhythm:** every section uses `--section-y` top and bottom, and every hero uses it on top.
- **Below 860 px:** two-column repeats stay on the grid (6 + 6, or 4 + 4 + 4 in the footer) and column caps are released.

## Type scale

| Role | Size | Weight | Tracking |
|---|---|---|---|
| Display (home hero) | `clamp(42px, 6.8vw, 80px)` / 1.02 | 600 | −0.032em |
| Display (subpage hero, closing) | `clamp(40px, 6vw, 72px)` / 1.02 | 600 | −0.032em |
| Section headline | `clamp(30px, 4.1vw, 48px)` / 1.08, or `clamp(28px, 3.4vw, 40px)` on subpages | 600 | −0.026em |
| Panel title | 20 px in Plus Jakarta Sans, 17 px in Inter for compact cards | 600 / 500 | −0.015em / −0.01em |
| Lede | `clamp(17px, 1.6vw, 19px)` / 1.55 | 400 | 0 |
| Body | 15 px / 1.65 | 400 | 0 |
| Eyebrow, captions, data | JetBrains Mono 12 px, uppercase eyebrows with 0.04em tracking | 400 | |

Inter's `cv11` and `ss01` features apply to Inter only; display type turns them off. Text uses 12, 13, 14, 15, 17, 19, 20 and 24 px and the weights 400, 500 and 600. Don't add in-between values. Labels inside the home page's small illustrations may use 11 px.

## Components

- **Buttons:** pill-shaped and 44 px tall (36 px small with a pointer, 44 px at touch widths). Primary is warm white on black. Secondary is a translucent outline. One primary per group.
- **Effects:** panels are flat. The only shadow is the screenshot's hairline, the only blur is the sticky header, and the page has no gradients.
- **Navigation:** sticky translucent header with a hairline after scroll. Below 860 px it becomes a full-screen menu that closes on Escape and returns focus to the toggle.
- **Product tour:** an ARIA tablist with arrow-key navigation, a ruled list on the left axis on desktop and a scrolling pill row on mobile. Each tab shows a real screenshot and three facts.
- **Evidence example:** clearly labelled interactive illustration of the evidence states, using synthetic data.
- **Platform cards:** Windows, macOS and Linux, with the visitor's platform highlighted. The release gate in `web/src/lib/release.ts` decides whether a card can ever link to an installer.
- **Notices:** a flat, orange-bordered panel for qualification and preview boundaries.

## Accessibility and motion

- Skip link, one `h1` per page with no skipped levels, visible `:focus-visible` rings in sky blue, and keyboard-reachable scroll regions.
- Entrance motion is limited to short opacity fades. Hero text is never animated, so it paints at first render. All motion is disabled under `prefers-reduced-motion`.
- Verified with axe-core (WCAG 2.2 AA rules plus best practices) and scripted keyboard checks. This is automated evidence, not a conformance claim.

## Hosting

The site is a static Next.js export. `NEXT_PUBLIC_SITE_BASE_PATH` sets the path prefix (`/MedScale` for GitHub Pages, empty for a root domain). `NEXT_PUBLIC_SITE_URL` sets canonical and social URLs. See `web/README.md` for deployment and the `medscale.si` cutover.
