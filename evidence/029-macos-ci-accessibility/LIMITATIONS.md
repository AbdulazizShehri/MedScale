# Spec 029 LIMITATIONS

- `RELEASE_READY = FALSE` — macOS CI presence does not close the release OS-matrix bar
- `macos_qualified = false` — runners prove build/test only; not packaging, notarization, or product UX/privacy shell qualification
- Accessibility READY_BASE covers fixture/CLI keyboard-path and disclosure clarity checks only
- **No WCAG 2.x audit**, contrast measurement, or assistive-technology product qualification
- **No final v0 UI** — EXTERNAL_GATES `FINAL_V0_UI_ARTIFACT` remains user-supplied when ready
- PRIVATE_DATA_READY / MULTI_CLIENT_RELEASE_READY unchanged (FALSE)
- REAL_PHI unauthorized; synthetic-only
- macOS CI may be slower than Linux; fail-fast remains false so other OSes continue if one fails
- Homebrew openssl/perl step is defensive; if runners already provide tools, install is idempotent enough for CI
