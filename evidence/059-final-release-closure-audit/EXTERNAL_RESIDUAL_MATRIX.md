# External Release Residual Matrix

After repository-owned material-findings clearance, the release doctor must retain exactly these external residuals:

| Doctor evidence class | External gate | Why it cannot be closed in-repo now |
|---|---|---|
| `macos_platform_product_qualification` | `MACOS_SIGNED_PRODUCT_QUALIFICATION` | The final native shell exists; production Developer ID signing/notarization and measured App Sandbox enforcement on the bound signed product remain external. |
| `release_sbom_signing_provenance` | `DESKTOP_RELEASE_SIGNING_PROVENANCE` | Requires owner-provisioned production signing identity/provenance. |
| `perf_budgets_attained_on_qualified_hardware` | `QUALIFIED_RELEASE_PERFORMANCE_HARDWARE` | Requires owner-declared qualified Windows/Linux/macOS hardware and the Spec 067 final-UI measurement packet; hosted CI is not qualified hardware. |
| `checksums_provenance_signing_verification` | `DESKTOP_RELEASE_SIGNING_PROVENANCE` | Verification path exists; real signature identity is not granted. |
| `wcag_final_v0_ui_accessibility_qualification` | `FINAL_V0_UI_ACCESSIBILITY_QUALIFICATION` | The final native UI exists; live keyboard, VoiceOver/NVDA/Orca, reflow, rendered-contrast, focus, and recovery-state qualification on the bound final package remains external. |

MESC is optional and absent MESC assets do not appear in this release residual set.
