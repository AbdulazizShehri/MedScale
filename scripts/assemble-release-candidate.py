#!/usr/bin/env python3
"""Assemble one MedScale release candidate from a single workflow run.

Reads the artifacts that tauri-preview and cli-release uploaded in this run
(one directory per artifact), keeps only files whose platform qualification
report passed, and writes:

  OUT/files/...                  the candidate files, flat
  OUT/SHA256SUMS                 sha256 of every file
  OUT/artifact-inventory.json    name, platform, arch, kind, size, sha256,
                                 qualification, signing, source SHA, run
  OUT/RELEASE_NOTES.md           what is in this candidate and its warnings

Nothing is published. Signing is recorded as it is: unsigned, or ad-hoc on
macOS, which is not Developer ID or notarization.

Usage: assemble-release-candidate.py ARTIFACTS_DIR OUT_DIR SOURCE_SHA RUN_URL VERSION
"""
import hashlib
import json
import shutil
import sys
from pathlib import Path

# (artifact directory, release-set platform, platform, arch, report)
DESKTOP = [
    ("tauri-installer-windows-latest", "windows", "windows", "x64", "windows.json"),
    ("tauri-installer-macos-latest", "macos", "macos", "arm64", "macos.json"),
    ("tauri-installer-macos-15-intel", "macos", "macos", "x64", "macos.json"),
    ("tauri-installer-ubuntu-22.04", "linux", "linux", "x64", "linux.json"),
    ("tauri-installer-ubuntu-22.04-arm", "linux", "linux", "arm64", "linux.json"),
]
REQUIRED_PASS = ("INSTALL", "LAUNCH", "UNINSTALL")
SIGNING = {"windows": "unsigned", "macos": "ad-hoc (not Developer ID, not notarized)", "linux": "unsigned"}


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def report(path: Path, keys) -> dict:
    data = json.loads(path.read_text(encoding="utf-8-sig"))
    failed = [k for k in keys if data.get(k) != "PASS"]
    if failed:
        sys.exit(f"{path}: qualification not passed: {', '.join(failed)}")
    return {k: data[k] for k in keys}


def one(paths, what):
    paths = list(paths)
    if len(paths) != 1:
        sys.exit(f"expected exactly one {what}, found {len(paths)}: {paths}")
    return paths[0]


def main() -> int:
    artifacts, out, source_sha, run_url, version = (Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3], sys.argv[4], sys.argv[5])
    files = out / "files"
    files.mkdir(parents=True, exist_ok=False)
    inventory = []

    def keep(src: Path, platform: str, arch: str, kind: str, qualification: dict, signing: str):
        dest = files / src.name
        if dest.exists():
            sys.exit(f"two artifacts share the name {src.name}")
        shutil.copy2(src, dest)
        inventory.append({
            "name": src.name, "platform": platform, "arch": arch, "kind": kind,
            "size": dest.stat().st_size, "sha256": sha256(dest),
            "qualification": qualification, "signing": signing,
        })

    for directory, set_platform, platform, arch, report_name in DESKTOP:
        base = artifacts / directory
        release = base / "tauri-release" / set_platform
        qual = report(base / "tauri-release-qualification" / report_name, REQUIRED_PASS)
        installer = one((release / "installer").iterdir(), f"{directory} installer")
        kind = {"windows": "installer", "macos": "dmg", "linux": "deb"}[platform]
        keep(installer, platform, arch, kind, qual, SIGNING[platform])
        # Notices, licenses and MPL sources for this platform's dependency graph.
        staging = out / "staging" / f"{platform}-{arch}"
        shutil.copytree(release, staging, ignore=shutil.ignore_patterns("installer"))
        notices = shutil.make_archive(str(out / f"MedScale_{version}_{platform}-{arch}-notices"), "zip", staging)
        Path(notices).rename(files / Path(notices).name)
        inventory.append({
            "name": Path(notices).name, "platform": platform, "arch": arch, "kind": "notices",
            "size": (files / Path(notices).name).stat().st_size, "sha256": sha256(files / Path(notices).name),
            "qualification": {}, "signing": "n/a",
        })
        extra = base / "tauri-release-extra"
        if platform == "linux":
            machine = "x86_64" if arch == "x64" else "aarch64"
            appimage_qual = report(base / "tauri-release-qualification" / f"linux-appimage-{machine}.json", ("APPIMAGE_LAUNCH",))
            keep(one(extra.glob("*.AppImage"), f"{directory} AppImage"), platform, arch, "appimage", appimage_qual, SIGNING[platform])
        if platform == "windows":
            keep(one(extra.glob("*-portable.zip"), "portable zip"), platform, arch, "portable", {"PORTABLE_LAUNCH": "PASS"}, SIGNING[platform])

    for cli in sorted(artifacts.glob("medscale-cli-*")):
        target = cli.name.removeprefix("medscale-cli-")
        archive = one([p for p in cli.iterdir() if p.name.endswith((".zip", ".tar.gz"))], f"{cli.name} archive")
        keep(archive, "cli", target, "cli", {"BINARY_TESTS_AND_CLEAN_PATH_RUN": "PASS"}, "unsigned")

    sbom = one(artifacts.glob("tauri-installer-ubuntu-22.04/tauri-release/linux/SBOM.cdx.json"), "SBOM")
    shutil.copy2(sbom, files / f"MedScale_{version}_SBOM.cdx.json")
    inventory.append({"name": f"MedScale_{version}_SBOM.cdx.json", "platform": "all", "arch": "all", "kind": "sbom",
                      "size": sbom.stat().st_size, "sha256": sha256(files / f"MedScale_{version}_SBOM.cdx.json"),
                      "qualification": {}, "signing": "n/a"})

    shutil.rmtree(out / "staging", ignore_errors=True)
    inventory.sort(key=lambda e: e["name"])
    (out / "SHA256SUMS").write_text("".join(f"{e['sha256']}  {e['name']}\n" for e in inventory), encoding="ascii")
    (out / "artifact-inventory.json").write_text(json.dumps({
        "schema": 1, "version": version, "source_sha": source_sha, "run": run_url,
        "published": False, "founder_approval": None,
        "real_phi_authorization": "NOT_AUTHORIZED", "clinical_use": "not authorized",
        "artifacts": inventory,
    }, indent=1), encoding="utf-8")
    rows = "\n".join(f"| `{e['name']}` | {e['platform']} {e['arch']} | {e['kind']} | {e['signing']} | `{e['sha256']}` |" for e in inventory)
    (out / "RELEASE_NOTES.md").write_text(f"""# MedScale {version} release candidate

Built from `{source_sha}` in {run_url}. **Not published.** Publication needs the founder's approval of these exact files.

Synthetic data only: `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`, and MedScale is not for clinical use.

None of these files is code-signed by a trusted publisher. Windows SmartScreen warns, and Smart App Control blocks unsigned apps. macOS requires
**Open Anyway** in System Settings → Privacy & Security. See docs/release/ZERO_COST_DISTRIBUTION.md.

| File | Platform | Kind | Signing | SHA-256 |
|---|---|---|---|---|
{rows}
""", encoding="utf-8")
    print(json.dumps([e["name"] for e in inventory], indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
