"""Turn a finished `model-qualification-batch` run into per-row evidence.

Reads each job log of the run (through the GitHub CLI), keeps the jobs that
succeeded and printed `SNAPSHOT_EVIDENCE` (repository and executed revision),
takes each job's weight file and test from `qualification/batch.json` at the
run's commit, and prints:
- Rust `ExecutedModel` entries for `crates/medscale-pack/src/compatibility.rs`;
- a Markdown summary of every job, failures included, for the evidence record.

Only rows that actually ran are recorded; failures are listed with their
first panic line and never become evidence.

Usage: record_results.py RUN_ID [--repo TheHalfMoon/MedScale]
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys

RUNTIME_BY_TEST = {
    "openmed_snapshot_external_103": "tract_onnx",
    "gliner_external_104": "candle_gliner",
}


def gh(*args: str) -> str:
    return subprocess.run(["gh", *args], check=True, capture_output=True, text=True, encoding="utf-8").stdout


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("run")
    parser.add_argument("--repo", default="TheHalfMoon/MedScale")
    args = parser.parse_args()

    run = json.loads(gh("run", "view", args.run, "--repo", args.repo, "--json", "jobs,headSha"))
    jobs = run["jobs"]
    batch_raw = gh(
        "api",
        f"repos/{args.repo}/contents/qualification/batch.json?ref={run['headSha']}",
        "-H",
        "Accept: application/vnd.github.raw+json",
    )
    batch = {entry["id"]: entry for entry in json.loads(batch_raw)}
    entries, summary = [], []
    for job in jobs:
        if not job["name"].startswith("qualify"):
            continue
        log = gh("api", f"repos/{args.repo}/actions/jobs/{job['databaseId']}/logs")
        log = re.sub(r"\x1b\[[0-9;]*m", "", log)
        lines = [re.sub(r"^\S+Z ", "", l) for l in log.splitlines()]
        evidence = next((l for l in lines if l.startswith("SNAPSHOT_EVIDENCE")), None)
        job_id = job["name"].removeprefix("qualify (").removesuffix(")")
        entry = batch.get(job_id)
        panic = next((l for l in lines if "panicked" in l or l.startswith("error:")), "")
        detail = next((l for l in lines if l.startswith(("model must", "called `Result"))), "")
        if job["conclusion"] != "success" or not evidence or not entry:
            summary.append(f"| {job['name']} | {job['conclusion']} | {(detail or panic)[:200]} |")
            continue
        fields = dict(kv.split("=", 1) for kv in evidence.split()[1:] if "=" in kv)
        if fields.get("repo") != entry.get("repo"):
            summary.append(f"| {job['name']} | mismatch | evidence repo {fields.get('repo')} != batch entry {entry.get('repo')} |")
            continue
        weights, test = entry["weights"], entry["test"]
        runtime = RUNTIME_BY_TEST.get(test)
        if runtime is None:
            runtime = "candle_mlx" if weights == "weights.safetensors" else "candle_safetensors"
        entries.append(
            "    ExecutedModel {\n"
            f'        repo_id: "{fields["repo"]}",\n'
            f'        revision: "{fields["revision"]}",\n'
            f'        runtime: "{runtime}",\n'
            f'        weight_file: "{weights}",\n'
            f'        evidence_run: "{args.run}",\n'
            "    },"
        )
        timing = " ".join(f"{k}={v}" for k, v in fields.items() if k.endswith("_ms"))
        summary.append(f"| {fields['repo']} @ `{fields['revision'][:8]}` | success | {runtime}, {weights}, {timing} |")
    print("// --- EXECUTED_MODELS entries ---")
    print("\n".join(entries))
    print("\n<!-- summary -->\n| Job | Result | Detail |\n|---|---|---|")
    print("\n".join(summary))
    return 0


if __name__ == "__main__":
    sys.exit(main())
