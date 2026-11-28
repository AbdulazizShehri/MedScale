#!/usr/bin/env python3
"""Generate deterministic Wave 1 conformance vectors for `crates/medscale-pack`.

Writes `crates/medscale-pack/tests/vectors/model_ecosystem_v4.json` using
only standard-library `medscale_clinical` v4 modules.
"""

from __future__ import annotations

import json
import pathlib
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
REPO = pathlib.Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "src"))

from medscale_clinical.core.device_capabilities import (  # noqa: E402
    MAX_BUDGET_BYTES,
    MIN_BUDGET_BYTES,
    MIB,
    budget_bucket,
)
from medscale_clinical.core.manifest_diff import diff_manifests  # noqa: E402
from medscale_clinical.core.model_card import render_model_card  # noqa: E402
from medscale_clinical.core.model_search import recommend_models  # noqa: E402
from medscale_clinical.core.portable_bundle import (  # noqa: E402
    canonical_json_bytes,
    content_id,
)
from medscale_clinical.core.portable_bundle_calibration import (  # noqa: E402
    CALIBRATION_METHODS,
    DECISION_CALIBRATION_EXTENSION,
    PPM_SCALE,
    DecisionCalibrationReason,
)


def synthetic_manifest_rows() -> list[dict]:
    return [
        {
            "repo_id": "OpenMed/Synthetic-Phone-NER",
            "family": "NER",
            "task": "token-classification",
            "languages": ["en", "ar"],
            "tier": "Tiny",
            "param_count": 33_000_000,
            "architecture": "bert",
            "base_model": "synthetic-base",
            "formats": ["onnx", "pytorch"],
            "canonical_labels": ["DISEASE", "DRUG"],
            "benchmark": {"f1": 0.89, "recall": 0.91},
            "arxiv": "2501.00001",
            "license": "apache-2.0",
            "reproducibility_hash": "0" * 64,
            "released": "2026-01-01",
            "disk_mb": 120.0,
            "download_mb": 110.0,
            "download_sizes": {"onnx": 110.0},
            "peak_ram_mb": {"phone": 1800.0, "laptop": 1800.0},
            "recommended_tier": "phone",
            "latency_ms": {"phone": 42.0, "laptop": 12.0},
            "script_coverage": {"Latin": 0.99, "Arabic": 0.98},
        },
        {
            "repo_id": "OpenMed/Synthetic-Laptop-PII",
            "family": "PII",
            "task": "token-classification",
            "languages": ["en"],
            "tier": "Base",
            "param_count": 110_000_000,
            "architecture": "deberta-v2",
            "base_model": "synthetic-deberta",
            "formats": ["onnx"],
            "canonical_labels": ["NAME", "ID_NUM"],
            "benchmark": {"recall": 0.96, "f1": 0.94},
            "arxiv": "2501.00002",
            "license": "apache-2.0",
            "reproducibility_hash": "1" * 64,
            "released": "2026-02-01",
            "disk_mb": 420.0,
            "download_mb": 390.0,
            "peak_ram_mb": 7500.0,
            "recommended_tier": "laptop",
            "script_coverage": {
                "Latin": 1.0,
                "Arabic": 0.95,
                "Bengali": 0.9,
                "Cyrillic": 0.9,
                "Devanagari": 0.9,
                "CJK": 0.9,
                "Greek": 0.9,
                "Hangul": 0.9,
                "Hebrew": 0.9,
                "Japanese": 0.9,
                "Thai": 0.9,
            },
        },
        {
            "repo_id": "OpenMed/Synthetic-Unenriched-Fallback",
            "family": "NER",
            "task": "token-classification",
            "languages": ["en"],
            "tier": "Small",
            "param_count": 66_000_000,
            "architecture": "roberta",
            "base_model": "synthetic-roberta",
            "formats": ["onnx"],
            "canonical_labels": ["PROBLEM"],
            "benchmark": {"f1": 0.87},
            "arxiv": "2501.00003",
            "license": "mit",
            "reproducibility_hash": "2" * 64,
            "released": "2026-03-01",
        },
        {
            "repo_id": "OpenMed/Synthetic-Server-Large",
            "family": "NER",
            "task": "token-classification",
            "languages": ["en"],
            "tier": "XLarge",
            "param_count": 1_500_000_000,
            "architecture": "modernbert",
            "base_model": "synthetic-xl",
            "formats": ["pytorch"],
            "canonical_labels": ["PROCEDURE"],
            "benchmark": {"f1": 0.98},
            "arxiv": "2501.00004",
            "license": "apache-2.0",
            "reproducibility_hash": "3" * 64,
            "released": "2026-04-01",
            "peak_ram_mb": 82_000.0,
            "recommended_tier": "server",
        },
    ]


