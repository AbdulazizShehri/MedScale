#!/usr/bin/env python3
# Note: this records the original import. Later revisions (explicit exports,
# per-file change notices, the file inventory) are listed in
# integrations/clinical-evidence-python/UPSTREAM.md under "Revisions after import".
"""Reproducibly import selected Apache-2.0 OpenMed clinical modules into MedScale.

Developer-only tool: never invoked by the MedScale runtime.  Source is pinned to
one upstream commit and copied without downloading checkpoints or datasets.
"""
from __future__ import annotations

import subprocess
from pathlib import Path

UPSTREAM = "maziyarpanahi/openmed"
COMMIT = "3e2323288ec551157c0b943b331f399ef1434649"
ROOT = Path(__file__).resolve().parents[1]
VENDOR = ROOT / "integrations" / "clinical-evidence-python"
PACKAGE = VENDOR / "src" / "medscale_clinical"
SOURCE_PATHS = [
    "openmed/clinical/evidence_span_audit.py",
    "openmed/clinical/nli_numeric_precheck.py",
    "openmed/clinical/claim_conflicts.py",
    "openmed/clinical/evidence_recency.py",
    "openmed/clinical/citation_ordering.py",
    "openmed/clinical/summary_citations.py",
    "openmed/clinical/citation_minimality.py",
    "openmed/clinical/claim_integrity.py",
    "openmed/clinical/units/__init__.py",
    "openmed/clinical/units/convert.py",
    "openmed/clinical/units/ucum.py",
    "openmed/clinical/lexicons/clinical_norm.py",
    "openmed/core/iso_temporal.py",
    "openmed/core/audit.py",
]
TEST_NAMES = [
    "test_evidence_span_audit.py",
    "test_nli_numeric_precheck.py",
    "test_claim_conflicts.py",
    "test_evidence_recency.py",
    "test_citation_ordering.py",
    "test_summary_citations.py",
    "test_citation_minimality.py",
    "test_claim_integrity.py",
]

def get(upstream_path: str) -> str:
    result = subprocess.run(
        ["gh", "api", "-H", "Accept: application/vnd.github.raw",
         f"repos/{UPSTREAM}/contents/{upstream_path}?ref={COMMIT}"],
        check=True, capture_output=True,
    )
    return result.stdout.decode("utf-8-sig")

