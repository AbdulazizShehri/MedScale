# Production signing requirements (post-v0.1.0 action)

v0.1.0 ships as an **unsigned engineering-preview release candidate**: `signing: NOT_GRANTED`, `release_ready: false`. Production signing is a separate action after the release. Nothing here is configured in the repository, and no credentials are stored in it.

## Windows

- **Certificate:** an Authenticode code-signing certificate held by the publishing organization. An EV certificate builds SmartScreen reputation immediately.
- **Signing scope:** both the installed `medscale-desktop-tauri.exe` and `MedScale_0.1.0_x64-setup.exe` (NSIS), with an RFC 3161 timestamp so signatures outlive the certificate.
- **Key custody:** a hardware token or a cloud HSM / signing service. The private key never enters the repository or a CI log.

## macOS

- **Certificate and runtime:** an Apple Developer ID Application certificate, with Hardened Runtime and the minimum entitlements WebView (WKWebView) needs.
- **Notarization:** submit the `.app`/`.dmg` to the Apple notary service, then staple the ticket to the DMG.
- **Signing scope:** sign the DMG itself as well, with a Developer ID certificate.

## Release authorization

- Signing runs only from a protected release workflow, on a tag, behind required reviewers, with secrets scoped to that environment.
- **The signed release set keeps everything the unsigned set verifies today** (`scripts/verify-tauri-release.ps1`, Spec 101):
  - `SHA256SUMS`
  - the CycloneDX SBOM
  - `NOTICE.md` and the licences
  - MPL source archives
  - a manifest pinning source, tree, lock-file, SBOM and notice digests
- **The installer qualification steps must pass on the signed artifacts:** Windows install, launch and uninstall; macOS DMG verify, mount, install, launch and uninstall.
- **Manifests:** they flip `signed` and `notarized` to true only after verification.
  - `release_ready` stays a separate, explicit decision.
  - It does not by itself authorize clinical use or real patient data. Those remain gated by clinical validation and regulatory approval (`EXTERNAL_GATES.md`).
