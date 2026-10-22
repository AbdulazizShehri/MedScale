#!/usr/bin/env python3
"""Verify and materialize MedScale's pinned, offline model-catalog bundle.

No network requests. Repository identities in source.json are provenance and
must not be replaced with MedScale product branding.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / "third_party" / "medscale-model-catalog-v3"
REVISION = re.compile(r"[a-fA-F0-9]{40}\Z")
SHA256 = re.compile(r"[a-fA-F0-9]{64}\Z")


def verify_bundle(metadata: Path, snapshot: Path) -> dict[str, str]:
    source = json.loads(metadata.read_text(encoding="utf-8"))
    if not isinstance(source, dict) or set(source) != {
        "repository", "commit", "manifest_sha256"
    }:
        raise ValueError("invalid MedScale catalog metadata fields")
    if not isinstance(source["repository"], str) or not source["repository"].strip():
        raise ValueError("missing actual catalog repository identity")
    if not isinstance(source["commit"], str) or not REVISION.fullmatch(source["commit"]):
        raise ValueError("invalid pinned catalog revision")
    if not isinstance(source["manifest_sha256"], str) or not SHA256.fullmatch(source["manifest_sha256"]):
        raise ValueError("invalid pinned catalog digest")
    data = snapshot.read_bytes()
    if hashlib.sha256(data).hexdigest() != source["manifest_sha256"].lower():
        raise ValueError("MedScale catalog checksum mismatch")
    if not data or len(data) > 64 * 1024 * 1024:
        raise ValueError("MedScale catalog size outside safety budget")
    if not data.endswith(b"\n"):
        raise ValueError("MedScale catalog JSONL has incomplete final row")
    return source


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify bundled MedScale catalog without network")
    parser.add_argument("--output", type=Path, help="copy verified bytes to this path, atomically")
    args = parser.parse_args()

    source = verify_bundle(BUNDLE / "source.json", BUNDLE / "models.jsonl")
    commit = os.environ.get("CATALOG_COMMIT")
    repository = os.environ.get("CATALOG_REPOSITORY")
    if commit is not None and commit != source["commit"]:
        raise ValueError("workflow/catalog pinned revision mismatch")
    if repository is not None and repository != source["repository"]:
        raise ValueError("workflow/catalog source identity mismatch")
    if args.output:
        out = args.output
        out.parent.mkdir(parents=True, exist_ok=True)
        tmp = None
        try:
            with tempfile.NamedTemporaryFile(dir=out.parent, prefix=".medscale-catalog-", delete=False) as handle:
                tmp = Path(handle.name)
                handle.write((BUNDLE / "models.jsonl").read_bytes())
                handle.flush()
                os.fsync(handle.fileno())
            os.replace(tmp, out)
        finally:
            if tmp is not None:
                tmp.unlink(missing_ok=True)
    print("MEDSCALE_CATALOG_BUNDLE verified sha256=" + source["manifest_sha256"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
