# TOOLCHAIN EVENT — Local Windows Build Toolchain Destruction (2026-09-18)

## Binding

```text
HOST=founder workstation (Windows 11 x64, user Shehr)
WORKTREE=C:/Users/Shehr/work/MedScale-074
BRANCH=spec/074-project-artifact-graph-foundation
WINDOW_START=2026-09-18 ~05:00 +03 (first anomaly: openssl rebuild demanded)
WINDOW_END=2026-09-18 ~07:05 +03 (confirmed: no link.exe anywhere)
REAL_PHI_USED=false
REPOSITORY_MUTATION_BY_EVENT=false (event touched only the machine toolchain)
```

## What happened

During the Spec 074-F qualification session, the local C/C++ toolchain was
removed live while builds were running:

1. `cargo test -p medscale-core --locked --test project_graph_074` passed
   14/14 (including 2 new adversarial tests) after a 28-minute OpenSSL
   vendored build + link. Linker (`link.exe`) provably present.
2. Full-scale run after that demanded an OpenSSL rebuild (`perl` missing:
   `C:\Strawberry` present in PATH but directory gone). Strawberry Perl
   5.42.3.1 portable restored to `C:\Users\Shehr\spp` (verified working).
3. Rebuild then failed in stages as more of the toolchain vanished:
   - `HostX64` MSVC binaries (`cl.exe`, `link.exe`, `nmake.exe`) gone;
     `HostX86` cross tools still present at first.
   - `VC/Auxiliary` (`vcvarsall.bat`) gone minutes after successfully
     initializing the `x86_amd64` environment from it (exit 0, tools found).
   - `Microsoft Visual Studio/Installer` (incl. `vswhere.exe`) gone.
   - `HKLM\SOFTWARE\Microsoft\VisualStudio\Setup` and SxS keys gone.
   - Windows 10 SDK (`C:\Program Files (x86)\Windows Kits`) gone.
   - Finally `HostX86` tools gone too; only `vctip.exe` (telemetry) remains.
4. MSI reinstall blocked by admin policy (`msiexec` denied by system
   administrator policy). `winget` reports Strawberry Perl installed while
   its files are absent (broken entry, uninstall exits 1603).

Result: no `link.exe` on the machine; no C compilation or final linking
possible locally. `rustc` metadata-only work still functions
(`cargo fmt`, `rustc --emit=metadata` typechecks, pure-PowerShell checks).

## Impact on Spec 074 qualification

```text
LOCAL_LINK_AND_C_BUILD=IMPOSSIBLE (external, workstation-only)
LOCAL cargo fmt --check=PASS
LOCAL git diff --check=PASS
LOCAL check-dependency-direction.ps1=PASS
LOCAL rustc --emit=metadata typecheck of new/edited 074 test files=PASS
LOCAL cargo test/clippy (any package needing link or C build scripts)=BLOCKED
CI (GitHub-hosted ubuntu/windows/macos runners)=UNAFFECTED, authoritative path
```

No local PASS is claimed for anything requiring link/C builds. Exact-head
qualification binds to CI run IDs in `EXACT_HEAD_QUALIFICATION.md`, not to
this workstation.

## Human remediation (external action)

1. Reinstall Visual Studio 2022 BuildTools with the MSVC v143 workload and
   a Windows 11 SDK (admin rights required; MSI policy currently denies).
2. Reinstall Strawberry Perl (or keep `C:\Users\Shehr\spp` on PATH).
3. Verify: `cl.exe`, `link.exe`, `nmake.exe`, `perl -Mstrict -e 1`.
4. Rerun the workspace qualification locally:
   `cargo fmt --all -- --check`,
   `./scripts/check-dependency-direction.ps1`,
   `cargo clippy --workspace --all-targets --locked -- -D warnings`,
   `cargo test --workspace --locked`,
   `cargo deny check --all-features`.
5. Close this gate row in `docs/planning/EXTERNAL_GATES.md` only after the
   local suite passes again. CI-first closure of Spec 074 is unaffected.

## Related records

- `docs/planning/EXTERNAL_GATES.md` row
  `LOCAL_WINDOWS_TOOLCHAIN_DESTRUCTION_2026_09_18` (`OPEN_WORKSTATION_ONLY`).
- Scale-run evidence that did complete before the event:
  `C:\Users\Shehr\AppData\Local\Temp\opencode\scale-run.log`
  (lab + reopen PASS; personal full-shape incomplete, see
  `SCALE_MEASUREMENTS.md`).
