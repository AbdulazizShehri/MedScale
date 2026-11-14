# Signing / provenance / notarization prep packet (Spec 039)

**Gate:** `APP_STORE_SIGNING_RELEASE` / general release signing credentials = not granted  
**Cursor must not** create fake identities, store production secrets, or publish signed releases without authority.

## What Cursor already completed

- Unsigned package checksum scaffold + verify script (`scripts/generate-package-checksums.ps1`, `scripts/verify-package-checksums.ps1`)
- SBOM scaffold (`scripts/generate-sbom-scaffold.ps1`) + Spec **046** Cargo.lock binding
- Spec **047** release dry-run + cross-verifier (`scripts/release-dry-run.ps1`, `scripts/verify-release-manifest.ps1`)
- Required-check inventory for CI (Spec 037)
- Public project license is `Apache-2.0` (founder decision 2026-09-14); NOTICE is included in qualified portable packages

## Per-platform prep (no credentials)

### Windows

| Field | Value |
|---|---|
| ARTIFACT_TO_SIGN | Spec 058 qualified unsigned portable ZIP; MSI/MSIX remain optional future native formats |
| SIGNING_STAGE | After qualified portable package assembly; before public distribution |
| EXPECTED_IDENTITY | Owner-chosen Authenticode certificate (TBD) |
| VERIFICATION_COMMAND | `Get-AuthenticodeSignature <artifact>` → Status Valid |
| NOTARIZATION_STAGE | N/A (Windows) |
| RELEASE_BINDING | Digest in release manifest + CI source/tree/lock SHAs |
| FAILURE_BEHAVIOR | Fail closed — do not publish unsigned as “signed” |

### macOS

| Field | Value |
|---|---|
| ARTIFACT_TO_SIGN | Future .app / .dmg / .pkg |
| SIGNING_STAGE | codesign after package; notarize before wide distribution |
| EXPECTED_IDENTITY | Developer ID Application (TBD); Team ID (TBD) |
| VERIFICATION_COMMAND | `codesign --verify --deep --strict`; `spctl --assess` |
| NOTARIZATION_STAGE | `notarytool submit` + staple (credentials required) |
| RELEASE_BINDING | Same digest/manifest binding |
| FAILURE_BEHAVIOR | Fail closed; never ship with ad-hoc identity as release |

### Linux

| Field | Value |
|---|---|
| ARTIFACT_TO_SIGN | Spec 058 qualified portable ZIP/checksum set; .deb/.rpm/AppImage/tarball remain optional future native formats |
| SIGNING_STAGE | Detached signature (sigstore/cosign or GPG) after package |
| EXPECTED_IDENTITY | Owner-chosen signing key (TBD) |
| VERIFICATION_COMMAND | `cosign verify-blob` / `gpg --verify` (chosen later) |
| NOTARIZATION_STAGE | N/A |
| RELEASE_BINDING | Digest + SBOM + provenance statement |
| FAILURE_BEHAVIOR | Fail closed |

## Exact external action

```text
GATE = DESKTOP_RELEASE_SIGNING_PROVENANCE / release signing identity
EXACT_EXTERNAL_ACTION = Provision signing identities + notarization credentials; document Team ID / cert thumbprints in a private owner store (not this repo)
REQUIRED_INPUTS = certs/keys; Apple notarization credentials if macOS public distro; store accounts if app stores
EXPECTED_OUTPUT = documented identity fingerprints + successful dry-run sign on one artifact
HOW_CURSOR_WILL_VERIFY = verification commands above on a release candidate artifact; doctor/release checklist row flips only with evidence
WHAT_UNIT_UNBLOCKS = signed release candidate packaging — still not automatic RELEASE_READY
```

## Non-claims

- No production keys in repository
- No notarization performed
- `RELEASE_READY` remains FALSE
- Qualified portable package checksums and release SBOM remain unsigned pending the external signing identity
