# Portable Release Package Contract — Spec 058

Each platform package is an unsigned ZIP containing exactly:

- `bin/medscale-cli[.exe]`
- `bin/medscale-desktop[.exe]`
- `LICENSE`
- `NOTICE.md` (the canonical third-party NOTICE inventory)
- `SBOM.cdx.json`
- `README.md`
- `package-manifest.json`

The manifest binds package version/revision, platform/architecture, source SHA, tree SHA, Cargo.lock SHA-256, and SHA-256 of every payload file except the manifest itself. ZIP entries are sorted, stored without compression and assigned the fixed ZIP epoch timestamp.

`release_ready=false`, `signed=false`, `notarized=false`, and `reproducible_binary_build=false` are mandatory.
