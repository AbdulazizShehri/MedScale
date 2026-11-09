# SPDX-License-Identifier: Apache-2.0
"""Conformance vectors for the Rust ports in crates/medscale-clinical.

MedScale's Core can't run Python, so each check it uses is ported to Rust and
must reproduce this reference package's reports exactly. This script runs the
reference on fixed synthetic inputs and writes inputs plus expected reports.
`tests/test_conformance_vectors.py` fails when the checked-in file is stale.

Usage: python conformance/generate_vectors.py [--check]
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
REPO_ROOT = PACKAGE_ROOT.parents[1]
VECTOR_DIR = REPO_ROOT / "crates" / "medscale-clinical" / "tests" / "vectors"

sys.path.insert(0, str(PACKAGE_ROOT / "src"))

from medscale_clinical.clinical.evidence_span_audit import audit_evidence_spans  # noqa: E402

DIGEST = "sha256:" + "ab" * 32

SPAN_CASES: list[tuple[str, list[list[object]]]] = [
    ("empty", []),
    ("single", [["note-1", "e1", 0, 10]]),
    ("exact_duplicate", [["note-1", "e1", 5, 9], ["note-1", "e2", 5, 9]]),
    ("nested", [["note-1", "outer", 0, 20], ["note-1", "inner", 4, 8]]),
    ("partial", [["note-1", "a", 0, 10], ["note-1", "b", 5, 15]]),
    ("touching_is_not_overlap", [["note-1", "a", 0, 5], ["note-1", "b", 5, 9]]),
    ("cross_source", [["note-1", "a", 0, 10], ["note-2", "b", 3, 6]]),
    ("digest_ids_kept", [[DIGEST, DIGEST, 1, 4], [DIGEST, "plain", 2, 3]]),
    ("ids_trimmed", [["  note-1 ", " e1", 0, 4], ["note-1", "e2", 1, 2]]),
    ("non_ascii_ids", [["ملاحظة-1", "دليل", 0, 6], ["ملاحظة-1", "é", 2, 4]]),
    (
        "mixed_unordered",
        [
            ["note-3", "z", 40, 60],
            ["note-1", "a", 0, 10],
            ["note-1", "b", 0, 10],
            ["note-1", "c", 2, 5],
            ["note-2", "d", 8, 12],
            ["note-1", "e", 9, 30],
            ["note-3", "y", 50, 55],
            ["note-3", "x", 55, 70],
        ],
    ),
]

ERROR_CASES: list[tuple[str, list[list[object]]]] = [
    ("empty_range", [["note-1", "a", 0, 4], ["note-1", "b", 7, 7]]),
    ("reversed_range", [["note-1", "a", 9, 3]]),
    ("blank_source", [["note-1", "a", 0, 4], ["   ", "b", 0, 4]]),
    ("blank_evidence", [["note-1", "", 0, 4]]),
]


def _records(rows: list[list[object]]) -> list[dict[str, object]]:
    return [
        {"source_id": s, "evidence_id": e, "start": a, "end": b} for s, e, a, b in rows
    ]


def evidence_span_audit_vectors() -> dict[str, object]:
    cases = []
    for name, rows in SPAN_CASES:
        report = audit_evidence_spans(_records(rows))
        cases.append(
            {
                "name": name,
                "spans": rows,
                "expected": report.to_dict(),
                "expected_json": report.to_json(),
            }
        )
    errors = []
    for name, rows in ERROR_CASES:
        try:
            audit_evidence_spans(_records(rows))
        except (TypeError, ValueError) as exc:
            errors.append({"name": name, "spans": rows, "error": str(exc)})
        else:
            raise AssertionError(f"{name} was expected to fail")
    return {
        "module": "medscale_clinical.clinical.evidence_span_audit",
        "generator": "integrations/clinical-evidence-python/conformance/generate_vectors.py",
        "cases": cases,
        "errors": errors,
    }


VECTORS = {"evidence_span_audit.json": evidence_span_audit_vectors}


def render(builder) -> str:
    return json.dumps(builder(), ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def main(argv: list[str]) -> int:
    check = "--check" in argv
    stale = []
    for name, builder in VECTORS.items():
        path = VECTOR_DIR / name
        text = render(builder)
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                stale.append(name)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8", newline="\n")
    if stale:
        print("stale conformance vectors: " + ", ".join(stale), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
