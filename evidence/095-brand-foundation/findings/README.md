# Native findings preserved before forward fixes

These are actual Slint `Window::take_snapshot` frames from the qualified Windows portable executable built in CI run `36618933148` (all six required jobs passed), branch candidate `a824e5f518ee656072d12c73d6c24eea23e80d39`, source tree `7b85d45a4fa3e25eece77cf45fb7d27222611392`, packaged binary SHA-256 `9b0a2b9e9f124ff33b86d6096d9f512bdf71ef2f85c41d52afb16dcd659887e4`. Both are 1100×720, scale 1, Windows 11, with isolated synthetic-only runtime state. The adjacent JSON records are preserved without changing their capture-time `visual_inspection_complete=false`; inspection is recorded separately.

| ID | Before frame | Observed issue | Forward source fix |
|---|---|---|---|
| F095-01 | `before-home-light-1100x720.png` | The inline partial badge overlapped the source-count copy at the minimum width; other status badges stretched like fields and repeated state words. | `b1a52f3`, `8ceb520`: stack the badge/copy, bound badge widths and remove repeated label words while retaining exact states and markers. |
| F095-02 | `before-about-dark-1100x720.png` | The large built-in Slint graphic dominated MedScale's About identity; its small text/link were nearly black on the dark surface. | `7be197c`: place attribution in a smaller white plaque, add themed toolkit/version copy and retain the built-in link. |

The after-renders and full light/dark review belong in `../NATIVE_VISUAL_REVIEW.md`. These before frames establish the visible defects, not acceptance of the fixes.
