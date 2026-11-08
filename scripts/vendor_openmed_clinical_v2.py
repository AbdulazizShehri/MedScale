#!/usr/bin/env python3
# Note: this records the original import. Later revisions (explicit exports,
# per-file change notices, the file inventory) are listed in
# integrations/clinical-evidence-python/UPSTREAM.md under "Revisions after import".
"""Copy the second, review-only OpenMed clinical safety tranche into MedScale.

This is a developer-only, source-pinned, auditable vendor script. It performs
no model inference or runtime network calls. Never executes upstream code.
"""
from __future__ import annotations

import argparse
import subprocess
from pathlib import Path

UPSTREAM = "maziyarpanahi/openmed"
COMMIT = "3e2323288ec551157c0b943b331f399ef1434649"
REPO = Path(__file__).resolve().parents[1]
DEST = REPO / "integrations" / "clinical-evidence-python"
PACKAGE = DEST / "src" / "medscale_clinical"
# Reviewed, separate from the first tranche. Each module has an upstream
# corresponding synthetic test in tests/unit/clinical/.
MODULES = (
    "citation_boundaries",
    "evidence_coverage",
    "evidence_table",
    "summary_omission_budget",
    "summary_length_budget",
    "summary_empty_evidence",
    "summary_demographic_minimizer",
    "summary_output_normalizer",
    "summary_retry_control",
    "summary_profiles",
    "summary_section_plan",
    "review_queue_sla",
    "review_state_machine",
    "refusal_taxonomy",
    "output_use_policy",
    "uncertainty_disclosure",
    "uncertainty_sources",
    "decision_support",
    "lab_reference_ranges",
    "nli_hypothesis_guard",
    "nli_premise_window",
    "nli_gate",
    "nli_labels",
    "nli_batch_plan",
    "sdoh_completeness",
    "sdoh_sensitive_use",
)

def load(source: str, local: Path | None) -> str:
    if local is not None:
        return (local / source).read_text(encoding="utf-8-sig")
    output = subprocess.run(
        ["gh", "api", "-H", "Accept: application/vnd.github.raw",
         f"repos/{UPSTREAM}/contents/{source}?ref={COMMIT}"],
        check=True, capture_output=True,
    )
    return output.stdout.decode("utf-8-sig")

def adapt(s: str) -> str:
    return (s.replace("openmed", "medscale_clinical")
             .replace("OPENMED", "MEDSCALE")
             .replace("OpenMed", "MedScale"))

def write(destination: Path, content: str) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(content, encoding="utf-8", newline="\n")

def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path,
                        help="An independently verified OpenMed checkout at the pinned commit")
    args = parser.parse_args()
    src_root = args.source_root
    if src_root is not None:
        current = subprocess.check_output(
            ["git", "-C", str(src_root), "rev-parse", "HEAD"], text=True
        ).strip()
        if current != COMMIT:
            parser.error("source checkout does not match pinned OpenMed commit")
    for module in MODULES:
        source = f"openmed/clinical/{module}.py"
        target = PACKAGE / "clinical" / f"{module}.py"
        if target.exists():
            parser.error(f"refusing to overwrite existing MedScale module: {target}")
        content = adapt(load(source, src_root))
        write(target, "# SPDX-License-Identifier: Apache-2.0\n"
              "# Adapted from maziyarpanahi/openmed at pinned commit. See UPSTREAM.md.\n"
              + content)
        test_source = f"tests/unit/clinical/test_{module}.py"
        test_target = DEST / "tests" / f"test_{module}.py"
        if test_target.exists():
            parser.error(f"refusing to overwrite test: {test_target}")
        test_text = adapt(load(test_source, src_root))
        if module == "sdoh_completeness":
            # The upstream test combines pure auditing with an entire separate
            # SDOH extractor/section parser. Keep the full reference unchanged
            # and execute its original seven stand-alone audit tests only.
            write(DEST / "tests" / "upstream_reference" /
                  "sdoh_completeness_full.py.txt", test_text)
            test_text = test_text.split(
                '\n@pytest.mark.parametrize(\n    "language,text",', 1
            )[0]
            start = test_text.index("from medscale_clinical.clinical.sdoh import (")
            end = test_text.index(")\n", start) + 2
            test_text = test_text[:start] + test_text[end:]
            test_text = test_text.replace(
                "from medscale_clinical.clinical.sections import detect_sections\n",
                "",
            )
        if module == "nli_labels":
            # Upstream has one integration test requiring the full NLI verifier;
            # retain that original test as reference but only run the pure label tests.
            write(DEST / "tests" / "upstream_reference" /
                  "nli_labels_full.py.txt", test_text)
            test_text = test_text.split(
                "def test_public_nli_result_preserves_verifier_api():", 1
            )[0]
        if module == "review_state_machine":
            # The first upstream test requires a separate persistence-linked
            # review_transitions implementation, not included in this library.
            write(DEST / "tests" / "upstream_reference" /
                  "review_state_machine_full.py.txt", test_text)
            start = test_text.index(
                "def test_existing_review_transition_module_preserves_both_public_apis()"
            )
            end = test_text.index("def _event(", start)
            test_text = test_text[:start] + test_text[end:]
        write(test_target, test_text.rstrip() + "\n")
        print(f"Copied {module} and selected source-derived tests")

    # Export the new units via the existing package namespace, without importing
    # an OpenMed monolithic package. Incremental API list follows the vendor list.
    init_file = PACKAGE / "clinical" / "__init__.py"
    exports = "".join(
        f"from .{name} import *  # noqa: F401,F403\n" for name in MODULES
    )
    # Re-export all original PR #477 names at the end, preserving its
    # EvidenceSpan and other public contracts where names collide.
    old = init_file.read_text(encoding="utf-8")
    docstring, baseline_exports = old.split("\n", 1)
    extra_aliases = (
        "from .lab_reference_ranges import fingerprint_source "
        "as fingerprint_lab_reference_source\n"
        "from .evidence_coverage import fingerprint_source "
        "as fingerprint_source\n"
    )
    init_file.write_text(
        docstring + "\n" + exports + baseline_exports + extra_aliases,
        encoding="utf-8",
        newline="\n",
    )
    print(f"Added {len(MODULES)} deterministic modules and {len(MODULES)} test files")

if __name__ == "__main__":
    main()
