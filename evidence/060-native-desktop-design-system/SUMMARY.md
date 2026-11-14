# Spec 060 Evidence Summary

State: `CLOSED_CANONICAL`.

Founder-approved visual direction supplied on 2026-09-15 is normalized into `PRODUCT.md` and `DESIGN.md`. MedScale Desktop now has a real native Slint 1.16.1 shell with the original MedScale M, canonical palette, navigation, global search, command palette, Home/Command Center, synthetic operational data, local/privacy posture, About/Slint attribution, keyboard focus semantics, and safe route-only shell actions.

`medscale-desktop --smoke` and `--perf-idle-ms` remain headless. The live doctor reports `desktop_shell=slint_native`, `final_v0_ui_present=true`, `tauri_admitted=false`, `wcag_conformance_claimed=false`, and `release_ready=false`.

Local full fmt/clippy/workspace tests and macOS native-window launch passed; see `LOCAL_QUALIFICATION.md`. Final exact-head run `34909664814` passed all six required jobs on `52b5966fc5c4f26a90b437dff91a72a698dc7c40`; PR #103 merged as `cb5dce0e9d6e4dcd261ef22c249b084aff275af5`, and post-merge main run `34911182508` passed all six required jobs.

The first PR exact-head run exposed and rejected `RUSTSEC-2026-0253` in the Slint 1.13.1 femtovg path. The candidate was hardened to Slint 1.16.1 / MSRV 1.88, which removes the vulnerable `lru` package from the active all-features/all-target graph without disabling the GPU renderer or weakening `cargo-deny`.

The second exact-head PR run confirmed `cargo-deny` success after the Slint 1.16.1 hardening, then exposed a Linux runner prerequisite: Ubuntu lacked `fontconfig.pc`. CI now installs the minimal `libfontconfig1-dev` native build dependency before the Rust matrix build.

Portable package qualification and artifact upload passed on Windows, macOS, and Linux in both final exact-head and post-merge main CI. This closes Spec 060 only; final WCAG/assistive-technology qualification, qualified-hardware performance, signing/provenance, and the remaining 061-067 product phase stay open.
