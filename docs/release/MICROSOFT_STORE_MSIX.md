# MedScale: trusted zero-certificate-cost distribution (MSIX pilot)

Status: **ARCHIVE PACKAGING PILOT ONLY — NO PUBLIC RELEASE**  
Decision reference: [Issue #507](https://github.com/AbdulazizShehri/MedScale/issues/507)  
Founder policy: no paid Windows or Apple certificates, no misleading bypass instructions.

## Why this exists

A free download is not necessarily a trusted installer. The existing Tauri desktop builds an **unsigned NSIS EXE**. Microsoft Store **does not sign EXE/MSI submissions**, even when they are listed in the Store. However, it **does sign accepted Store MSIX/AppX packages for free**. Microsoft offers free individual developer registration in the new onboarding flow. The Store package is therefore the primary realistic Windows route to download and install without an end-user SmartScreen bypass.

- Microsoft confirmation of signing distinction: https://learn.microsoft.com/en-us/windows/apps/publish/faq/get-started-with-the-microsoft-store
- Tauri-to-MSIX official guide (2026): https://learn.microsoft.com/en-us/windows/apps/dev-tools/winapp-cli/guides/tauri
- WinApp CLI packaging: https://github.com/microsoft/winappCli/blob/main/plugins/winapp/skills/winapp-package/SKILL.md
- Free Store registration: https://blogs.windows.com/windowsdeveloper/2025/09/10/free-developer-registration-for-individual-developers-on-microsoft-store/

This is not proof of Store certification or universal no-warning compatibility. Windows device policy, enterprise restrictions, and the Store's certification process can still affect installation.

## Implementation in this isolated pilot

- `apps/desktop-tauri/msix/Package.appxmanifest.template`: a **full-trust Windows desktop** MSIX manifest, not a substitute for the ordinary Tauri identifier or clinical/data policies.
- `scripts/package-tauri-msix.ps1`: runs **only on disposable Windows GitHub Actions runners**. Stages the exact released Tauri executable, generates appropriately sized icon assets, fills an explicit identity, builds an **unsigned** MSIX with `winapp package --no-sign`, unpacks and checks its contents, and records a source-bound SHA-256 qualification manifest.
- `.github/workflows/windows-msix-qualification.yml`: PR proof, Windows x64 only; pinned Windows CLI setup action and version. Builds source from the locked Rust/NPM inputs, checks the archive, and uploads **metadata only**, never the unsigned MSIX.

The fake `MedScale.CI.Preview` identity is strictly for this build proof. It is **not** a Store publisher identity.

The `PartnerCenter` profile cannot be used until the owner registers the exact app identity and publisher. No MSIX is currently signed with a trusted certificate.

## Next implementation and qualification gates

1. Review and repair the MSIX pilot at its exact commit on a Windows hosted runner. Check `winapp` CLI version, schema validation, logo dimensions, archive correctness and dependency inclusion. Do not infer an installed-app pass from unpack success.
2. Package a second candidate with a disposable **CI-only self-signed certificate**. Trust it **only on that disposable runner** and verify a clean MSIX install/launch/uninstall and no residual AppX package installation. Never transfer or install this development certificate onto an end user's computer.
3. Test an **actual MSIX installed** Tauri application with Windows 11 x64 WebView2, startup, native dialogs, encrypted vault creation/unlock, recovery, FHIR import, Research Canvas, backup, app data persistence, local-first egress controls, and update migration from the previous NSIS application. MSIX file/registry virtualization and package data paths differ from NSIS. Keep the canonical `org.medscale.desktop.preview` data path and never delete/migrate a user's data speculatively.
4. Confirm whether `runFullTrust` (restricted capability) is needed for the Tauri executable and include an accurate explanation in the Store submission; Microsoft Store can reject unsupported restricted capability usage. Do not claim Microsoft certification until it happens.
5. Request the founder to **enroll at** https://storedeveloper.microsoft.com/ **as an individual**, reserve the app name, and provide the exact Partner Center **Package Name, Publisher X.500 name, Publisher Display Name, and required listing metadata**. The account owner must complete identity/terms verification, and any MFA, personally.
6. Use `-Profile PartnerCenter` only with exact reserved identity values on a controlled CI run. Build the unsigned MSIX for **Store upload only**, with SHA-256, SBOM, source revisions, policy/privacy statements, third-party notices, and proof of installed app behavior. Submit only after founder-approved exact artifacts and independent release security review.
7. After Microsoft Store approves and publishes the build, verify the Store's **Microsoft-signed** package and real installation on a **clean Windows 11 host with Smart App Control enabled**. Only then expose a working `Get it from Microsoft` link on the MedScale Download page. Do not link an unreleased preview or suggest disabling security controls.

## SignPath as secondary route

SignPath Foundation may eventually sign **direct-download Windows EXE/installer** at no certificate cost, but acceptance is discretionary. Its requirements include an already released app, sufficient project reputation, public source/licensing compliance, an explicit **Code signing policy**, signing approvers, MFA, and a verifiable build chain. It signs with SignPath Foundation's certificate, not a personal certificate. Even a valid Authenticode signature does not guarantee immediate SmartScreen reputation.

Review PR #500 and https://signpath.org/terms.html before applying. Do not claim SignPath has accepted MedScale.

## macOS: a separate gate

Microsoft Store does not solve macOS Gatekeeper. A conventional frictionless direct Mac download normally requires Apple Developer ID signing and notarization, which generally requires a paid Apple Developer Program membership. A self-signed or ad-hoc signature is **not equivalent**. Without an eligible organization-sponsored account, a no-payment, no-warning one-click macOS DMG is **not currently established**.

Do not bypass Gatekeeper, remove quarantine attributes, globally disable protections, or mislabel an ad-hoc signed DMG as notarized.

## Release and clinical gates (UNCHANGED)

`RELEASE_READY=false` and `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`. No public software release is approved. Clinical/medical claims, independent security and legal/privacy evaluation, and real-data restrictions remain independent blockers. **The finished Store packaging route is not itself authorization to release MedScale for clinical use or handle patient records.**

Related work: PRs #500, #501, #502, #503, #504 and Issue #507. Avoid overlapping changes to existing release-candidate and website feature work.
