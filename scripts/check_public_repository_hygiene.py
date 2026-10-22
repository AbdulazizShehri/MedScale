#!/usr/bin/env python3
"""Prevent internal agent files and non-public plans from being published.

This is a tracked-PATH gate: it deliberately does not inspect Git history or
assert that old public commits, archives, forks or caches are inaccessible.
"""
from __future__ import annotations

from pathlib import Path, PurePosixPath
import subprocess
import sys

BLOCKED_FILES = {
    "AGENTS.md",
    "CURSOR.md",
    "docs/product/POST_RELEASE_ROADMAP.md",
    "docs/brand/FULL_MEDScale_RENAME_EXECUTION.md",
    "docs/brand/MEDSCALE_NATIVE_PROJECT_IDENTITY.md",
    "docs/brand/PRE_RELEASE_IDENTITY_AND_PROVENANCE_AUDIT.md",
}
BLOCKED_PREFIXES = (
    ".cursor/", ".specify/", ".impeccable/",
    "docs/planning/", "docs/matrices/",
)
BLOCKED_BASENAMES = {
    "plan.md", "tasks.md", "research.md", "requirements.md",
    "analyze-notes.md", "converge-notes.md", "MUSE_EXECUTION_PLAN.md",
    "clarifications.md", "quickstart.md",
}


def violations(paths: list[str]) -> list[str]:
    result: list[str] = []
    for p in paths:
        name = PurePosixPath(p).name
        forbidden = (
            p in BLOCKED_FILES
            or p.startswith(BLOCKED_PREFIXES)
            or p.startswith("docs/community/OPEN_SOURCE_HANDOFF_")
            or p.startswith("docs/product/RELEASE_CANDIDATE_")
            or p.startswith("docs/brand/RENAME_INVENTORY_")
            or p.startswith("docs/brand/ACTIVE_OWNED_PATH_MIGRATION_")
            or p.startswith("specs/007-openmed-capability-absorption-parity/")
            or p.startswith("specs/071-openmed-evidence-center/")
            or (p.startswith("specs/") and name in BLOCKED_BASENAMES)
        )
        if forbidden:
            result.append(p)
    return sorted(result)


def main() -> int:
    tracked = subprocess.check_output(["git", "ls-files", "-z"]).decode().split("\0")
    found = violations([p for p in tracked if p])
    if found:
        print("ERROR: private MedScale operational files tracked in public repository:")
        print("\n".join(f" - {p}" for p in found))
        return 1
    # Keep the public release-gate summary factual without republishing
    # maintainer operations, workstation details or internal strategy.
    gates_path = Path("docs/qualification/EXTERNAL_GATES.md")
    if gates_path.is_file():
        public_gates = gates_path.read_text(encoding="utf-8")
        required = (
            "REAL_PHI_AUTHORIZATION", "NOT_AUTHORIZED",
            "DESKTOP_RELEASE_SIGNING_PROVENANCE",
            "QUALIFIED_RELEASE_PERFORMANCE_HARDWARE",
            "MACOS_SIGNED_PRODUCT_QUALIFICATION",
            "FINAL_V0_UI_ACCESSIBILITY_QUALIFICATION",
            "Spec 072", "including Models/Evidence",
        )
        forbidden = (
            "Cursor behavior", "LOCAL_WINDOWS_TOOLCHAIN_DESTRUCTION",
            "OCR_ENGINE_LLM_ENDPOINT", "FOUNDER_REVIEW_POLICY_AMENDMENT",
            "TAURI_DESKTOP_MIGRATION_DECISION_2026",
            "FOUNDER_PRODUCTIZATION_REVIEW_AMENDMENT",
            "docs/planning/",
        )
        missing = [item for item in required if item not in public_gates]
        leaked = [item for item in forbidden if item in public_gates]
        if missing or leaked:
            print(f"ERROR: public qualification summary invalid: missing={missing} leaked={leaked}")
            return 1
    else:
        print("ERROR: public qualification gate summary missing")
        return 1
    print("PASS: public paths and qualification status are free of internal plans")
    return 0


if __name__ == "__main__":
    sys.exit(main())
