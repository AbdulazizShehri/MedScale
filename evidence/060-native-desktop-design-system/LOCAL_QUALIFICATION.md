# Local Qualification — Spec 060

Date: 2026-09-15
Platform: macOS development host (not qualified release hardware)

## Gates before first PR head
- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS.
- `cargo test --workspace --locked`: PASS.
- `git -c core.whitespace=cr-at-eol diff --check`: PASS.
- `cargo test --locked -p medscale-core --test native_desktop_ui_060`: 3/3 PASS.
- Native `MedScale` window observed through macOS System Events: PASS.
- `medscale-desktop --smoke`: PASS without opening UI.
- `medscale-desktop --perf-idle-ms 50`: PASS without opening UI.

## Renderer qualification iteration
The initial Slint 1.13.1 GPU candidate measured about 9.2 MB binary and ~127 MiB RSS locally, but PR #103 exact-head run `34905865201` correctly rejected it because femtovg 0.17 selected `lru 0.16.4` (`RUSTSEC-2026-0253`). No advisory ignore was added.

A Slint 1.13.1 software-renderer experiment removed `lru` but sampled substantially higher RSS (~165 MiB). The final candidate therefore moves to Slint 1.16.1 / Rust MSRV 1.88 / femtovg 0.23.2. Its all-features/all-target dependency graph contains no `lru`; `libfuzzer-sys` remains only as an all-target build-tool metadata path and receives an exact-crate NCSA license exception.

Final Slint 1.16.1 development sample:
- release binary: 12,293,200 bytes (~11.72 MiB);
- live RSS after window appearance: 97,648 KiB (~95.4 MiB);
- native window: observed as `MedScale`;
- headless smoke and bounded idle probes: PASS.

MSRV qualification:
- `cargo +1.88.0 check --workspace --all-targets --locked`: PASS.
- `smol_str` is pinned at 0.3.2 and `typed-index-collections` at 3.3.0 because newer semver-compatible transitive releases require Rust 1.89/1.90 even though Slint 1.16.1 itself declares Rust 1.88.
- the pins preserve the declared workspace MSRV instead of silently raising it.

These are development-host observations only. They do not clear `QUALIFIED_RELEASE_PERFORMANCE_HARDWARE` and do not claim final UI latency or memory-budget attainment.

## Accessibility observation
Slint accessibility is enabled and primary custom actions use `FocusScope`, keyboard Enter/Space activation, visible focus borders, accessible roles/labels, and default accessibility actions. macOS exposed the native window and accessibility elements to System Events. This is engineering evidence only, not screen-reader or WCAG conformance qualification.

## Visual review limitation
The remote macOS process did not have Screen Recording permission, so `screencapture` could not produce a visual artifact. Native window existence and title were observed through WindowServer/System Events. Final visual/a11y qualification remains owned by later product slices and Spec 067.

## Canonical CI qualification
- Final PR #103 exact head: `52b5966fc5c4f26a90b437dff91a72a698dc7c40`.
- Exact-head run `34909664814`: PASS, all six required jobs.
- Portable release package qualification + evidence upload: PASS on Windows, macOS, and Linux.
- Merge commit: `cb5dce0e9d6e4dcd261ef22c249b084aff275af5`.
- Post-merge main run `34911182508`: PASS, all six required jobs.
- Post-merge portable/runtime artifacts: present for Windows, macOS, and Linux.

This evidence closes Spec 060. It does not claim production signing/notarization, qualified-hardware performance attainment, private-data readiness, or WCAG conformance.
