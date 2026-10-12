"""Select the next batch of catalog rows for per-row execution qualification.

Input: the paged JSON output of `medscale packs catalog` (MedScale's own
`runtime_expectation` for every row), so a batch only contains rows the code
says should run. Rows already recorded as executed are skipped, and at most
one row is taken per (family, architecture, runtime, language, size tier)
group, so each batch widens coverage instead of repeating near-duplicates.
Weight files are chosen from Hub sizes so every artifact fits the unchanged
1 GiB Pack bound (ONNX: `model.onnx`, else `model_fp16.onnx`, else
`model_int8.onnx`).

Qualification tooling only (network: Hugging Face API for file sizes).

Usage:
  medscale packs catalog --snapshot models.jsonl --commit <sha> --limit 500 --offset N > page_N.json
  select_batch.py page_0.json page_500.json ... --count 24 --out qualification/batch.json
"""

from __future__ import annotations

import argparse
import json
import sys
import time
import urllib.request

BOUND = 1 << 30

TEXT = {
    "en": "Patient John Smith, born 12 March 1980, has type 2 diabetes and pain in the left knee.",
    "nl": "Patiënt Jan de Vries, geboren op 12 maart 1980, woont in Utrecht en heeft diabetes.",
    "fr": "Le patient Jean Dupont, né le 12 mars 1980, habite à Lyon et souffre de diabète.",
    "de": "Der Patient Hans Müller, geboren am 12. März 1980, wohnt in Berlin und hat Diabetes.",
    "es": "La paciente María García, nacida el 3 de mayo de 1975, vive en Sevilla y tiene diabetes.",
    "it": "Il paziente Mario Rossi, nato il 12 marzo 1980, vive a Milano e ha il diabete.",
    "pt": "O paciente João Silva, nascido em 12 de março de 1980, mora em Lisboa e tem diabetes.",
    "hi": "मरीज़ राहुल शर्मा का जन्म 12 मार्च 1980 को हुआ और उन्हें मधुमेह है।",
    "te": "రోగి రవి కుమార్ 1980 మార్చి 12న జన్మించారు మరియు హైదరాబాద్‌లో నివసిస్తున్నారు.",
    "ja": "患者の山田太郎さんは1980年3月12日生まれで、東京に住んでいます。",
    "ko": "환자 김민수는 1980년 3월 12일에 태어났으며 서울에 살고 있습니다.",
    "zh": "患者张伟，1980年3月12日出生，住在北京。",
    "tr": "Hasta Ahmet Yılmaz, 12 Mart 1980 doğumlu ve İstanbul'da yaşıyor.",
    "ar": "المريض أحمد علي، مواليد 12 مارس 1980، يسكن في الرياض ويعاني من السكري.",
}
ZERO_SHOT_LABELS = "disease, drug, anatomy, gene"


def hub_sizes(repo: str) -> dict[str, int]:
    url = f"https://huggingface.co/api/models/{repo}?blobs=true"
    for attempt in range(3):
        try:
            with urllib.request.urlopen(url, timeout=30) as response:
                data = json.load(response)
            return {s["rfilename"]: s.get("size") or 0 for s in data.get("siblings", [])}
        except Exception:  # noqa: BLE001 - qualification tooling, retried
            time.sleep(2 * (attempt + 1))
    return {}


def choose(row: dict, kind: str, sizes: dict[str, int]):
    formats = [f.lower() for f in row["formats"]]
    if kind == "expected_runnable":
        for name in ("model.onnx", "model_fp16.onnx", "model_int8.onnx"):
            if 0 < sizes.get(name, 0) <= BOUND:
                return name, "openmed_snapshot_external_103", "tract_onnx"
        return None
    if kind == "expected_runnable_zero_shot":
        if 0 < sizes.get("weights.safetensors", 0) <= BOUND:
            return "weights.safetensors", "gliner_external_104", "candle_gliner"
        return None
    if kind == "expected_runnable_safetensors":
        if "mlx-fp" in formats:
            name, runtime = "weights.safetensors", "candle_mlx"
        else:
            name, runtime = "model.safetensors", "candle_safetensors"
        if 0 < sizes.get(name, 0) <= BOUND:
            return name, "safetensors_external_104", runtime
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("pages", nargs="+")
    parser.add_argument("--count", type=int, default=24)
    parser.add_argument("--skip", nargs="*", default=[], help="repo ids to skip (for example earlier failures)")
    parser.add_argument("--out", default="qualification/batch.json", help="UTF-8 JSON output path")
    args = parser.parse_args()

    rows = []
    for page in args.pages:
        rows.extend(json.load(open(page, encoding="utf-8"))["rows"])
    executed_groups = set()
    candidates = []
    for row in sorted(rows, key=lambda r: r["repo_id"]):
        expectation = row["runtime_expectation"]
        kind = expectation["kind"]
        lang = (row.get("languages") or ["en"])[0]
        family = row.get("family")
        if kind == "executed_tested":
            runtime = expectation.get("runtime")
            executed_groups.add((family, row.get("architecture"), runtime, lang, row.get("tier")))
            continue
        if kind in {"expected_runnable", "expected_runnable_safetensors", "expected_runnable_zero_shot"}:
            if row["repo_id"] not in args.skip:
                candidates.append((row, kind, lang, family))

    batch, taken = [], set()
    for row, kind, lang, family in candidates:
        if len(batch) >= args.count:
            break
        runtime_guess = {
            "expected_runnable": "tract_onnx",
            "expected_runnable_zero_shot": "candle_gliner",
        }.get(kind, "candle_mlx" if "mlx-fp" in [f.lower() for f in row["formats"]] else "candle_safetensors")
        group = (family, row.get("architecture"), runtime_guess, lang, row.get("tier"))
        if group in executed_groups or group in taken:
            continue
        picked = choose(row, kind, hub_sizes(row["repo_id"]))
        if picked is None:
            continue
        weights, test, _runtime = picked
        taken.add(group)
        entry = {
            "id": f"{len(batch):02d}-{row['repo_id'].split('/')[-1][:48]}",
            "repo": row["repo_id"],
            "weights": weights,
            "test": test,
            "text": TEXT.get(lang, TEXT["en"]),
            "labels": ZERO_SHOT_LABELS if test == "gliner_external_104" else "",
        }
        batch.append(entry)
        print(f"selected {entry['id']} ({weights})", file=sys.stderr)
    with open(args.out, "w", encoding="utf-8", newline="\n") as out:
        json.dump(batch, out, ensure_ascii=False, indent=1)
        out.write("\n")
    print(f"{len(batch)} rows written to {args.out}", file=sys.stderr)
    return 0 if batch else 1


if __name__ == "__main__":
    sys.exit(main())
