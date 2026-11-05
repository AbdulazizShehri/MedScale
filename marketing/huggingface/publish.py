#!/usr/bin/env python3
"""Publish the approved Hugging Face showcase from this repository.

Needs the huggingface_hub package and an authenticated MedScale account with write
access to the two specific Spaces. Never place tokens in code or CI logs.
"""
from __future__ import annotations
import argparse
import os
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent
TARGETS = (
    ("MedScale/MedScale", PROJECT_ROOT / "space"),
    ("MedScaleAI/README", PROJECT_ROOT / "organization"),
)

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true", help="Validate the files without writing")
    args = parser.parse_args()
    for repo_id, folder in TARGETS:
        for file in (folder / "README.md", folder / "index.html"):
            if not file.is_file():
                raise SystemExit(f"Missing publish file for {repo_id}: {file}")
        md = (folder / "README.md").read_text(encoding="utf-8")
        html = (folder / "index.html").read_text(encoding="utf-8")
        if "sdk: static" not in md or "app_file: index.html" not in md:
            raise SystemExit(f"{repo_id}: invalid static Space metadata")
        if not html.lstrip().lower().startswith("<!doctype html>"):
            raise SystemExit(f"{repo_id}: index.html missing doctype")
        print(f"Validated {repo_id} ({len(md)}B README, {len(html)}B HTML)")

    if args.dry_run:
        print("DRY RUN. No external writes or changes were made.")
        return 0

    try:
        from huggingface_hub import HfApi
    except ImportError as exc:
        raise SystemExit("Install with: python -m pip install huggingface_hub") from exc

    token = os.environ.get("HF_TOKEN")
    api = HfApi(token=token)  # If HF_TOKEN is absent, uses an existing CLI login.
    identity = api.whoami()
    current_user = str(identity.get("name", ""))
    if current_user.lower() != "medscale":
        raise SystemExit(f"Refusing to publish as {current_user!r}; log in as @MedScale.")

    # Never overwrite the organization card's pre-existing files without an
    # explicit snapshot. A missing README Space is created as a public Space.
    for repo_id, folder in TARGETS:
        if not api.repo_exists(repo_id=repo_id, repo_type="space"):
            if repo_id != "MedScaleAI/README":
                raise SystemExit(f"Expected personal Space does not exist: {repo_id}")
            print("Creating public organization-card Space MedScaleAI/README")
            api.create_repo(repo_id=repo_id, repo_type="space", space_sdk="static", private=False)
        info = api.repo_info(repo_id=repo_id, repo_type="space")
        if getattr(info, "private", False):
            raise SystemExit(f"Refusing to publish privately: {repo_id}")
        print("Publishing", repo_id)
        api.upload_folder(
            repo_id=repo_id,
            repo_type="space",
            folder_path=str(folder),
            commit_message="docs: present MedScale product and research professionally",
            allow_patterns=["README.md", "index.html"],
        )
        print("Published https://huggingface.co/spaces/" + repo_id)
    print("Profile fields and avatars still require manual UI edit; see PROFILE-FIELDS.md.")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
