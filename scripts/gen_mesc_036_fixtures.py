#!/usr/bin/env python3
"""Generate synthetic MESC release fixtures for Spec 036 (not a real MESC release)."""

from __future__ import annotations

import hashlib
import json
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "evidence" / "012-mesc-artifact-integration" / "fixtures"


def hx(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def write_good(good: Path) -> None:
    good.mkdir(parents=True, exist_ok=True)
    model = b"synthetic-model-bytes-v1"
    sbom = b'{"bomFormat":"CycloneDX","specVersion":"1.5","components":[]}'
    notice = b"Synthetic NOTICE - no redistribution rights claimed.\n"
    (good / "model.bin").write_bytes(model)
    (good / "sbom.json").write_bytes(sbom)
    (good / "NOTICE").write_bytes(notice)
    manifest = {
        "schema_version": 1,
        "producer_id": "synthetic.medscale.mesc",
        "release_id": "syn-036-001",
        "release_tag": "syn-v0.0.1",
        "source_commit": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "source_tree": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "model_id": "syn-model",
        "tokenizer_id": "syn-tok",
        "base_model_id": "syn-base",
        "corpus_id": "syn-corpus",
        "training_receipt_digest_hex": hx(b"train-receipt"),
        "evaluation_receipt_digest_hex": hx(b"eval-receipt"),
        "sbom_path": "sbom.json",
        "sbom_digest_hex": hx(sbom),
        "rights_license": "SYNTHETIC-NO-RIGHTS",
        "rights_notice_path": "NOTICE",
        "provenance_note": "synthetic fixture for Spec 036; not a real MESC release",
        "limitations": [
            "not a real MESC release",
            "product admit remains gate-blocked",
        ],
        "runtime_requirements": "offline-fixture-only",
        "epoch": 1,
        "artifacts": [
            {
                "kind": "model_weights",
                "path": "model.bin",
                "byte_length": len(model),
                "sha256_hex": hx(model),
            },
            {
                "kind": "sbom",
                "path": "sbom.json",
                "byte_length": len(sbom),
                "sha256_hex": hx(sbom),
            },
            {
                "kind": "notice",
                "path": "NOTICE",
                "byte_length": len(notice),
                "sha256_hex": hx(notice),
            },
        ],
    }
    (good / "manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )


def main() -> None:
    if ROOT.exists():
        shutil.rmtree(ROOT)
    ROOT.mkdir(parents=True)
    good = ROOT / "synthetic-good"
    write_good(good)

    def clone(name: str) -> Path:
        d = ROOT / name
        shutil.copytree(good, d)
        return d

    clone("missing-manifest")
    (ROOT / "missing-manifest" / "manifest.json").unlink()

    clone("digest-mismatch")
    model_len = len((ROOT / "digest-mismatch" / "model.bin").read_bytes())
    (ROOT / "digest-mismatch" / "model.bin").write_bytes(b"X" * model_len)

    d = clone("size-mismatch")
    m = json.loads((d / "manifest.json").read_text(encoding="utf-8"))
    m["artifacts"][0]["byte_length"] = 1
    (d / "manifest.json").write_text(json.dumps(m, indent=2) + "\n", encoding="utf-8")

    clone("missing-file")
    (ROOT / "missing-file" / "model.bin").unlink()

    d = clone("bad-schema")
    m = json.loads((d / "manifest.json").read_text(encoding="utf-8"))
    m["schema_version"] = 99
    (d / "manifest.json").write_text(json.dumps(m, indent=2) + "\n", encoding="utf-8")

    d = clone("unknown-field")
    m = json.loads((d / "manifest.json").read_text(encoding="utf-8"))
    m["evil_extra"] = True
    (d / "manifest.json").write_text(json.dumps(m, indent=2) + "\n", encoding="utf-8")

    d = clone("duplicate-path")
    m = json.loads((d / "manifest.json").read_text(encoding="utf-8"))
    m["artifacts"].append(dict(m["artifacts"][0]))
    (d / "manifest.json").write_text(json.dumps(m, indent=2) + "\n", encoding="utf-8")

    clone("missing-rights")
    (ROOT / "missing-rights" / "NOTICE").unlink()

    clone("missing-sbom")
    (ROOT / "missing-sbom" / "sbom.json").unlink()

    print("wrote", sorted(p.name for p in ROOT.iterdir()))


if __name__ == "__main__":
    main()
