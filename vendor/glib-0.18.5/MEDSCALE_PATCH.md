# glib 0.18.5, MedScale soundness backport

This directory is the `glib` 0.18.5 crate as published on crates.io, with one
targeted fix. It replaces the registry crate through `[patch.crates-io]` in
`apps/desktop-tauri/src-tauri/Cargo.toml`. Only the Linux desktop uses it:
Tauri's Linux backend (GTK 3, WebKitGTK) depends on glib 0.18, and the Windows
and macOS graphs contain no glib crate.

## Provenance

- Source: `glib-0.18.5.crate` from crates.io, sha256
  `233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5`, equal to
  the checksum in the desktop `Cargo.lock` before this change.
- Upstream: <https://github.com/gtk-rs/gtk-rs-core>, license `MIT`. Copyright
  remains with the gtk-rs authors.
- The first vendoring commit is the unmodified extraction, so the fix reviews
  as a plain diff against it.

## The fix

[RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429): in
`VariantStrIter::impl_get` (`src/variant_iter.rs`), the out-pointer passed to
`g_variant_get_child` was `&p` on an immutable binding, so GLib wrote through a
shared reference. That is undefined behaviour. The fix makes the binding
mutable and passes `&mut p`. It is the same change upstream made in
gtk-rs-core PR #1343, released in glib 0.20.

One more line, in `src/lib.rs`: `#![allow(warnings, clippy::all)]`. A path
dependency gets the repository's `-D warnings`, and rustc 1.97 lints this 2023
code (`mismatched_lifetime_syntaxes`, `unused_parens`) where a registry crate
would have its lints capped. `vendor/candle-core-0.9.1` does the same. Nothing
else is changed.

## When to remove it

When Tauri's Linux backend moves to glib 0.20 or later, delete this directory
and the `[patch.crates-io]` entry, and let the registry crate back in.
