#!/usr/bin/env python3
"""Write the Tauri desktop's cargo-deny config: the repository deny.toml,
unchanged, with its dependency graph limited to the shipped desktop targets.

The Tauri desktop ships on Windows and macOS only (EXTERNAL_GATES.md,
TAURI_GLIB_ADVISORY_F096_T01). Limiting the graph to those targets keeps every
advisory, license, ban and source rule exactly as strict as the repository
policy; the only change is that Linux-only crates (GTK 3, glib 0.18) are not in
the shipped graph. The script refuses a base config that ignores advisories or
relaxes the unsound policy.

Usage: tauri_deny_config.py BASE_DENY_TOML OUT_TOML
"""
import re
import sys

SHIPPED_TARGETS = [
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
]


def main() -> int:
    base, out = sys.argv[1], sys.argv[2]
    text = open(base, encoding="utf-8").read()
    if not re.search(r"^ignore = \[\]$", text, re.M):
        sys.exit("base deny config must not ignore advisories")
    if not re.search(r'^unsound = "all"$', text, re.M):
        sys.exit('base deny config must keep unsound = "all"')
    if re.search(r"^targets\s*=", text, re.M):
        sys.exit("base deny config already limits targets")
    if text.count("[graph]\n") != 1:
        sys.exit("base deny config must have one [graph] table")
    targets = ", ".join(f'"{t}"' for t in SHIPPED_TARGETS)
    text = text.replace("[graph]\n", f"[graph]\ntargets = [{targets}]\n", 1)
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    print("\n".join(SHIPPED_TARGETS))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
