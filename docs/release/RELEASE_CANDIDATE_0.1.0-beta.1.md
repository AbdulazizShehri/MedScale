# MedScale 0.1.0-beta.1 release candidate

**Status: candidate, not published.** Built from commit `6d0279e93fd011b719652a14f6cfa74dc82fad1c` in [this run](https://github.com/AbdulazizShehri/MedScale/actions/runs/43247164670).
Every file below passed its installation test in that run, a ClamAV scan with fresh signatures and a checksum re-verification, and has a GitHub build-provenance attestation
(`gh attestation verify <file> --repo AbdulazizShehri/MedScale`).

`REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`. Synthetic workspaces and licensed non-patient audio only. Not for clinical use.
No file is code-signed by a trusted publisher. See [ZERO_COST_DISTRIBUTION.md](ZERO_COST_DISTRIBUTION.md).

## Files

| File | Platform | Arch | Kind | Bytes | SHA-256 | Tested |
|---|---|---|---|---|---|---|
| `MedScale_0.1.0-beta.1_SBOM.cdx.json` | all | all | sbom | 330,821 | `e6d5b45f4c9d50318d4d3213a82e108d828a73ae634e8fade666718f75a5906c` | n/a (CycloneDX SBOM) |
| `MedScale_0.1.0-beta.1_linux-arm64-notices.zip` | linux | arm64 | notices | 3,560,195 | `c80bb183e52bc8f2b6830da7fe8737f7e85d4f42341b223617c20a7f23655303` | n/a (licenses, notices and MPL sources) |
| `MedScale_0.1.0-beta.1_linux-x64-notices.zip` | linux | x64 | notices | 3,560,192 | `173195f3cdfd8c52a8823d781891827bbe79496651810fc9abe222e3c981bb67` | n/a (licenses, notices and MPL sources) |
| `MedScale_0.1.0-beta.1_macos-arm64-notices.zip` | macos | arm64 | notices | 3,560,456 | `cdedc1d5f38e55f74d9ac171d2b32dd9e3d207747b45044f21e9244b41a2304f` | n/a (licenses, notices and MPL sources) |
| `MedScale_0.1.0-beta.1_macos-x64-notices.zip` | macos | x64 | notices | 3,560,465 | `62f47a2cba2f885cf2db72a7de8bf4a0997a281e13b8aa2a56534764ef8d7664` | n/a (licenses, notices and MPL sources) |
| `MedScale_0.1.0-beta.1_windows-x64-notices.zip` | windows | x64 | notices | 3,560,767 | `de175a0c78fe33dbcd9dd501c06fb57bfdfd84dec27db82bdce2f40b09ac4839` | n/a (licenses, notices and MPL sources) |
| `MedScale_0.1.0_aarch64.AppImage` | linux | arm64 | appimage | 98,298,376 | `35b3efcdffa2e91e02afd18f4bec5ede521af35318be63314f6a8614b1a3e12a` | Opened without installing on a clean runner |
| `MedScale_0.1.0_aarch64.dmg` | macos | arm64 | dmg | 21,140,979 | `e7774cd442a9c1586430284d8eb3506622a48825fe7ef8fbec872c0cfa4c11b0` | Mounted, copied to Applications, opened and removed on a clean runner |
| `MedScale_0.1.0_amd64.AppImage` | linux | x64 | appimage | 99,944,952 | `4223ea2e2c2a7a59e9fdcd8f9b22159580267ae0660a1c0733502a6abe39dde3` | Opened without installing on a clean runner |
| `MedScale_0.1.0_amd64.deb` | linux | x64 | deb | 24,107,016 | `f8e1936e819e2b6561a96a5231a13ba350c5698c03ab6061e1e06f592fdf85bc` | Installed, opened, reinstalled keeping data, and removed on a clean runner |
| `MedScale_0.1.0_arm64.deb` | linux | arm64 | deb | 24,506,814 | `54e6ef0a2fc89f77c9540b665c3a1d83b1d5b660472f8a16320ec036add08e09` | Installed, opened, reinstalled keeping data, and removed on a clean runner |
| `MedScale_0.1.0_x64-portable.zip` | windows | x64 | portable | 24,677,882 | `492f5a604ec55ae5e26d05c2471ab7b628c4218025f1bd74048400a20d86aa16` | Extracted and opened on a clean runner |
| `MedScale_0.1.0_x64-setup.exe` | windows | x64 | installer | 14,280,170 | `e922e47360d87681cd80a8c1f7b3f9e07e2c95d8b9eef82a254731d8fbdfbe22` | Installed, opened and uninstalled on a clean runner (Windows installed journey) |
| `MedScale_0.1.0_x64.dmg` | macos | x64 | dmg | 23,161,269 | `c8bf3d25ca248edf84b529bb2de8dd4c195ea22843fa3ee54575325322a70279` | Mounted, copied to Applications, opened and removed on a clean runner |
| `medscale-0.1.0-aarch64-apple-darwin.tar.gz` | cli | aarch64-apple-darwin | cli | 21,874,028 | `82fd44696b0fe09e406e7c483ed0e005e46d20a5e197c63a8cbc83093400aae3` | Binary tests, then extracted and run from a clean PATH |
| `medscale-0.1.0-aarch64-unknown-linux-gnu.tar.gz` | cli | aarch64-unknown-linux-gnu | cli | 24,536,929 | `edc53fc45167536ac16e81dea3dc8730abde2e2fcbef9db7faecd89ee7fc9319` | Binary tests, then extracted and run from a clean PATH |
| `medscale-0.1.0-x86_64-apple-darwin.tar.gz` | cli | x86_64-apple-darwin | cli | 23,047,287 | `3e2da9809915d1270b2dbc35e491206e5fbc4131d18f3b688888f1b876a7f09a` | Extracted and run (version, status) under Rosetta 2; binary tests not run on Intel |
| `medscale-0.1.0-x86_64-pc-windows-msvc.zip` | cli | x86_64-pc-windows-msvc | cli | 24,393,665 | `630f4e021e4766c9910628e6107adb3888d4fda6bb1c8e1d19d9b2b80607da30` | Binary tests, then extracted and run from a clean PATH |
| `medscale-0.1.0-x86_64-unknown-linux-gnu.tar.gz` | cli | x86_64-unknown-linux-gnu | cli | 24,217,975 | `6035f4a8124d1c82a9c923cb760bfa528e84dad992f1416ec1c2542083eb7f7b` | Binary tests, then extracted and run from a clean PATH |

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
- Real patient data blockers B2, B3 and B4 remain (REAL_DATA_READINESS.md).

## To publish (founder only)

1. Review the files and the run above.
2. Create the GitHub Release `v0.1.0-beta.1` from commit `6d0279e93fd011b719652a14f6cfa74dc82fad1c` with these exact files and make it immutable.
3. Post one GitHub comment containing these lines, plus one `SIGNING <key> <state>` line per platform:

```
APPROVE 35b3efcdffa2e91e02afd18f4bec5ede521af35318be63314f6a8614b1a3e12a
APPROVE e7774cd442a9c1586430284d8eb3506622a48825fe7ef8fbec872c0cfa4c11b0
APPROVE 4223ea2e2c2a7a59e9fdcd8f9b22159580267ae0660a1c0733502a6abe39dde3
APPROVE f8e1936e819e2b6561a96a5231a13ba350c5698c03ab6061e1e06f592fdf85bc
APPROVE 54e6ef0a2fc89f77c9540b665c3a1d83b1d5b660472f8a16320ec036add08e09
APPROVE 492f5a604ec55ae5e26d05c2471ab7b628c4218025f1bd74048400a20d86aa16
APPROVE e922e47360d87681cd80a8c1f7b3f9e07e2c95d8b9eef82a254731d8fbdfbe22
APPROVE c8bf3d25ca248edf84b529bb2de8dd4c195ea22843fa3ee54575325322a70279
APPROVE 82fd44696b0fe09e406e7c483ed0e005e46d20a5e197c63a8cbc83093400aae3
APPROVE edc53fc45167536ac16e81dea3dc8730abde2e2fcbef9db7faecd89ee7fc9319
APPROVE 3e2da9809915d1270b2dbc35e491206e5fbc4131d18f3b688888f1b876a7f09a
APPROVE 630f4e021e4766c9910628e6107adb3888d4fda6bb1c8e1d19d9b2b80607da30
APPROVE 6035f4a8124d1c82a9c923cb760bfa528e84dad992f1416ec1c2542083eb7f7b
```

4. Engineering then adds the tag to the download page's qualification gate and fills `release-manifest.json`; `npm run verify-release` must pass before any link appears.
