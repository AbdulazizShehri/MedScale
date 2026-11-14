# Spec 047 — Release Dry-Run + Cross-Verifier

## Delivered

- `scripts/release-dry-run.ps1` binds live `source_sha` / `tree_sha` / `cargo_lock_sha256`, build-environment identity, and admissions-derived native-deps inventory (`model_assets_included=false`).
- `scripts/verify-release-manifest.ps1` cross-checks manifest ↔ live git/lock ↔ SBOM lock property ↔ checksums lock digest; refuses `release_ready`/`signed`/`notarized` true.
- Doctor `release_dry_run_verifier_present=true`.

## Honesty

| Claim | Value |
|---|---|
| RELEASE_READY | false |
| release_sbom_native_model_assets | still missing |
| checksums_provenance_signing_verification | still missing (unsigned) |
| model_assets_included | false |

## Commands

```text
pwsh ./scripts/release-dry-run.ps1
pwsh ./scripts/verify-release-manifest.ps1
pwsh ./scripts/verify-package-checksums.ps1
```
