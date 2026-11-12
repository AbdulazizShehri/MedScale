#!/usr/bin/env python3
"""Write the founder-facing report for one release candidate from its
artifact-inventory.json (made by scripts/assemble-release-candidate.py).

Every name, size and SHA-256 comes from the inventory, never typed by hand.
The report ends with the exact approval lines the download page's verifier
expects; only the founder can post them.

Usage: release-candidate-report.py INVENTORY_JSON OUT_MD
"""
import json
import sys
from pathlib import Path

TESTED = {
    "installer": "Installed, opened and uninstalled on a clean runner (Windows installed journey)",
    "dmg": "Mounted, copied to Applications, opened and removed on a clean runner",
    "deb": "Installed, opened, reinstalled keeping data, and removed on a clean runner",
    "appimage": "Opened without installing on a clean runner",
    "portable": "Extracted and opened on a clean runner",
    "cli": "Binary tests, then extracted and run from a clean PATH",
    "notices": "n/a (licenses, notices and MPL sources)",
    "sbom": "n/a (CycloneDX SBOM)",
}
WARN = {
    "windows": "Unsigned: SmartScreen warns (More info, Run anyway); Smart App Control blocks it.",
    "macos": "Ad-hoc signed, not notarized: approve once under System Settings, Privacy & Security, Open Anyway.",
    "linux": "Unsigned: check the SHA-256 before installing. arm64 needs an ARMv8.2-A CPU with FP16.",
    "cli": "Unsigned: the same SmartScreen and Gatekeeper prompts apply to the binary.",
    "all": "",
}


def tested(a: dict) -> str:
    # The Intel macOS CLI is cross-built on Apple silicon: its binary tests
    # don't run, only a smoke run of the extracted archive under Rosetta 2.
    if a["kind"] == "cli" and a["arch"] == "x86_64-apple-darwin":
        return "Extracted and run (version, status) under Rosetta 2; binary tests not run on Intel"
    return TESTED.get(a["kind"], "")


def main() -> int:
    inv = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    out = Path(sys.argv[2])
    files = inv["artifacts"]
    rows = "\n".join(
        f"| `{a['name']}` | {a['platform']} | {a['arch']} | {a['kind']} | {a['size']:,} | `{a['sha256']}` | {tested(a)} |"
        for a in files
    )
    downloads = [a for a in files if a["kind"] not in ("notices", "sbom")]
    approve = "\n".join(f"APPROVE {a['sha256']}" for a in downloads)
    warnings = "\n".join(f"- **{p}:** {w}" for p, w in WARN.items() if w)
    out.write_text(f"""# MedScale {inv['version']} release candidate

**Status: candidate, not published.** Built from commit `{inv['source_sha']}` in [this run]({inv['run']}).
Every file below passed its installation test in that run, a ClamAV scan with fresh signatures and a checksum re-verification, and has a GitHub build-provenance attestation
(`gh attestation verify <file> --repo AbdulazizShehri/MedScale`).

`REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`. Synthetic workspaces and licensed non-patient audio only. Not for clinical use.
No file is code-signed by a trusted publisher. See [ZERO_COST_DISTRIBUTION.md](ZERO_COST_DISTRIBUTION.md).

## Files

| File | Platform | Arch | Kind | Bytes | SHA-256 | Tested |
|---|---|---|---|---|---|---|
{rows}

## Warnings shown with each download

{warnings}

## Still open (not blocking an unsigned engineering beta, stated publicly)

- Not code-signed by a trusted publisher (founder decision; SignPath Foundation possible after a first release).
- Smart App Control on Windows 11 blocks unsigned apps; there is no workaround we recommend.
- No automatic updates. Users install a newer version over the old one; back up the vault first.
- Upgrades between released versions are untested: this is the first release.
- Real patient data blockers B2, B3 and B4 remain (REAL_DATA_READINESS.md).

## To publish (founder only)

1. Review the files and the run above.
2. Create the GitHub Release `v{inv['version']}` from commit `{inv['source_sha']}` with these exact files and make it immutable.
3. Post one GitHub comment containing these lines, plus one `SIGNING <key> <state>` line per platform:

```
{approve}
```

4. Engineering then adds the tag to the download page's qualification gate and fills `release-manifest.json`; `npm run verify-release` must pass before any link appears.
""", encoding="utf-8")
    print(f"wrote {out} with {len(files)} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
