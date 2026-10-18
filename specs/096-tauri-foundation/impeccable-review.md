Method: dual-agent (A: `/root/tauri_design_assessment`; B: `/root/tauri_detector_assessment`)

# Preparatory presentation review

**Reviewed head:** `342dd2800d8056a98175c6b9490c62b1e5ed54d3`, before the following repair commit.

**Framework:** pbakaus/impeccable revision `0d6b47ea19b63afe15e3f93a44d5d9fbbc6fd275`, skill 4.4.0, official cached CLI 0.1.8. Context loaded once against `apps/desktop-tauri/src/App.tsx` and the current PRODUCT/DESIGN files. Critique target slug: `apps-desktop-tauri-src-app-tsx`; no ignore list was present. Assessment A finished before Assessment B findings entered synthesis. Neither assessment edited the implementation or saw the other's findings.

This is a source/development-browser critique, not native visual acceptance, independent provider review, WCAG evidence or release qualification. Native frames, Figma comparison and required Jev/Alibaba OCR review remain open.

## Design health

| Heuristic | Score / 4 | Observed issue |
|---|---:|---|
| Visibility of system status | 3 | Keyboard-selected results can leave the viewport |
| Match with real world | 2 | Core/bridge/posture vocabulary needs translation |
| User control and freedom | 3 | Escape restores focus; compact recovery navigation needed |
| Consistency and standards | 3 | Windows displays a Mac-only shortcut caption |
| Error prevention | 3 | Bounded route-only surface; unavailable capabilities stay explicit |
| Recognition rather than recall | 3 | Large route catalog increases scanning |
| Flexibility and efficiency | 2 | Selection does not scroll into view |
| Aesthetic and minimalist design | 3 | Secondary route heading and empty pane are oversized |
| Error recognition and recovery | 2 | Intended preview boundary resembles a user connection problem |
| Help and documentation | 1 | Preview scope lacks explanation at the failure point |

Total: **25/40**, a heuristic assessment of the pre-repair scaffold. It is not an acceptance score. Cognitive load was moderate: route grouping helps, but chunking, minimal choices and progressive disclosure need work. Clinical route promises followed by repeated connection errors create uncertainty for first-time users. Literal unavailable/unknown states, approved identity and Home's compact table/list were strengths.

## Dispositions and repairs

| Priority | Finding | Classification | Forward repair |
|---|---|---|---|
| P1 | Arrow selection can be invisible at 1100×720 | ACCEPT | Stable option IDs, active descendant and selected-option scrolling |
| P2 | Informational status labels are 7–10 px and visually weak | ADAPT_TO_MEDSCALE | Main state/table labels increased to 11 px; stronger neutral muted tokens |
| P2 | Secondary routes use a large editorial heading and 315 px empty pane | ADAPT_TO_MEDSCALE | Compact heading/status section; unused hero/card styles removed |
| P2 | Preview unavailability sounds like a recoverable user connection error | ACCEPT | Explain navigation/appearance scope and provide a working Return to Home action |
| P2 | Blank search arbitrarily shows the first 12 equal options | ADAPT_TO_MEDSCALE | Five deliberate entry routes; typing searches all 25 |
| P2 | Shortcut caption names Cmd only on Windows | ACCEPT | Platform-aware Ctrl/Cmd caption; navigation-only search label |
| P2 | Tab-focus and Enter can target different options | ACCEPT | Focus synchronizes selected option; parent interaction inspection found this additional issue |
| P2 | Modal background is not inert; input focus styling is overridden | ACCEPT | Background inert while open; explicit visible input focus |
| — | Alternate fonts, blue/teal branding, gradients or a replacement mark | REJECT_BRAND_CONFLICT | Preserve locked first-party identity, local Inter and JetBrains Mono NL |
| — | Fake records, authentication, model dashboards or live actions | OUT_OF_SCOPE | No invented data or privileged capability added as a visual remedy |

Power users were affected by invisible selection, shortcut ambiguity and excessive route spacing. Keyboard/low-vision users were affected by tiny status text and absent active-descendant association. New users lacked a clear explanation of preview scope. The repairs preserve every route and the single sidebar.

## Mechanical and browser evidence

Assessment B ran the source detector once against App.tsx: exit 0, complete JSON `[]`, zero rules/locations/false positives. Raw output was retained in the root checkout's ignored `target/productization/spec-096-impeccable-assessment-b-detector.json`. This result covers the pre-repair source only; no rerun or clean-native inference is made.

Assessment A inspected Home in both themes at 1440×900 and 1100×720, Patients in dark at both sizes, and the palette. Assessment B independently inspected a fresh hidden browser tab at 1280×720: all 25 route buttons were present; navigation/main content scrolled independently; Settings search/Enter and Ctrl+K/Escape restored trigger focus. Both closed their temporary tabs; viewport overrides were reset. Mutable evaluate/injection was unsupported by the browser tool, so no live detector overlay or overlay server ran. Absence of `impeccable` console logs is not a browser-detector PASS.

The parent performed one bounded repair confirmation at 1100×720: Tab twice then Enter opened Patients; the modal exposed only its own accessibility tree; a broad query and nine Arrow Down presses selected Models, scrolled the result into view and linked the combobox's active descendant to its stable ID. Measured lower bounds differed by less than 0.001 px due to browser floating-point geometry, and the screenshot showed the full selected row. Escape restored the route-search trigger. Native WebView and assistive-technology verification remain open.

Questions skipped: the founder already specified the brand, staged migration and autonomous ordinary implementation authority; the critique was integrated into that implementation rather than opening a new design interview. This review does not substitute for Jev or Alibaba Open Code Review.

## Subsequent Windows native inspection

Actual 76e9fe3 Windows package frames and bounded native interaction observations are now retained in `evidence/096-tauri-foundation/native/76e9fe3`. They confirm rendering in both themes at both targets and native palette selection, Enter, Tab wrap and Escape/focus return. This subsequent parent inspection does not change Assessment A's historical score, rerun Assessment B, or establish independent provider/native design acceptance. The original browser critique's scope remains intact.
