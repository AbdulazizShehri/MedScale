# Spec 095 native visual review

**Review state:** eight-frame native visual matrix inspected; F095-01 and F095-02 closed for the sampled frames.

**Renderable source candidate:** `7be197cd2505aa1bb0537b7bfb0fd40defe27482`, tree `e72568c074c9aa0f1e4d065036e52bdedb79a5fd`.

**CI run:** `36637843428`; all six required jobs passed, including Windows workspace tests and portable package qualification.

**Windows package ZIP SHA-256:** `aac7e1b2e854d9f2b219c4029b81a3422f6a267fbaa567584e9a571b01a96b35` (matched the CI digest sidecar).

**Packaged executable SHA-256:** `cd58993420bddd33c70cfee4819df2425ee84d55e3485a46364584b90471bcec` (matched the package manifest).

**Package source commit:** `654fca938dd4fd9421531b227f108060ed9f7c00`, the GitHub PR merge ref; its source tree matches the branch candidate tree above.
**Capture method:** actual shown native Windows application through Slint `Window::take_snapshot`, `SLINT_SCALE_FACTOR=1`, one process per frame, fresh nonsynced synthetic vault. Each PNG has adjacent JSON binding source tree, candidate head, package executable digest, actual pixels, renderer/platform, route/theme/density and its own PNG digest. The JSON deliberately leaves `visual_inspection_complete=false`; this human review supplies the visual findings.

## Required rendered matrix

| Route | Theme | Size | Density | Frame | Review |
|---|---|---|---|---|---|
| Home | light | 1440×900 | standard | `captures/home-light-1440x900.png` | PASS — full sampled Home sections visible; exact subject/state/authority copy readable |
| Home | dark | 1440×900 | standard | `captures/home-dark-1440x900.png` | PASS — same hierarchy with white mark and readable neutral contrasts |
| Home | light | 1100×720 | standard | `captures/home-light-1100x720.png` | PASS — prior status/source collision absent; visible top content fits |
| Home | dark | 1100×720 | standard | `captures/home-dark-1100x720.png` | PASS — prior collision absent; sampled status text and controls legible |
| Home | light | 1100×720 | compact | `captures/home-light-1100x720-compact.png` | PASS — compact spacing preserves full visible labels and status markers |
| Home | dark | 1100×720 | compact | `captures/home-dark-1100x720-compact.png` | PASS — compact dark content has no sampled overlap/clipping |
| About | light | 1100×720 | standard | `captures/about-light-1100x720.png` | PASS — MedScale leads; toolkit attribution/version visible |
| About | dark | 1100×720 | standard | `captures/about-dark-1100x720.png` | PASS — white mark, reduced Slint plaque and themed toolkit copy readable |

## Findings to close with after-renders

- **F095-01 Home status collision — CLOSED in the sampled matrix.** The six-job-green prior candidate `a824e5f` visibly overlapped the partial badge and source-count copy at 1100×720 and stretched other state badges like inputs. The original frame/JSON are preserved in `findings/`. Forward fixes `b1a52f3` and `8ceb520` stack the source-count line, bound status widths and remove repeated label words while retaining literal state names and non-color markers. The after frames show no collision in either theme or density at 1100×720; partial, unknown and unmeasured labels are fully visible.
- **F095-02 About hierarchy and attribution contrast — CLOSED in the sampled matrix.** The same prior candidate rendered a Slint graphic larger than the MedScale identity and nearly black toolkit detail on the dark background. The original frame/JSON are preserved in `findings/`. `7be197c` reduces the card, gives MedScale the top hierarchy and places the built-in Slint attribution/link in a smaller white plaque with separate themed version text. Both after frames make product name, license line and toolkit/version text readable. The built-in plaque's internal version/link remains small, but the adjacent themed text supplies legible attribution; link activation is not measured by a still frame.

## Acceptance observations to record from pixels

- Approved paired M is black on light and white on dark, with the same two-peak/central-contact/lower-void silhouette. The 1440 and 1100 in-product marks remain visibly legible; the SVG numeric and minimum-size guards are separate evidence.
- Inter heading/body and JetBrains Mono NL subject metadata render in the sampled frames with readable hierarchy. The visible subject ID, date/version and state marker glyphs are legible; this does not qualify every scientific glyph or table.
- Home shows the explicitly synthetic subject Sarah Chen, `synthetic-patient-sarah-chen`, type 2 diabetes mellitus, three source records, zero admitted Packs and a clear proposal-versus-authority limitation. It does not infer evidence or clinical readiness from relevance.
- Sampled partial, unknown and unmeasured badges retain literal state names plus distinct ASCII markers in both themes. The other eight defined states are covered by deterministic mapping guards, not by this rendered matrix.
- At 1100×720 the sampled top content has no overlap or clipped essential label in standard/compact. The bottom Projects/Data/Research OS actions are below the 1100 viewport; a visible scroll bar and the 1440 frames show content exists, but this review does not assert physical scroll operation or below-fold keyboard access.
- About's MedScale hierarchy, Apache-2.0 line and Slint attribution are readable in both themes. The third-party Slint graphic retains its own color inside a clearly separate toolkit plaque; the MedScale mark/wordmark remain monochrome.

## What this review measures

The final record will cover one Windows native renderer at the listed sizes/themes/densities, package/source/binary binding, and directly inspected visible pixels. Separate deterministic tests and CI cover state/asset and build invariants. Still frames do not establish physical keyboard flow, focus order, link activation, screen reader/UI Automation behavior, WCAG conformance, OS/text enlargement, cross-platform pixels, every route/state, clinical/model quality or release/privacy/multi-client readiness. `RELEASE_READY=false`, `PRIVATE_DATA_READY=false`, `MULTI_CLIENT_RELEASE_READY=false`.