def build_vectors() -> dict:
    rows = synthetic_manifest_rows()
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = pathlib.Path(tmpdir)
        m1 = tmp / "manifest_old.jsonl"
        m2 = tmp / "manifest_new.jsonl"
        m1.write_text("".join(json.dumps(r) + "\n" for r in rows), encoding="utf-8")

        rows_v2 = [dict(r) for r in rows[1:]]
        rows_v2[0] = {
            **rows_v2[0],
            "param_count": 115_000_000,
            "formats": ["pytorch", "onnx"],
            "benchmark": {"recall": 0.97, "f1": 0.95},
        }
        rows_v2.append(
            {
                **rows[0],
                "repo_id": "OpenMed/Synthetic-Added-Tiny",
            }
        )
        m2.write_text("".join(json.dumps(r) + "\n" for r in rows_v2), encoding="utf-8")

        rec_cases = {}
        for tier in ("phone", "laptop", "workstation", "server"):
            res = recommend_models(
                device_tier=tier,
                task="token-classification",
                language="en",
                manifest_path=m1,
            )
            rec_cases[tier] = [r.repo_id for r in res]

        diff = diff_manifests(m1, m2)
        card = render_model_card(rows[0])

    jcs_inputs = [
        {"b": 2, "a": 1, "nested": {"z": True, "m": [3, 1, 2]}},
        {"unicode": "مرحبا", "empty": {}, "zero": 0},
    ]
    jcs_vectors = [
        {
            "input": obj,
            "canonical_utf8": canonical_json_bytes(obj).decode("utf-8"),
            "content_id": content_id(canonical_json_bytes(obj)),
        }
        for obj in jcs_inputs
    ]

    bucket_inputs = [0, 1, 512 * MIB, (512 * MIB) + 1, 4096 * MIB, MAX_BUDGET_BYTES, MAX_BUDGET_BYTES * 4]
    bucket_vectors = [
        {"input_bytes": val, "bucket_bytes": budget_bucket(val)}
        for val in bucket_inputs
    ]

    return {
        "schema_version": 1,
        "manifest_rows": rows,
        "recommendations_by_tier": rec_cases,
        "manifest_diff": {
            "added": list(diff.added),
            "removed": list(diff.removed),
            "changed_repos": [c.repo_id for c in diff.changed],
        },
        "model_card_sample": {
            "repo_id": rows[0]["repo_id"],
            "rendered": card,
        },
        "jcs_vectors": jcs_vectors,
        "calibration_constants": {
            "extension": DECISION_CALIBRATION_EXTENSION,
            "ppm_scale": PPM_SCALE,
            "methods": sorted(CALIBRATION_METHODS),
            "reason_codes": [r.value for r in DecisionCalibrationReason],
        },
        "device_capability_buckets": {
            "min_budget_bytes": MIN_BUDGET_BYTES,
            "max_budget_bytes": MAX_BUDGET_BYTES,
            "cases": bucket_vectors,
        },
    }


def main() -> int:
    out = REPO / "crates" / "medscale-pack" / "tests" / "vectors" / "model_ecosystem_v4.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    data = build_vectors()
    out.write_text(json.dumps(data, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"Wrote {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
