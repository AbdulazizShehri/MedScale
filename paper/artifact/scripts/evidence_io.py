#!/usr/bin/env python3
"""Cross-platform evidence capture for MedScale paper experiments."""

from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path("paper/artifact/raw/ci")


def _configure_console() -> None:
    """Use deterministic UTF-8 console output across CI platforms."""
    for stream in (sys.stdout, sys.stderr):
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is not None:
            reconfigure(encoding="utf-8", errors="replace")


def _run_text(command: list[str]) -> str:
    result = subprocess.run(
        command,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    return result.stdout.strip()


def manifest() -> int:
    ROOT.mkdir(parents=True, exist_ok=True)
    runner_os = os.environ.get("RUNNER_OS", sys.platform)
    checkout_sha = _run_text(["git", "rev-parse", "HEAD"])
    lines = [
        f"checkout_sha={checkout_sha}",
        f"github_sha={os.environ.get('GITHUB_SHA', '')}",
        f"runner_os={runner_os}",
        f"runner_arch={os.environ.get('RUNNER_ARCH', '')}",
        f"rustc={_run_text(['rustc', '--version'])}",
        f"cargo={_run_text(['cargo', '--version'])}",
        f"python={sys.version.split()[0]}",
    ]
    (ROOT / f"environment-{runner_os}.txt").write_text(
        "\n".join(lines) + "\n", encoding="utf-8"
    )
    return 0


def run_capture(label: str, command: list[str]) -> int:
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        raise SystemExit("experiment command is required")

    ROOT.mkdir(parents=True, exist_ok=True)
    runner_os = os.environ.get("RUNNER_OS", sys.platform)
    output_path = ROOT / f"{label}-{runner_os}.log"

    with output_path.open("w", encoding="utf-8", newline="\n") as handle:
        process = subprocess.Popen(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=os.environ.copy(),
        )
        assert process.stdout is not None
        for line in process.stdout:
            sys.stdout.write(line)
            handle.write(line)
        return process.wait()


def hash_evidence() -> int:
    """Hash every raw-evidence file recursively using stable relative paths."""
    ROOT.mkdir(parents=True, exist_ok=True)
    checksum_path = ROOT / "checksums.sha256"
    lines: list[str] = []
    for path in sorted(ROOT.rglob("*"), key=lambda item: item.as_posix()):
        if not path.is_file() or path == checksum_path:
            continue
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        relative = path.relative_to(ROOT).as_posix()
        lines.append(f"{digest}  {relative}")
    checksum_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return 0


def main() -> int:
    _configure_console()

    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="mode", required=True)
    sub.add_parser("manifest")
    sub.add_parser("hash")
    run_parser = sub.add_parser("run")
    run_parser.add_argument("--label", required=True)
    run_parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()

    if args.mode == "manifest":
        return manifest()
    if args.mode == "hash":
        return hash_evidence()
    return run_capture(args.label, args.command)


if __name__ == "__main__":
    raise SystemExit(main())
