# Spec 049 — Package Upgrade/Rollback Dry-Run Scaffold

## Delivered

- `scripts/verify-package-upgrade-rollback.ps1` checks unsigned baseline manifest/checksums + optional candidate dry-run lock continuity.
- Doctor `package_upgrade_rollback_scaffold_present=true`.
- Missing class `release_package_upgrade_rollback_proof` retained (no real installers).

## Honesty

| Claim | Value |
|---|---|
| RELEASE_READY | false |
| Real package upgrade/rollback | not proven |
| Scaffold present | true |
