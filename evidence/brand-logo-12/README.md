# Logo 12 visual verification (2026-10-19)

These are rendered checks of the founder-approved Logo 12 replacement on Windows 11. Synthetic workspace only.

| File | What it shows |
|---|---|
| `reference-vs-reconstruction.png` | The approved reference (left) beside the vector canonical logo (right). Symbol IoU 0.925, wordmark IoU 0.911 against the thresholded reference. |
| `01-welcome.png` | Desktop Welcome screen: symbol and wordmark (horizontal arrangement). |
| `02-home.png` | Sidebar brand and Home header mark in the dark theme. |
| `03-about.png` | About: the large symbol and the vector wordmark. The tagline is separate from the logo. |
| `04-patients.png` | A working surface with the sidebar brand. |
| `06-sidebar-brand-4x.png` | Sidebar brand captured at 4× device scale. The wordmark weight is uniform across "Med" and "Scale". |
| `07-exe-icon-4x.png` | The 32 px icon embedded in the built Windows executable, enlarged 4× with nearest-neighbour scaling. |
| `icon-readability-sheet.png` | Icons at 16, 24, 32, 64, 128 and 256 px at 1:1 on dark and light backgrounds, plus 16/24/32/64 enlarged 4×. |
| `readme-lockup-preview.png` | The README-scale horizontal lockup on GitHub dark and light backgrounds. |

**How these were captured:**

- The desktop build was a debug build of `apps/desktop-tauri` with `custom-protocol`.
- It was launched with a WebView2 DevTools port. Navigation and screenshots were driven over the Chrome DevTools Protocol, and `devicePixelRatio` was 1.25.
- The icon was extracted from the executable with `System.Drawing.Icon.ExtractAssociatedIcon`.

**Findings:**

- At 24 px and above, the three frames read distinctly, using the optical-size masters below 128 px.
- At 16 px the icon reads as a compact frame cluster, but the individual frames are not distinct. This is a recorded platform-size limitation; see `docs/brand/LOGO_SPEC.md`.
- These renders do not qualify accessibility, and they are not a release decision.
