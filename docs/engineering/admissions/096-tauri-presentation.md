# Spec 096 dependency admission — presentation preview

**Operation:** locked normal `DEPENDENCY`; no upstream implementation copied into the repository.

**Placement:** isolated `apps/desktop-tauri` presentation workspace; no Core, canonical store, key or network authority.
**Qualification:** preparatory synthetic-only build, not release admission.

Tauri Rust 2.12.0 / build 2.7.0 from `https://github.com/tauri-apps/tauri` use the Apache-2.0 OR MIT license choice. The app selects only Wry, compression, Windows controls and X11 features. API/CLI npm 2.12.0 are from the same official project. The React/React DOM 19.3.0 runtime is MIT; TypeScript 7.0.2 is Apache-2.0; Vite 8.3.1 and Tailwind 4.3.3 are MIT build tools. All resolved package identities/integrities are in the two committed lockfiles. The root Core workspace and its lockfile are untouched.

## Exact MPL source obligations

These unmodified transitive CSS/platform libraries are admitted by exact-version license exceptions, without adding MPL to the global allow list. MPL source remains separately licensed. No MedScale file is derived from a modified MPL source file. Any preview binary artifact must include the original `.crate` source archives, their original notices, a full MPL text and a notice identifying the sources. The packaging script verifies the archive SHA-256 against the locked identity before inclusion.

| Dependency | Upstream | Crate archive SHA-256 |
|---|---|---|
| cssparser 0.37.0 | https://github.com/servo/rust-cssparser | `8c9cdaae01d5ed7882b04d795e7f752f46ff52d2fa3b50a20d28c464510bba98` |
| cssparser-macros 0.7.1 | https://github.com/servo/rust-cssparser | `d045de693cb712d0b22c6a64be5b953f67b3ce00ab5ad3dd5d8b441886ab8e1a` |
| dtoa-short 0.3.5 | https://github.com/upsuper/dtoa-short | `cd1511a7b6a56299bd043a9c167a6d2bfb37bf84a6dfceaba651168adfb43c87` |
| option-ext 0.2.0 | https://github.com/soc/option-ext | `04744f49eae99ab78e0d5c0b603ab218f515ea8cfe5a456d7629ad883a3b6e7d` |
| selectors 0.38.0 | https://github.com/servo/stylo | `8adfa1c298912827b8a28b223b3b874357397ae706e6190acd9bf28cee99114d` |

Canonical registry source URL for each row is `https://static.crates.io/crates/NAME/NAME-VERSION.crate`. Original headers and license files remain in those archives. The preview also carries the existing Inter and JetBrains Mono NL OFL notices and the first-party project NOTICE. Lightning CSS 1.32.0/1.33.0 is unmodified MPL build tooling in the npm lock, absent from the shipped JS bundle; a redistribution of that tool itself requires its original source/notices separately.

## Security and lifecycle

Npm installation disables all lifecycle scripts. The direct and complete npm lock inventories were checked with NPMScan; the actual lock paths were independently checked after a duplicate-version source-mapping warning. See 096 research for limits. Rust cargo-deny is run against this final graph; `glib` 0.18.5 / RUSTSEC-2024-0429 remains an open Linux security finding and is not ignored. License admission does not resolve it. Since PR #177, Tauri grants the main local window exactly the named commands generated from `apps/desktop-tauri/src-tauri/build.rs` (67 at that PR), each a narrow request that Rust validates and sends through the in-process Core session; no Tauri plugin and no filesystem, HTTP, shell, process or generic Core permission is granted. Before PR #177 the only command was the shell-status diagnostic.

Updates require a forward lockfile change, advisory/license/script review, narrow capability review and native evidence at the changed head. The exit strategy is to remove this isolated preview and retain Slint until migration parity; a Tauri upstream upgrade must not change Core semantics or the PHI boundary. No source-policy or release claim is made from dependency resolution alone.
