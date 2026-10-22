"""No-network MedScale catalog integrity tests."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from scripts.verify_medscale_catalog_bundle import verify_bundle


class BundledCatalogTests(unittest.TestCase):
    def test_pinned_bundle_has_matching_digest(self):
        root = Path(__file__).resolve().parents[2] / "third_party" / "medscale-model-catalog-v3"
        metadata = verify_bundle(root / "source.json", root / "models.jsonl")
        self.assertEqual(
            hashlib.sha256((root / "models.jsonl").read_bytes()).hexdigest(),
            metadata["manifest_sha256"],
        )
        self.assertEqual(len(metadata["commit"]), 40)

    def test_checksum_tampering_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="medscale-catalog-test-") as temporary:
            root = Path(temporary)
            source = root / "source.json"
            manifest = root / "models.jsonl"
            manifest.write_bytes(b'{"repo_id":"synthetic/model"}\n')
            source.write_text(
                json.dumps({
                    "repository": "synthetic/catalog",
                    "commit": "1" * 40,
                    "manifest_sha256": hashlib.sha256(manifest.read_bytes()).hexdigest(),
                }),
                encoding="utf-8",
            )
            verify_bundle(source, manifest)
            manifest.write_bytes(b'{"repo_id":"synthetic/other"}\n')
            with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                verify_bundle(source, manifest)

    def test_ambiguous_source_descriptor_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="medscale-catalog-test-") as temporary:
            root = Path(temporary)
            source = root / "source.json"
            manifest = root / "models.jsonl"
            manifest.write_bytes(b'{"repo_id":"synthetic/model"}\n')
            source.write_text(
                json.dumps({"repository": "synthetic/catalog", "commit": "1" * 40}),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ValueError, "metadata fields"):
                verify_bundle(source, manifest)


if __name__ == "__main__":
    unittest.main()
