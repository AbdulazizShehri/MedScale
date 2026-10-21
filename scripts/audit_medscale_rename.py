#!/usr/bin/env python3
"""Reproducible, non-destructive MedScale naming inventory.

Lists tracked source references; NEVER rewrites source, provenance or benchmark data.
"""
from __future__ import annotations

import json
import re
import subprocess
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TERMS = re.compile(r"openmed|mesc|everhealth|medorigin", re.IGNORECASE)
OUT_JSON = Path("docs/brand/RENAME_INVENTORY_2026-10-21.json")
OUT_MD = Path("docs/brand/RENAME_INVENTORY_2026-10-21.md")


def category(path: str) -> str:
    if path.startswith(("third_party/", "vendor/")):
        return "SOURCE_LEGAL_OR_VENDOR"
    if path.startswith(("evidence/", "specs/", "docs/planning/", "docs/matrices/",
                        "docs/legal/", ".specify/")):
        return "HISTORICAL_OR_SCIENTIFIC_RECORD"
    if path.startswith("apps/desktop-tauri/"):
        return "ACTIVE_PRODUCT"
    if path.startswith("crates/medscale-cli/"):
        return "ACTIVE_CLI"
    if path.startswith("crates/medscale-desktop/ui/"):
        return "LEGACY_UI"
    if path.startswith("crates/") and "/tests/" in path:
        return "COMPATIBILITY_TEST"
    if path.startswith("crates/"):
        return "FIRST_PARTY_TECHNICAL"
    if path.startswith((".github/", "scripts/")):
        return "BUILD_OR_EVALUATION"
    return "OTHER_REVIEW"


def inventory() -> list[dict]:
    tracked = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT).split(b"\0")
    records = []
    exclusions = {OUT_JSON.as_posix(), OUT_MD.as_posix(),
                  "scripts/audit_medscale_rename.py"}
    for raw in tracked:
        if not raw:
            continue
        rel = raw.decode("utf-8", errors="replace")
        if rel in exclusions:
            continue
        path = ROOT / rel
        if not path.is_file():
            continue
        try:
            content = path.read_text(encoding="utf-8")
        except (UnicodeError, OSError):
            continue
        matches = TERMS.findall(content)
        if matches:
            counts = dict(sorted(Counter(s.lower() for s in matches).items()))
            records.append({"path": rel, "category": category(rel),
                            "occurrences": counts, "total": len(matches)})
    return sorted(records, key=lambda item: item["path"])


def main() -> None:
    rows = inventory()
    category_counts = Counter(row["category"] for row in rows)
    terms = Counter()
    for row in rows:
        terms.update(row["occurrences"])
    data = {"purpose": "Name migration inventory; no source mutation",
            "categories": dict(sorted(category_counts.items())),
            "term_counts": dict(sorted(terms.items())),
            "files_with_matches": len(rows), "files": rows}
    (ROOT / OUT_JSON).write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n",
                                encoding="utf-8")
    lines = [
        "# MedScale source-name migration inventory",
        "",
        "Generated using `py -3 scripts/audit_medscale_rename.py` against tracked files.",
        "",
        "**Interpretation:** A match is not proof of non-compliant branding. "
        "Immutable measurements, third-party copyright notices, model identifiers, "
        "and required scientific citations must remain accurate. This inventory "
        "tracks first-party migration work without modifying source content.",
        "",
        f"- Files containing tracked source-name terms: **{len(rows)}**",
        f"- Occurrences: **{sum(terms.values())}**",
        "",
        "| Disposition | Matched files |",
        "|---|---:|",
    ]
    lines.extend(f"| {key} | {count} |" for key, count in sorted(category_counts.items()))
    lines.extend(["", "## First-party and customer-facing files requiring review", "",
                  "| Category | Path | Matching terms |", "|---|---|---|"])
    active = {"ACTIVE_PRODUCT", "ACTIVE_CLI", "LEGACY_UI", "FIRST_PARTY_TECHNICAL",
              "OTHER_REVIEW"}
    for row in rows:
        if row["category"] in active:
            lines.append(f"| {row['category']} | `{row['path']}` | "
                         f"{', '.join(f'{k}: {v}' for k, v in row['occurrences'].items())} |")
    lines.extend(["", "## Next actions", "",
                  "1. Rebrand active product copy and first-party names wherever semantics permit.",
                  "2. For persisted IDs, schemas and APIs: design backwards-compatible migrations and tests.",
                  "3. For external model repository identifiers: mirror actual permitted assets and validate "
                  "resolution before switching functional URLs or claiming a MedScale-owned model.",
                  "4. Keep science, license notices, benchmark provenance and historic evidence accurate.",
                  "5. Verify exact-head CI, Windows/macOS packages and E2E before merging/releasing.",
                  ""])
    (ROOT / OUT_MD).write_text("\n".join(lines), encoding="utf-8")
    print(f"Matched files: {len(rows)}; total occurrences: {sum(terms.values())}")
    print("Categories: " + str(dict(sorted(category_counts.items()))))


if __name__ == "__main__":
    main()
