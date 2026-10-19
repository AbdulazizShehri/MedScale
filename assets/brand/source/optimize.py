import json
import sys
from pathlib import Path

from symbol_fit import render, score, svg

q = json.loads(Path(sys.argv[1]).read_text())
steps = {k: (0.02 if k in ("ts", "bs") else 1.0 if k == "sw" else 4.0) for k in q}
best = score(render(svg(q), "opt"))
print("start", round(best, 4))
for rnd in range(int(sys.argv[2]) if len(sys.argv) > 2 else 3):
    improved = False
    for k in list(q):
        for sign in (1, -1):
            trial = dict(q)
            trial[k] = q[k] + sign * steps[k]
            s = score(render(svg(trial), "opt"))
            if s > best + 1e-5:
                best, q = s, trial
                improved = True
                print(f"round {rnd} {k} -> {q[k]:.2f}  IoU {best:.4f}", flush=True)
                break
    for k in steps:
        steps[k] = max(steps[k] / 2, 0.0025 if k in ("ts", "bs") else 0.5)
    Path(sys.argv[1].replace(".json", "-best.json")).write_text(json.dumps(q, indent=1))
    if not improved and all(v <= (0.0025 if k in ("ts", "bs") else 0.5) for k, v in steps.items()):
        break
print("best", round(best, 4), json.dumps(q))
