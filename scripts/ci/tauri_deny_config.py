#!/usr/bin/env python3
"""Write the Tauri desktop's cargo-deny config: the repository deny.toml,
unchanged, with its dependency graph limited to the shipped desktop targets.

The Tauri desktop ships on Windows, macOS and x86_64 Linux. Limiting the graph
to those targets keeps every advisory, license, ban and source rule exactly as
strict as the repository policy; the only change is that crates for other
targets are not in the shipped graph. The Linux graph includes GTK 3 and glib
0.18; glib is the vendored copy with the RUSTSEC-2024-0429 fix
(vendor/glib-0.18.5/MEDSCALE_PATCH.md), so no advisory is ignored for it. The script refuses a base config that relaxes the unsound
policy, or ignores an advisory without naming it and giving a reason (each
exception is a founder decision recorded in deny.toml).

Usage: tauri_deny_config.py BASE_DENY_TOML OUT_TOML
"""
import re
import sys
import tomllib

SHIPPED_TARGETS = [
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-unknown-linux-gnu",
]


def main() -> int:
    base, out = sys.argv[1], sys.argv[2]
    text = open(base, encoding="utf-8").read()
    ignores = tomllib.loads(text).get("advisories", {}).get("ignore", [])
    for entry in ignores:
        if not (
            isinstance(entry, dict)
            and re.fullmatch(r"RUSTSEC-\d{4}-\d{4}", str(entry.get("id", "")))
            and len(str(entry.get("reason", "")).strip()) >= 20
        ):
            sys.exit("every ignored advisory needs an id and a reason naming its mitigation")
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
