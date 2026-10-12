"""Compare a MedScale document run with the Hugging Face reference model.

Qualification-only harness (runs on `qual/**` CI branches, never in the
product). It loads the acquired snapshot with Hugging Face Transformers
(`local_files_only=True`, safetensors only, no remote code), tokenizes with
the snapshot's own `tokenizer.json` exactly as MedScale does, decodes the
logits with OpenMed v3.0.0's ONNX decoder (`openmed/onnx/inference.py`
`_decode_entities` / `_split_label` / `_flush_entity`, ea920f36, Apache-2.0,
transcribed below), and compares the entities with MedScale's
`SNAPSHOT_DOCUMENT` line.

MedScale trims whitespace at span edges (a documented difference), so spans
are compared on their trimmed text. Scores must agree within --tolerance.

Usage: hf_reference_compare.py SNAPSHOT_DIR TEXT QUALIFICATION_LOG
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def split_label(raw: str) -> tuple[str, str]:
    normalized = str(raw).strip()
    if len(normalized) > 2 and normalized[1] in {"-", "_"}:
        prefix = normalized[0].upper()
        if prefix in {"B", "I", "E", "L", "S", "U"}:
            return prefix, normalized[2:]
    return "", normalized


def decode(logits, offsets, id2label, text):
    import numpy as np

    shifted = logits - logits.max(axis=-1, keepdims=True)
    probs = np.exp(shifted)
    probs /= probs.sum(axis=-1, keepdims=True)
    label_ids = probs.argmax(axis=-1)
    scores = probs.max(axis=-1)
    entities, current = [], None

    def flush(cur):
        if cur is None:
            return None
        score = sum(cur["scores"]) / len(cur["scores"])
        if 0 <= cur["start"] < cur["end"] <= len(text):
            entities.append(
                {
                    "label": cur["label"],
                    "score": score,
                    "text": text[cur["start"] : cur["end"]],
                }
            )
        return None

    for label_id, score, (start, end) in zip(label_ids, scores, offsets):
        prefix, label = split_label(id2label.get(int(label_id), f"LABEL_{label_id}"))
        if start == end or label.upper() == "O":
            current = flush(current)
            continue
        starts_new = (
            current is None
            or current["label"] != label
            or prefix in {"B", "S", "U"}
            or (prefix not in {"I", "E", "L"} and start > current["end"])
        )
        if starts_new:
            current = flush(current)
            current = {"label": label, "start": start, "end": end, "scores": [float(score)]}
        else:
            current["end"] = max(current["end"], end)
            current["scores"].append(float(score))
        if prefix in {"E", "L", "S", "U"}:
            current = flush(current)
    flush(current)
    return entities


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("snapshot")
    parser.add_argument("text")
    parser.add_argument("log")
    parser.add_argument("--tolerance", type=float, default=1e-3)
    args = parser.parse_args()

    import torch
    from tokenizers import Tokenizer
    from transformers import AutoConfig, AutoModelForTokenClassification

    snapshot = Path(args.snapshot)
    config = AutoConfig.from_pretrained(snapshot, local_files_only=True)
    model = AutoModelForTokenClassification.from_pretrained(
        snapshot, local_files_only=True, use_safetensors=True, trust_remote_code=False
    ).eval()
    tokenizer = Tokenizer.from_file(str(snapshot / "tokenizer.json"))
    tokenizer.no_padding()
    tokenizer.no_truncation()
    encoding = tokenizer.encode(args.text)
    ids = torch.tensor([encoding.ids])
    with torch.no_grad():
        logits = model(input_ids=ids, attention_mask=torch.ones_like(ids)).logits[0].numpy()
    # Char offsets: `tokenizers` reports byte-free char offsets for str input.
    offsets = [tuple(o) for o in encoding.offsets]
    special = encoding.special_tokens_mask
    offsets = [(0, 0) if s else o for o, s in zip(offsets, special)]
    reference = decode(logits, offsets, config.id2label, args.text)

    line = next(
        (l for l in Path(args.log).read_text(encoding="utf-8").splitlines() if l.startswith("SNAPSHOT_DOCUMENT ")),
        None,
    )
    if line is None:
        print("REFERENCE_COMPARISON status=missing_medscale_output")
        return 2
    # Both external tests print the entity array last on the line (one may
    # prefix timing fields).
    medscale = json.loads(line[line.index("[") :])

    ref = [(e["label"], e["text"].strip(), e["score"]) for e in reference if e["text"].strip()]
    ours = [(e["label"], e["text"].strip(), e["score"]) for e in medscale]
    same_spans = [(l, t) for l, t, _ in ref] == [(l, t) for l, t, _ in ours]
    max_delta = max((abs(a[2] - b[2]) for a, b in zip(ref, ours)), default=0.0)
    ok = same_spans and max_delta <= args.tolerance
    print(
        "REFERENCE_COMPARISON "
        f"status={'match' if ok else 'mismatch'} reference_entities={len(ref)} "
        f"medscale_entities={len(ours)} max_score_delta={max_delta:.2e} tokens={len(encoding.ids)}"
    )
    if not ok:
        print("REFERENCE_ENTITIES " + json.dumps(ref, ensure_ascii=False))
        print("MEDSCALE_ENTITIES " + json.dumps(ours, ensure_ascii=False))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
