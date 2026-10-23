#!/usr/bin/env python3
"""Publish MedScaleAI organization card to its public HF README Space only."""
import os
from pathlib import Path
from huggingface_hub import HfApi

REPO = "MedScaleAI/README"
ROOT = Path(__file__).resolve().parents[1]
CARD = ROOT / "huggingface" / "organization" / "README.md"
HOME = ROOT / "huggingface" / "organization" / "index.html"

def main() -> None:
    token = os.environ.get("MEDSCALE_HF_WRITE_TOKEN")
    if not token:
        raise SystemExit("Missing MEDSCALE_HF_WRITE_TOKEN; set a write-scoped Actions secret, never commit it.")
    if not CARD.is_file() or not HOME.is_file():
        raise SystemExit("Missing organization card source file(s).")
    card = CARD.read_text(encoding="utf-8")
    for term in ("MedScale Beta", "synthetic", "not clinically validated", "medscale.si"):
        if term.lower() not in card.lower():
            raise SystemExit(f"Mandatory beta disclosure missing: {term}")
    api = HfApi(token=token)
    api.create_repo(repo_id=REPO, repo_type="space", space_sdk="static", private=False, exist_ok=True)
    if api.space_info(REPO).private:
        raise SystemExit("Organization card Space is private; no public card could be confirmed.")
    api.upload_file(path_or_fileobj=str(CARD), path_in_repo="README.md", repo_id=REPO, repo_type="space", commit_message="docs: publish MedScale Beta organization card")
    api.upload_file(path_or_fileobj=str(HOME), path_in_repo="index.html", repo_id=REPO, repo_type="space", commit_message="docs: publish static org-card landing")
    print("Uploaded MedScaleAI public org-card sources to https://huggingface.co/spaces/MedScaleAI/README")

if __name__ == "__main__":
    main()
