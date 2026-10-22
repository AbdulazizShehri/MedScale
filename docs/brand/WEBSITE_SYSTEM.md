# MedScale website system

**Status:** active for `web/` (2026-10-22). The website implements it in `web/src/app/globals.css`. Copy rules are in [`MARKETING_COPY.md`](MARKETING_COPY.md).

## Shared brand contract vs. website decisions

| Area | Shared MedScale contract (desktop, CLI, website) | Website-only decision |
|---|---|---|
| Logo | Logo 12 symbol plus the 2026-10-22 wordmark, from `assets/brand/` only. Monochrome, never recoloured, never retyped. | Header lockup at 26 px high; footer at 24 px; closing mark at 72 px. |
| Color | Black and warm white. Sky blue `#70B8C7` for interaction and support; orange `#FB905A` for attention, review, conflict and preview state. Status never relies on color alone. | Near-black page `#08090A` with three raised surfaces. Faint sky-blue radial light behind hero and closing sections. |
| Type | Inter and JetBrains Mono NL, unchanged OFL files from `assets/brand/fonts/`. | Latin WOFF2 subsets of the same files, self-hosted, with the OFL texts in `/fonts/`. No font service. |
| Voice | `VOICE_AND_COPY.md`. Evidence first. No clinical claims. | Two-tone headlines: a gray lead-in, then a white statement. |
| Imagery | Real product screenshots use the synthetic workspace only. | Screens sit in a light window frame on the dark page, with a caption naming them as real screenshots. |

## Tokens

| Token | Value | Use |
|---|---|---|
| `--bg` / `--bg-1` / `--bg-2` / `--bg-3` | `#08090A` / `#0C0D0F` / `#121316` / `#191A1D` | page, cards, raised panels, icon tiles |
| `--ink` / `--ink-2` / `--ink-3` | `#F4F4F3` / `#A4A7AC` / `#8B8F96` | primary, body, meta. `--ink-3` is at least 5.3:1 on every surface. |
| `--line` / `--line-2` / `--line-3` | white at 7.5% / 13% / 22% | dividers, borders, hover borders |
| `--blue`, `--orange` | `#70B8C7`, `#FB905A` | above 8:1 on `--bg-2` |
| Radius | 6 / 10 / 14 / 20 px, and 999 px for pills and buttons | |
| Spacing | 4 px base. Section padding is `clamp(80px, 11vw, 144px)`. Gutter is `clamp(20px, 4vw, 40px)`. Shell maximum is 1200 px. | |

## Type scale

| Role | Size | Weight | Tracking |
|---|---|---|---|
| Display (hero) | `clamp(44px, 7.2vw, 84px)` / 1.0 | 540 | −0.042em |
| Section headline | `clamp(32px, 4.4vw, 52px)` / 1.06 | 540 | −0.034em |
| Card title | 17–24 px | 560 | −0.01 to −0.02em |
| Lede | `clamp(17px, 1.6vw, 19px)` / 1.55 | 400 | 0 |
| Body | 15 px / 1.65 | 400 | 0 |
| Eyebrow, captions, data | JetBrains Mono 11–12 px, uppercase eyebrows with 0.04em tracking | 400–500 | |

## Components

- **Buttons:** pill-shaped and 44 px tall (36 px small). Primary is warm white on black. Secondary is a translucent outline. One primary per group.
- **Navigation:** sticky translucent header with a hairline after scroll. Below 860 px it becomes a full-screen menu that closes on Escape and returns focus to the toggle.
- **Product tour:** an ARIA tablist with arrow-key navigation, vertical on desktop and a scrolling pill row on mobile. Each tab shows a real screenshot and three facts.
- **Evidence example:** clearly labelled interactive illustration of the evidence states, using synthetic data.
- **Platform cards:** Windows, macOS and Linux, with the visitor's platform highlighted. The release gate in `web/src/lib/release.ts` decides whether a card can ever link to an installer.
- **Notices:** an orange-bordered panel for qualification and preview boundaries.

## Accessibility and motion

- Skip link, one `h1` per page with no skipped levels, visible `:focus-visible` rings in sky blue, and keyboard-reachable scroll regions.
- Entrance motion is limited to short fades. Hero text is never animated, so it paints at first render. All motion is disabled under `prefers-reduced-motion`.
- Verified with axe-core (WCAG 2.2 AA rules plus best practices) and scripted keyboard checks. This is automated evidence, not a conformance claim.

## Hosting

The site is a static Next.js export. `NEXT_PUBLIC_SITE_BASE_PATH` sets the path prefix (`/MedScale` for GitHub Pages, empty for a root domain). `NEXT_PUBLIC_SITE_URL` sets canonical and social URLs. See `web/README.md` for deployment and the `medscale.si` cutover.
