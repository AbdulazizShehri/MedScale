# Windows native preview inspection

## Identity and limits

Source head: `76e9fe31c2611f1abc02a9575641988e27784486`

Source tree: `cd7251fdbc142ba61de1ccc1a4b1993ff6935df0`

Native CI: [36778619937](https://github.com/TheHalfMoon/MedScale/actions/runs/36778619937)

Windows job: `110102763437`

Executable SHA-256: `9f006846d4ff998bbbbbd75e6b18ae81b6bfefa6f16dfcbb94ac44f1fa2822cb`

The downloaded Windows package's 1,027 inventory entries were checked against actual file sizes and SHA-256 hashes, with zero mismatches. All five admitted MPL source archives and the complete 16,726-byte MPL license are present. The package is an **UNQUALIFIED_PREVIEW**. Linux, Windows and macOS native build/package jobs succeeded; the separate dependency policy job failed on the recorded glib advisory. No security, privacy or release PASS follows from the builds.

The executable was launched directly from that verified package. The supported Computer Use `@oai/sky` API selected its returned process/window and captured the foreground **MedScale Preview** window. A first background capture showed an occluding app and was discarded; it is not evidence. No browser viewport emulation, injected UI, development server, mocked response or edited screenshot supplied these native frames.

The tool returns JPEG captures. Their original bytes are retained alongside PNG exports made by System.Drawing decode/encode, with no crop, resize or retouch. PNG exports retain JPEG capture loss and are not raw framebuffer or pixel-equivalence evidence. The tool's pointer halo is visible in some frames. `capture-metadata.json` binds every original and PNG identity, capture time, dimensions and local capture platform. `build-receipt.json` preserves the CI receipt's fields, with only CRLF-to-LF export normalization recorded by original/export digests in capture metadata. Its original `NOT_CAPTURED` field describes packaging time and is not overwritten by this later inspection.

## Frames

| View | Tauri client target | Captured native frame | File |
|---|---|---|---|
| Home, dark | 1440 × 900 | 1442 × 931, including native chrome | [PNG](home-dark-1440x900.png) |
| Home, light | 1440 × 900 | 1442 × 931, including native chrome | [PNG](home-light-1440x900.png) |
| Home, light | 1100 × 720 | 1102 × 751, including native chrome | [PNG](home-light-1100x720.png) |
| Home, dark | 1100 × 720 | 1102 × 751, including native chrome | [PNG](home-dark-1100x720.png) |
| Palette, dark, Patients selected | 1100 × 720 | 1102 × 751 | [PNG](palette-dark-1100x720.png) |
| Patients unavailable, dark | 1100 × 720 | 1102 × 751 | [PNG](patients-unavailable-dark-1100x720.png) |

Client targets are bound to the initial window configuration and the configured minimum reached through the Windows sizing menu. Native capture adds 2 px horizontally and 31 px vertically. This is not an independent OS client-rectangle measurement.

## Observations and comparison

- Approved paired-M geometry, first-party geometric route icons, local Inter and JetBrains Mono NL render in both themes. Dark uses layered near-black surfaces; light has its own white/neutral hierarchy. The app retains one sidebar and compact native identity.
- Home's actual status is unavailable, with no invented patient or evidence rows. The compact four-row availability table remains readable at both target sizes. At the smaller size, additional work-area links require scrolling; the main region and route list scroll independently while theme controls and synthetic status remain visible.
- The native accessibility tree exposed all 25 named route buttons, the appearance group, the labeled availability table and the palette dialog. This is an inventory observation, not a screen-reader or all-route interaction qualification.
- Ctrl+K opened the native dialog. Arrow Down visibly selected Patients; Enter opened the Patients unavailable view. The underlying regions were absent from the palette accessibility subtree. Shift+Tab wrapped from the search input to Settings, and Tab returned to the input. Escape dismissed the dialog and restored the visible focus ring to the originating Dark theme control. The UI Automation focused-element field only identified the WebView document, so semantic focus assertions are limited to the visible ring and observed behavior.
- The supplied brand boards were the visual comparison authority. No canonical Figma golden target or Claude Design output was available; no pixel match, independent native design-review PASS, clinical route parity, macOS/Linux visual acceptance, WCAG conformance or privacy qualification is claimed.

This receipt advances Windows native evidence only. Current-head qualification must account for subsequent changes; the images always remain bound to their original head/tree/executable.
