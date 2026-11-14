# Evidence LIMITATIONS — Spec 049

- Scaffold only: no MSI/MSIX/DMG/deb/AppImage installers.
- Does not clear `release_package_upgrade_rollback_proof`.
- Rollback procedure is re-verify of baseline digests, not package-manager undo.
- `RELEASE_READY` remains FALSE.
