# Spec 072 Release Residual Audit

The rebuilt product does not change the canonical release-readiness boundary. Repository-owned requalification must preserve exactly the external release evidence classes returned by the release doctor:

| Missing evidence class | External gate | Rebuilt-product action |
|---|---|---|
| `macos_platform_product_qualification` | `MACOS_SIGNED_PRODUCT_QUALIFICATION` | Bind the rebuilt package to production Developer ID/notarization and measured App Sandbox/product qualification. |
| `release_sbom_signing_provenance` | `DESKTOP_RELEASE_SIGNING_PROVENANCE` | Sign the rebuilt release SBOM/provenance with owner-provisioned production identity. |
| `perf_budgets_attained_on_qualified_hardware` | `QUALIFIED_RELEASE_PERFORMANCE_HARDWARE` | Execute the Spec 072 rebuilt-product performance packet on declared qualified Windows/Linux/macOS hardware. |
| `checksums_provenance_signing_verification` | `DESKTOP_RELEASE_SIGNING_PROVENANCE` | Sign and verify rebuilt-package checksums/provenance with the authorized production identity. |
| `wcag_final_v0_ui_accessibility_qualification` | `FINAL_V0_UI_ACCESSIBILITY_QUALIFICATION` | Execute the Spec 072 rebuilt-product accessibility packet with keyboard, VoiceOver/NVDA/Orca, 200% reflow, rendered contrast/focus, and recovery-state observations. |

MESC is a separate project and is excluded from MedScale release residuals, external gates, completion axes, and execution scope. Real PHI, partner credentials, terminology rights, private-data OS evidence, and platform-qualified sandbox evidence remain separate authority/readiness axes and must not be silently inferred from product requalification.
