# Limitations — Spec 058

- Portable ZIP qualification is not MSI/MSIX/DMG/pkg/deb/rpm/AppImage qualification.
- No production signing key, signature, notarization or app-store action is used.
- Bit-for-bit proof covers package assembly from identical built binaries; compiler/build reproducibility remains unproven.
- Baseline/candidate lifecycle qualification exercises package-slot state transitions; application data migration/recovery remains the separately qualified Spec 048 axis.
- Final v0 UI and macOS signed App Sandbox product qualification remain unavailable.
- `RELEASE_READY=false`.
