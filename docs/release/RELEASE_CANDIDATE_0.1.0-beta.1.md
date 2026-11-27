# MedScale 0.1.0-beta.1 release candidate

**Status: candidate, not published.** Built from commit `80c67d2dfae139f7471f7f414c9cd112d8a5b07e` in [run 45916049004](https://github.com/AbdulazizShehri/MedScale/actions/runs/45916049004) (artifact `medscale-release-candidate-0.1.0-beta.1`, ID `15164819520`, 471,748,741 bytes, expires `2027-02-25T00:19:31Z`).
Every file below passed its platform installation or execution check in that run, a ClamAV 1.5.4 scan with fresh signatures (`34` files scanned, `0` infected), a `sha256sum -c SHA256SUMS` re-verification (`34/34 OK`, independently cross-checked locally), and carries a GitHub SLSA v1 build-provenance attestation ([attestation 72905331](https://github.com/AbdulazizShehri/MedScale/attestations/72905331), verifiable with `gh attestation verify <file> --repo AbdulazizShehri/MedScale`).

This candidate supersedes the earlier `6d0279e9` build (runs `43247164670` / `43693295275`) and incorporates all security, FHIR, vault, usability, and qualification hardening merged through PR `#532` (including B1–B5, 64 MiB Argon2id with legacy rewrap, secret zeroization, crash-dump hardening, keyed sealed-blob filenames, Browse URL path PII scanning, Rust idle auto-lock, `#459` saved-model removal, `#457` recovery-code guard, safe model/SQLite/runtime guards, sealed desktop helper records, and hard WebView2 residue assertions).

`REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`. Synthetic workspaces and licensed non-patient audio only. Not for clinical use.
No file is code-signed by a trusted publisher. See [ZERO_COST_DISTRIBUTION.md](ZERO_COST_DISTRIBUTION.md).

## Core files and top-level companions (19 of 34)

| File | Platform | Arch | Kind | Bytes | SHA-256 | Tested |
|---|---|---|---|---|---|---|
| `MedScale_0.1.0-beta.1_SBOM.cdx.json` | all | all | sbom | 330,821 | `efb41bb707effdf1a47c1b5b94934f8c2b5038140313c3d5ec6d86bc3e83e257` | n/a (CycloneDX 1.5 SBOM, 849 components) |
| `MedScale_0.1.0-beta.1_linux-arm64-notices.zip` | linux | arm64 | notices | 3,560,182 | `452357d934520aec17e77db7203f9871c0b3f7b21c2ddb1398b9947e4d8424d6` | n/a (licenses, notices and MPL sources) |
| `MedScale_0.1.0-beta.1_linux-x64-notices.zip` | linux | x64 | notices | 3,560,180 | `3d7de34185151eff38e39ffac9e4e4a69f08662f6831221b8dd61a9ae4994c72` | n/a (licenses, notices and MPL sources) |
| `MedScale_0.1.0-beta.1_macos-arm64-notices.zip` | macos | arm64 | notices | 3,560,449 | `29911d991757dd1387adb88483c85d02b62bd29606b48ae77138eabe0bbe3cad` | n/a (licenses, notices and MPL sources) |
| `MedScale_0.1.0-beta.1_macos-x64-notices.zip` | macos | x64 | notices | 3,560,444 | `4c5674bea44c596bcd48869eef7f3594db61f8351b81a6bf7174d347d88082be` | n/a (licenses, notices and MPL sources) |
| `MedScale_0.1.0-beta.1_windows-x64-notices.zip` | windows | x64 | notices | 3,560,756 | `60673bf51aeb7e74434e6dfd82e08d286a24f06397f9f88e87dfa473257125a8` | n/a (licenses, notices and MPL sources) |
| `MedScale_0.1.0_aarch64.AppImage` | linux | arm64 | appimage | 98,355,720 | `26df980006523c1f36fcd212e8fa7b5b68c5bfeb08a0589684c1c1ae1ff6f80c` | Opened without installing on a clean ARM64 runner (`APPIMAGE_LAUNCH: PASS`) |
| `MedScale_0.1.0_aarch64.dmg` | macos | arm64 | dmg | 21,196,503 | `cff1085243ea69479b1e804d44f24b043c6f1d3ef3345d805a3b43bf969ac67a` | Mounted, copied to Applications, Gatekeeper recorded, opened, reopened, and removed keeping user data (`INSTALL`/`LAUNCH`/`UNINSTALL: PASS`) |
| `MedScale_0.1.0_amd64.AppImage` | linux | x64 | appimage | 100,006,392 | `451e9c6106c8d57fe259d0e38bad8cafd13ee5b7737c826ee1c6da8ec438d67e` | Opened without installing on a clean x64 runner (`APPIMAGE_LAUNCH: PASS`) |
| `MedScale_0.1.0_amd64.deb` | linux | x64 | deb | 24,174,660 | `d9b042fe33d78b99b28d86864938754c4790362743dbf692a28f0ce9e66326bb` | Installed, opened, reinstalled keeping data, and removed on a clean runner (`INSTALL`/`LAUNCH`/`UNINSTALL: PASS`) |
| `MedScale_0.1.0_arm64.deb` | linux | arm64 | deb | 24,576,942 | `47d37caff16dff0b6efe064a15912971296f6b6694417da065808228dba48ec7` | Installed, opened, reinstalled keeping data, and removed on a clean runner (`INSTALL`/`LAUNCH`/`UNINSTALL: PASS`) |
| `MedScale_0.1.0_x64-portable.zip` | windows | x64 | portable | 24,741,925 | `027da581370cfed28c604bb19c48f8d4d0ed8fab0cc6102594a682073a4ebf0b` | Extracted and opened on a clean runner (`PORTABLE_LAUNCH: PASS`) |
| `MedScale_0.1.0_x64-setup.exe` | windows | x64 | installer | 14,315,377 | `66e85d5e31c7921313481187a0b4e528444c353cbd95a7d8a2e634f39befbc08` | 25/25 Windows installed-journey steps PASS (Mark of the Web, sample FHIR refusal, model download/recovery/ASR, encrypted vault + FHIR import, Canvas, 0 WebView2 disk residue after positive control, uninstall keeping data, reinstall) |
| `MedScale_0.1.0_x64.dmg` | macos | x64 | dmg | 23,220,592 | `466fc9beaa2e54debbbad0bd91822676f8ab8bba7596ea8bc583f9432ab1522a` | Mounted, copied to Applications, Gatekeeper recorded, opened, reopened, and removed keeping user data on an Intel runner (`INSTALL`/`LAUNCH`/`UNINSTALL: PASS`) |
| `medscale-0.1.0-aarch64-apple-darwin.tar.gz` | cli | aarch64-apple-darwin | cli | 21,938,842 | `051bf50ae0562e069ffec4b0438194199d0b231d3b8e51584fda99da5ea04d87` | Binary tests, then extracted and run from a clean PATH (`BINARY_TESTS_AND_CLEAN_PATH_RUN: PASS`) |
| `medscale-0.1.0-aarch64-unknown-linux-gnu.tar.gz` | cli | aarch64-unknown-linux-gnu | cli | 24,610,291 | `4a692aa7f89027223cb5d0e3bd77a073a5c5e062a4799a34ddf9716229e7df29` | Binary tests, then extracted and run from a clean PATH (`BINARY_TESTS_AND_CLEAN_PATH_RUN: PASS`) |
| `medscale-0.1.0-x86_64-apple-darwin.tar.gz` | cli | x86_64-apple-darwin | cli | 23,139,318 | `39448772304ba6f1737c68f4d1339266d6bcd67ba50f7e3762271576b5dbe168` | Extracted and run (version, status) under Rosetta 2; binary tests not run on Intel |
| `medscale-0.1.0-x86_64-pc-windows-msvc.zip` | cli | x86_64-pc-windows-msvc | cli | 24,441,921 | `7e663b1a4e7873cf2158acd4ea41a3f2c6dbb5ab9568b483eaeec932682c162f` | Binary tests, then extracted and run from a clean PATH (`BINARY_TESTS_AND_CLEAN_PATH_RUN: PASS`; locally smoke-tested `medscale 0.1.0`) |
| `medscale-0.1.0-x86_64-unknown-linux-gnu.tar.gz` | cli | x86_64-unknown-linux-gnu | cli | 24,300,285 | `0f58c07ff32a5c5104ee31c6ffab9130e6bdf64934491d859a9a1edb87b9f00d` | Binary tests, then extracted and run from a clean PATH (`BINARY_TESTS_AND_CLEAN_PATH_RUN: PASS`) |

## Per-installer package manifests, SBOMs and NOTICE companions (15 of 34)

Every packaged installer (`windows-x64`, `macos-arm64`, `macos-x64`, `linux-x64`, `linux-arm64`) ships with its own verified Spec 101 package manifest, CycloneDX 1.5 SBOM (849 components), and `NOTICE.md` (cross-checked against `artifact-inventory.json` and `SHA256SUMS`):

| File | Platform | Arch | Kind | Bytes | SHA-256 |
|---|---|---|---|---|---|
| `MedScale_0.1.0-beta.1_windows-x64-package-manifest.json` | windows | x64 | package-manifest | 308,212 | `90aaf3c7d9cdcc6ef304d67976647d83cf1924a0c4a37ad049bd54cf33908506` |
| `MedScale_0.1.0-beta.1_windows-x64-SBOM.cdx.json` | windows | x64 | platform-sbom | 346,693 | `be81a064cf095db5468b0829b8e3ffa9ac2c3ae453842a05777c98763b6ff5cd` |
| `MedScale_0.1.0-beta.1_windows-x64-NOTICE.md` | windows | x64 | platform-notice | 2,555 | `a46f98e4755339cd57e459a7760b4030da2d271c01c09e8dc58a23cb3def0fa5` |
| `MedScale_0.1.0-beta.1_macos-arm64-package-manifest.json` | macos | arm64 | package-manifest | 300,453 | `6e2a49c4b9e6d0a6c955eccc4dd5e2edc4a95ba6ce7f4f0a82b35053267819f6` |
| `MedScale_0.1.0-beta.1_macos-arm64-SBOM.cdx.json` | macos | arm64 | platform-sbom | 330,813 | `10d533d3ec7781abacca48bb24bfa298f9b967d3e2f895f11b1917608683b061` |
| `MedScale_0.1.0-beta.1_macos-arm64-NOTICE.md` | macos | arm64 | platform-notice | 2,539 | `ee8e6987f0ea2d814f5709fd9b20a260512f8c962109066e5622e1f9828ad6cd` |
| `MedScale_0.1.0-beta.1_macos-x64-package-manifest.json` | macos | x64 | package-manifest | 300,443 | `f24e39e46481857ee2918d1fe8d2bf0f12a57925f42ebc2ad8c9c88a24b9f4c7` |
| `MedScale_0.1.0-beta.1_macos-x64-SBOM.cdx.json` | macos | x64 | platform-sbom | 330,812 | `2d41834449a67e4e3f028ad32197b2389601f781ba8057b1f0f1658b1c2bffd6` |
| `MedScale_0.1.0-beta.1_macos-x64-NOTICE.md` | macos | x64 | platform-notice | 2,539 | `ee8e6987f0ea2d814f5709fd9b20a260512f8c962109066e5622e1f9828ad6cd` |
| `MedScale_0.1.0-beta.1_linux-x64-package-manifest.json` | linux | x64 | package-manifest | 299,779 | `1070f9aef6f4052df958d765190cce7dbc81bd3eca1b7734a36af4e8329d6cc3` |
| `MedScale_0.1.0-beta.1_linux-x64-SBOM.cdx.json` | linux | x64 | platform-sbom | 330,821 | `efb41bb707effdf1a47c1b5b94934f8c2b5038140313c3d5ec6d86bc3e83e257` |
| `MedScale_0.1.0-beta.1_linux-x64-NOTICE.md` | linux | x64 | platform-notice | 2,539 | `ee8e6987f0ea2d814f5709fd9b20a260512f8c962109066e5622e1f9828ad6cd` |
| `MedScale_0.1.0-beta.1_linux-arm64-package-manifest.json` | linux | arm64 | package-manifest | 299,781 | `33f97b0d45768a463b0eb6dc47b26ced6e0c2d41c5d6d2356327104480b38be4` |
| `MedScale_0.1.0-beta.1_linux-arm64-SBOM.cdx.json` | linux | arm64 | platform-sbom | 330,822 | `17df559e56e6321c1f8c6dc82e6e52d5480785619abbf190661b48547e23fb10` |
| `MedScale_0.1.0-beta.1_linux-arm64-NOTICE.md` | linux | arm64 | platform-notice | 2,539 | `ee8e6987f0ea2d814f5709fd9b20a260512f8c962109066e5622e1f9828ad6cd` |

## Warnings shown with each download

- **windows:** Unsigned: SmartScreen warns (More info, Run anyway); Smart App Control blocks it.
- **macos:** Ad-hoc signed, not notarized: approve once under System Settings, Privacy & Security, Open Anyway.
- **linux:** Unsigned: check the SHA-256 before installing. arm64 needs an ARMv8.2-A CPU with FP16.
- **cli:** Unsigned: the same SmartScreen and Gatekeeper prompts apply to the binary.

## Still open (not blocking an unsigned engineering beta, stated publicly)

- Not code-signed by a trusted publisher (founder decision; SignPath Foundation possible after a first release).
- Smart App Control on Windows 11 blocks unsigned apps; there is no workaround we recommend.
- No automatic updates. Users install a newer version over the old one; back up the vault first.
- Upgrades between released versions are untested: this is the first release.
- Real patient data remains unauthorized (`REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`; see [REAL_DATA_READINESS.md](../beta/REAL_DATA_READINESS.md)). Independent human security review, legal/regulatory review (B6), and clinical validation have not closed.

## Before publishing (engineering — completed)

The website's release gate checks each packaged installer (Windows installer, both DMGs, both `.deb`) against its own package manifest, SBOM and NOTICE, and every other file against `SHA256SUMS`. Run [45916049004](https://github.com/AbdulazizShehri/MedScale/actions/runs/45916049004) built and assembled all 34 files directly in a single pass (`medscale-release-candidate-0.1.0-beta.1`, artifact ID `15164819520`, 471,748,741 bytes, archive digest `sha256:90ec38e41863cce4915320a71bd63d7d387aaa40dde730cf589ed2ddd7c06815`, expires `2027-02-25T00:19:31Z`). All 15/15 workflow jobs succeeded, all 34 files passed `sha256sum -c SHA256SUMS` (34/34 OK, independently re-verified locally) and ClamAV 1.5.4 (`Scanned files: 34`, `Infected files: 0`), GitHub build-provenance attestations were created for all 34 subjects ([attestation 72905331](https://github.com/AbdulazizShehri/MedScale/attestations/72905331)), and an in-memory dry-run of `web/src/lib/release-manifest-gen.ts` (`checkedManifest`) validated all 13 download entries while keeping `web/src/lib/release-manifest.json` inactive (`"release": null`).

## To publish (founder only)

1. Review the files and run [45916049004](https://github.com/AbdulazizShehri/MedScale/actions/runs/45916049004).
2. Create the GitHub Release `v0.1.0-beta.1` from commit `80c67d2dfae139f7471f7f414c9cd112d8a5b07e` with **all 34 files of artifact `15164819520` plus its `SHA256SUMS`**, and make it immutable.
3. Post one GitHub comment containing these 13 `APPROVE` lines and 13 `SIGNING` lines:

```
APPROVE 26df980006523c1f36fcd212e8fa7b5b68c5bfeb08a0589684c1c1ae1ff6f80c
APPROVE cff1085243ea69479b1e804d44f24b043c6f1d3ef3345d805a3b43bf969ac67a
APPROVE 451e9c6106c8d57fe259d0e38bad8cafd13ee5b7737c826ee1c6da8ec438d67e
APPROVE d9b042fe33d78b99b28d86864938754c4790362743dbf692a28f0ce9e66326bb
APPROVE 47d37caff16dff0b6efe064a15912971296f6b6694417da065808228dba48ec7
APPROVE 027da581370cfed28c604bb19c48f8d4d0ed8fab0cc6102594a682073a4ebf0b
APPROVE 66e85d5e31c7921313481187a0b4e528444c353cbd95a7d8a2e634f39befbc08
APPROVE 466fc9beaa2e54debbbad0bd91822676f8ab8bba7596ea8bc583f9432ab1522a
APPROVE 051bf50ae0562e069ffec4b0438194199d0b231d3b8e51584fda99da5ea04d87
APPROVE 4a692aa7f89027223cb5d0e3bd77a073a5c5e062a4799a34ddf9716229e7df29
APPROVE 39448772304ba6f1737c68f4d1339266d6bcd67ba50f7e3762271576b5dbe168
APPROVE 7e663b1a4e7873cf2158acd4ea41a3f2c6dbb5ab9568b483eaeec932682c162f
APPROVE 0f58c07ff32a5c5104ee31c6ffab9130e6bdf64934491d859a9a1edb87b9f00d
SIGNING windows unsigned
SIGNING windows-portable unsigned
SIGNING macos ad-hoc
SIGNING macos-intel ad-hoc
SIGNING linux-deb unsigned
SIGNING linux-appimage unsigned
SIGNING linux-arm64-deb unsigned
SIGNING linux-arm64-appimage unsigned
SIGNING cli-windows-x64 unsigned
SIGNING cli-macos-arm64 unsigned
SIGNING cli-macos-x64 unsigned
SIGNING cli-linux-x64 unsigned
SIGNING cli-linux-arm64 unsigned
```

4. Engineering then runs, in `web/`, `node --experimental-strip-types scripts/manifest-from-candidate.mjs <artifact-inventory.json> v0.1.0-beta.1 <approval comment URL> [<SIGNING comment URL>]`. It fills `release-manifest.json` and adds the tag to the download page's qualification gate. `npm run verify-release` must then pass against the published bytes before any link appears.
