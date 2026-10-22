#!/usr/bin/env python3
"""Paper RQ4 D1 persisted-storage measurement for the synthetic Spec 092 campaign."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

RAW_ROOT = Path("paper/artifact/raw/ci")
PREFIX = "medscale-092-"


def _git(*args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    return result.stdout.strip()


def _campaign_dirs() -> list[Path]:
    temp_root = Path(tempfile.gettempdir())
    return sorted(
        path
        for path in temp_root.glob(f"{PREFIX}*")
        if path.is_dir() and not path.is_symlink()
    )


def clean() -> int:
    removed = 0
    for path in _campaign_dirs():
        shutil.rmtree(path)
        removed += 1
    print(f"removed_campaign_dirs={removed}")
    return 0


def _tree_stats(root: Path) -> dict[str, int]:
    if not root.exists():
        return {"bytes": 0, "files": 0, "directories": 0}
    total_bytes = 0
    files = 0
    directories = 0
    for path in root.rglob("*"):
        if path.is_symlink():
            continue
        if path.is_dir():
            directories += 1
        elif path.is_file():
            files += 1
            total_bytes += path.stat().st_size
    return {"bytes": total_bytes, "files": files, "directories": directories}


def _ratio(numerator: int, denominator: int) -> float | None:
    if denominator == 0:
        return None
    return numerator / denominator


def measure() -> int:
    campaigns = _campaign_dirs()
    if len(campaigns) != 1:
        raise SystemExit(
            f"expected exactly one {PREFIX}* directory after D1 run, found {len(campaigns)}"
        )

    base = campaigns[0]
    required = {name: base / name for name in ("vault", "backup", "restored")}
    missing = [name for name, path in required.items() if not path.is_dir()]
    if missing:
        raise SystemExit(f"D1 campaign missing required directories: {missing}")

    paths = {
        "vault": required["vault"],
        "backup": required["backup"],
        "restored": required["restored"],
        "stage": base / "stage",
    }
    measurements = {name: _tree_stats(path) for name, path in paths.items()}

    vault_children: dict[str, dict[str, int]] = {}
    for child in sorted(paths["vault"].iterdir(), key=lambda item: item.name):
        if child.is_symlink():
            continue
        if child.is_file():
            vault_children[child.name] = {
                "bytes": child.stat().st_size,
                "files": 1,
                "directories": 0,
            }
        elif child.is_dir():
            vault_children[child.name] = _tree_stats(child)

    vault_bytes = measurements["vault"]["bytes"]
    backup_bytes = measurements["backup"]["bytes"]
    restored_bytes = measurements["restored"]["bytes"]
    runner_os = os.environ.get("RUNNER_OS", os.name)

    report = {
        "schema_version": 1,
        "experiment": "D1-integrated-persisted-storage",
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "claim_scope": "descriptive persisted bytes/file counts for one deterministic synthetic campaign",
        "synthetic_only": True,
        "real_phi": False,
        "release_ready": False,
        "platform_qualified": False,
        "storage_threshold_claimed": False,
        "binding": {
            "git_sha": _git("rev-parse", "HEAD"),
            "git_tree": _git("rev-parse", "HEAD^{tree}"),
            "runner_os": runner_os,
            "runner_arch": os.environ.get("RUNNER_ARCH", ""),
            "source_test": "crates/medscale-core/tests/whole_platform_092.rs::every_plane_survives_backup_restore_and_restart_through_core",
        },
        "measurements": measurements,
        "vault_top_level_composition": vault_children,
        "descriptive_ratios": {
            "backup_bytes_divided_by_vault_bytes": _ratio(backup_bytes, vault_bytes),
            "restored_bytes_divided_by_vault_bytes": _ratio(restored_bytes, vault_bytes),
        },
        "methodology": {
            "campaign_directory_prefix": PREFIX,
            "precondition": "stale directories with this test-owned prefix removed before the D1 process",
            "measurement": "recursive regular-file stat().st_size sum after the unchanged Spec 092 test process exits",
            "symlinks_followed": False,
            "population_inference": False,
        },
        "exclusions": [
            "compiled binaries and Cargo target directories",
            "Cargo registry/cache state",
            "model weights and external model caches",
            "operating-system caches",
            "external network/object stores beyond locally persisted campaign state",
            "real PHI",
        ],
        "limitations": [
            "One deterministic synthetic campaign is constructed per OS; this is not a production capacity estimate.",
            "Size ratios are descriptive and are not compression, efficiency, durability, or readiness metrics.",
            "Filesystem allocation blocks and sparse-file physical allocation are not measured; logical file bytes are reported.",
        ],
    }

    output = RAW_ROOT / f"d1-storage-{runner_os}.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote={output}")
    print(f"vault_bytes={vault_bytes}")
    print(f"backup_bytes={backup_bytes}")
    print(f"restored_bytes={restored_bytes}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("clean", "measure"))
    args = parser.parse_args()
    if args.mode == "clean":
        return clean()
    return measure()


if __name__ == "__main__":
    raise SystemExit(main())
