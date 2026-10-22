#!/usr/bin/env python3
"""Prevent internal agent files and non-public plans from being published.

This is a tracked-PATH gate: it deliberately does not inspect Git history or
assert that old public commits, archives, forks or caches are inaccessible.
"""
from __future__ import annotations

from pathlib import PurePosixPath
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
    print("PASS: public tracked paths contain no internal agent files or private plan files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
