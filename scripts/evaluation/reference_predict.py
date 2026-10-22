"""Run pinned OpenMed (PyPI `openmed==3.0.0`) on a MedScale gold set.

Head-to-head benchmark driver (Spec 103). It loads one locally downloaded
model snapshot (the catalog-pinned revision MedScale also runs) with
OpenMed's own `analyze_text` defaults, and writes the predicted spans plus
same-runner timing for MedScale's scorer
(`crates/medscale-pack/tests/reference_python_external_103.rs`).

Usage: reference_predict.py GOLD_JSON MODEL_DIR REPO REVISION OUT_JSON

Synthetic gold text only. Evaluation evidence, not clinical validation.
"""
import json
import resource
import sys
import time

import openmed
from openmed import analyze_text


def main() -> int:
    gold_path, model_dir, repo, revision, out_path = sys.argv[1:6]
    gold = json.load(open(gold_path, encoding="utf-8"))
    loader = openmed.ModelLoader() if hasattr(openmed, "ModelLoader") else None
    kwargs = {"loader": loader} if loader is not None else {}

    predictions: dict[str, list[dict]] = {}
    doc_ms: list[float] = []
    cold_ms = None
    for i, doc in enumerate(gold["documents"]):
        started = time.perf_counter()
        result = analyze_text(doc["text"], model_name=model_dir, **kwargs)
        elapsed = (time.perf_counter() - started) * 1000.0
        if i == 0:
            # First call: model load plus one document (OpenMed's cold start).
            cold_ms = elapsed
            started = time.perf_counter()
            result = analyze_text(doc["text"], model_name=model_dir, **kwargs)
            elapsed = (time.perf_counter() - started) * 1000.0
        doc_ms.append(elapsed)
        spans = []
        for e in result.entities:
            if e.start is None or e.end is None:
                continue
            spans.append({"start": int(e.start), "end": int(e.end), "label": str(e.label)})
        predictions[doc["id"]] = spans

    out = {
        "repo": repo,
        "revision": revision,
        "openmed_version": getattr(openmed, "__version__", "unknown"),
        "settings": "analyze_text defaults (aggregation_strategy=simple, sentence_detection=True)",
        "cold_start_ms": cold_ms,
        "doc_latency_ms": doc_ms,
        # Linux reports ru_maxrss in KiB.
        "peak_rss_kib": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
        "predictions": predictions,
    }
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(out, f)
    print(f"REFERENCE_PREDICTIONS repo={repo} docs={len(predictions)} cold_start_ms={cold_ms:.0f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
