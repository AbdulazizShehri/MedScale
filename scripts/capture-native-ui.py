"""Capture one actual Slint window from a qualified binary; never a mock UI."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import uuid

from PIL import Image


def sha256_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--candidate-head", required=True)
    parser.add_argument("--expected-tree", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--route", default="Home")
    parser.add_argument("--theme", choices=("light", "dark"), default="light")
    parser.add_argument("--width", type=int, default=1440)
    parser.add_argument("--height", type=int, default=900)
    parser.add_argument("--compact", action="store_true")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    output = args.output_dir.resolve()
    if not output.is_relative_to(repo):
        parser.error("evidence output must remain inside this repository")
    if not 1100 <= args.width <= 1920 or not 720 <= args.height <= 1200:
        parser.error("window size outside qualified capture bounds")
    for sha in (args.candidate_head, args.expected_tree):
        if len(sha) != 40 or any(c not in "0123456789abcdef" for c in sha):
            parser.error("head/tree must be full lowercase Git hashes")
    executable = args.executable.resolve(strict=True)
    manifest = json.loads((executable.parent.parent / "package-manifest.json").read_text(encoding="utf-8-sig"))
    if manifest["source"]["tree_sha"] != args.expected_tree:
        parser.error("portable binary source tree does not match candidate tree")
    candidate_tree = subprocess.run(
        ["git", "rev-parse", f"{args.candidate_head}^{{tree}}"], cwd=repo,
        capture_output=True, text=True, timeout=10, check=True,
    ).stdout.strip()
    if candidate_tree != args.expected_tree:
        parser.error("candidate head does not resolve to the portable source tree")
    payload = [entry for entry in manifest["payload"]
               if entry["path"] == f"bin/{executable.name}"]
    if len(payload) != 1:
        parser.error("portable manifest does not bind the selected executable")
    binary_sha256 = sha256_file(executable)
    if binary_sha256 != payload[0]["sha256"]:
        parser.error("selected executable differs from the qualified package payload")
    raw = output / "raw"
    raw.mkdir(parents=True, exist_ok=True)
    name = f"{args.route.lower().replace(' ', '-')}-{args.theme}-{args.width}x{args.height}{'-compact' if args.compact else ''}"
    ppm = raw / f"{name}-{uuid.uuid4().hex}.ppm"
    png = output / f"{name}.png"
    record = output / f"{name}.json"
    if png.exists() or record.exists():
        parser.error("evidence exists; use a fresh output directory")
    command = [str(executable), "--render-evidence", str(ppm), "--render-route", args.route,
               "--render-theme", args.theme, "--render-size", f"{args.width}x{args.height}"]
    if args.compact:
        command.append("--render-compact")
    environment = dict(os.environ)
    environment["SLINT_SCALE_FACTOR"] = "1"
    completed = subprocess.run(command, cwd=repo, env=environment, capture_output=True,
                               text=True, timeout=30, check=True)
    with Image.open(ppm) as captured:
        if captured.size != (args.width, args.height):
            raise RuntimeError(f"native pixel size {captured.size} differs from requested {(args.width, args.height)}")
        captured.save(png)
    metadata = {
        "spec": "095-brand-foundation", "method": "Slint Window::take_snapshot of actual shown AppWindow",
        "candidate_head": args.candidate_head, "binary_source_sha": manifest["source"]["git_sha"],
        "tree_sha": args.expected_tree, "binary_sha256": binary_sha256,
        "platform": platform.platform(), "route": args.route, "theme": args.theme,
        "logical_width": args.width, "logical_height": args.height, "pixel_width": args.width,
        "pixel_height": args.height, "scale_factor": 1, "compact": args.compact,
        "fixture": "existing synthetic projections and fresh nonsynced temporary synthetic vault",
        "png_sha256": hashlib.sha256(png.read_bytes()).hexdigest(),
        "stdout": completed.stdout.strip(), "stderr": completed.stderr.strip(),
        "visual_inspection_complete": False, "wcag_qualified": False, "release_ready": False,
    }
    record.write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    print(f"NATIVE_CAPTURE_OK={png}")


if __name__ == "__main__":
    main()
