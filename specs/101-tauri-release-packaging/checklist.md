# Qualification checklist: Spec 101

- [x] Founder authorization recorded (decision 4, 2026-10-07); zero cost; signing out of scope.
- [x] Extends the Spec 058 manifest, the Spec 054 SBOM and the Spec 096 package; no parallel process.
- [x] Explicit dependency on PR #177; stacked branch; not merge-eligible before its base chain.
- [x] Build-time tool downloads enumerated with URL, version, digest source, license, cache, update and failure behavior; weak-digest limitation recorded.
- [x] Install-time network avoided (`webviewInstallMode=skip`); WebView2 Evergreen prerequisite documented.
- [x] AppImage rejected (floating, unverified download).
- [x] Rollback and update story stated.
- [x] Qualification matrix with separate cells per platform.
- [x] T101-01 bundle config plus config test; T101-02 CI bundle step.
- [ ] T101-03 measured CI wall time per platform.
- [ ] T101-04/05 manifest, `SHA256SUMS` and fail-closed verifier with negative tests.
- [ ] T101-07/08/09 install, launch and uninstall probes on CI runners only.
- [ ] T101-10 evidence per platform at the exact head, with a not-performed list.
- [ ] T101-11 exact-range scope and requirement traceability.
- [ ] T101-12 #177 merged; exact-head CI; scoped Jev/OCR status; normal merge; post-main CI.