def put(path: Path, value: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(value, encoding="utf-8", newline="\n")

def rebrand(source: str) -> str:
    # Keep original upstream license/NOTICE verbatim in third_party/.
    return (
        source.replace("openmed", "medscale_clinical")
        .replace("OPENMED", "MEDSCALE")
        .replace("OpenMed", "MedScale")
    )

def run() -> None:
    if (VENDOR / "FEATURE_PARITY_V2.md").exists():
        raise SystemExit(
            "Refusing tranche-1 refresh on a tranche-2 package: it would "
            "overwrite the new clinical exports. Use a fresh pre-v2 tree, "
            "then execute vendor_openmed_clinical_v2.py."
        )
    for source in SOURCE_PATHS:
        destination = PACKAGE / source.removeprefix("openmed/")
        original = get(source)
        adapted = rebrand(original)
        if not source.endswith("/__init__.py"):
            adapted = (
                "# SPDX-License-Identifier: Apache-2.0\n"
                "# Derived from maziyarpanahi/openmed; see UPSTREAM.md for origin and changes.\n"
                + adapted
            )
        put(destination, adapted)
        print(f"ported {source} -> {destination.relative_to(ROOT)}")

    put(PACKAGE / "__init__.py",
        '"""MedScale local-first clinical evidence verification (optional Python)."""\n')
    put(PACKAGE / "core" / "__init__.py", '"""Common validation utilities."""\n')
    put(PACKAGE / "clinical" / "lexicons" / "__init__.py",
        '"""Clinical normalization dictionaries."""\n')
    put(PACKAGE / "clinical" / "__init__.py",
        '"""Selected MedScale clinical evidence primitives. No model, network or PHI handling."""\n'
        "from .evidence_span_audit import *  # noqa: F401,F403\n"
        "from .summary_citations import *  # noqa: F401,F403\n"
        "from .citation_ordering import *  # noqa: F401,F403\n"
        "from .citation_minimality import *  # noqa: F401,F403\n"
        "from .nli_numeric_precheck import *  # noqa: F401,F403\n"
        "from .evidence_recency import *  # noqa: F401,F403\n"
        "from .claim_conflicts import *  # noqa: F401,F403\n"
        "from .claim_integrity import *  # noqa: F401,F403\n")

    for name in TEST_NAMES:
        src = "tests/unit/clinical/" + name
        dest = VENDOR / "tests" / name
        put(dest, rebrand(get(src)))
        print(f"ported upstream test {name}")
    put(VENDOR / "third_party" / "openmed" / "LICENSE", get("LICENSE"))
    put(VENDOR / "third_party" / "openmed" / "NOTICE", get("NOTICE"))
    put(VENDOR / "UPSTREAM.md",
        "# OpenMed-derived clinical evidence modules\n\n"
        f"- Upstream: https://github.com/{UPSTREAM}\n"
        f"- Exact source commit: `{COMMIT}` (branch `master` at acquisition)\n"
        "- Original source license: Apache-2.0 (see third_party/openmed/LICENSE).\n"
        "- Original upstream NOTICE preserved in third_party/openmed/NOTICE.\n"
        "- Changes: modules copied into the `medscale_clinical` namespace,\n"
        "  product identifiers replaced, selected imports adjusted, and minimal\n"
        "  package initializers supplied. All eight corresponding upstream test\n"
        "  modules were ported to the renamed namespace. No weights, datasets,\n"
        "  cloud adapters, or background network services were copied.\n"
        "- Integration: an isolated, optional Python source package. Importing it\n"
        "  does not automatically activate any clinical authority in MedScale's Rust\n"
        "  desktop or FHIR flows. Clinical outputs require qualification and review.\n"
        "- The copy script fetches from GitHub when explicitly run by a developer;\n"
        "  importing and executing the copied library does not fetch source code.\n"
    )
    put(VENDOR / "pyproject.toml",
        '[build-system]\nrequires = ["setuptools>=68"]\nbuild-backend = "setuptools.build_meta"\n\n'
        '[project]\nname = "medscale-clinical-evidence"\nversion = "0.1.0"\n'
        'description = "Local clinical evidence audit primitives for MedScale"\n'
        'requires-python = ">=3.11"\ndependencies = []\n'
        'license = "Apache-2.0"\n\n'
        '[project.optional-dependencies]\ntest = ["pytest>=8", "pytest-subtests>=0.13"]\n\n'
        '[tool.setuptools.packages.find]\nwhere = ["src"]\n\n'
        '[tool.pytest.ini_options]\ntestpaths = ["tests"]\n'
    )
    # Preserve the maintained integration guide on subsequent vendor refreshes.
    if not (VENDOR / "README.md").exists():
        put(VENDOR / "README.md",
        "# MedScale Clinical Evidence (Python)\n\n"
        "A vendored, renamed set of deterministic clinical evidence and citation\n"
        "audits from OpenMed. For local synthetic tests: `python -m pytest tests`\n"
        "with `PYTHONPATH=src`, or install this isolated package in a virtual env.\n\n"
        "This is NOT yet connected to MedScale's production Rust authority plane,\n"
        "and is not certified for patient care. No LLM/NLI model is bundled.\n"
        "Upstream provenance and legal notices: [UPSTREAM.md](UPSTREAM.md).\n"
    )
    print(f"DONE: {len(SOURCE_PATHS)} source files + {len(TEST_NAMES)} test files")

if __name__ == "__main__":
    run()
