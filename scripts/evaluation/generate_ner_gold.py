"""Generate MedScale's synthetic biomedical NER gold sets (deterministic).

Three single-type sets matching the OpenMed NER families: disease
(DiseaseDetect, `DISEASE`), chemical/drug (PharmaDetect, `CHEM`) and anatomy
(AnatomyDetect, `Anatomy`). Sentences are written for MedScale; terms are
common, unambiguous examples. Each sentence also mentions entities of the
other types as distractors, so a model that tags them loses precision.

Boundary convention: the gold span is the term itself without laterality or
other modifiers ("knee" in "left knee", "diabetes mellitus" in "type 2
diabetes mellitus" is annotated as the full disease name "type 2 diabetes
mellitus"). Training corpora differ here, so overlap scores are reported
beside strict ones.

Usage: generate_ner_gold.py OUT_DIR
"""

from __future__ import annotations

import json
import os
import sys

DISEASES = ["hypertension", "asthma", "type 2 diabetes mellitus", "pneumonia", "migraine",
            "atrial fibrillation", "rheumatoid arthritis", "chronic kidney disease", "anemia",
            "hypothyroidism"]
CHEMICALS = ["metformin", "lisinopril", "aspirin", "amoxicillin", "atorvastatin", "warfarin",
             "levothyroxine", "ibuprofen", "insulin", "prednisone"]
# Paired body parts, so "left"/"right" always reads naturally.
ANATOMY = ["knee", "kidney", "shoulder", "lung", "hip", "ankle", "elbow", "wrist", "eye", "ear"]

# Each template uses {d} disease, {c} chemical, {a} anatomy slots.
TEMPLATES = [
    "The patient has {d} and takes {c} daily.",
    "{c} was started for {d} after the visit.",
    "She reported pain in the left {a} and a history of {d}.",
    "Imaging of the {a} was normal; {c} was continued.",
    "He was admitted with {d} and pain in the {a}.",
    "The dose of {c} was reduced because of {d}.",
    "Examination of the {a} showed no abnormality.",
    "A history of {d} was noted, and {c} was prescribed.",
    "Swelling of the right {a} improved after {c}.",
    "Follow-up for {d} is planned in three months.",
]

SETS = {
    "disease": {"slot": "d", "label": "DISEASE", "map": {"disease": "DISEASE"}},
    "chemical": {"slot": "c", "label": "CHEMICAL", "map": {"chem": "CHEMICAL", "chemical": "CHEMICAL"}},
    "anatomy": {"slot": "a", "label": "ANATOMY", "map": {"anatomy": "ANATOMY"}},
}


def build(kind: str) -> dict:
    spec = SETS[kind]
    documents = []
    for i in range(40):
        template = TEMPLATES[i % len(TEMPLATES)]
        cycle = i // len(TEMPLATES)  # shifts the terms each pass, so no sentence repeats
        values = {"d": DISEASES[(i + cycle) % len(DISEASES)],
                  "c": CHEMICALS[(i * 3 + 2 * cycle) % len(CHEMICALS)],
                  "a": ANATOMY[(i * 7 + 3 * cycle) % len(ANATOMY)]}
        text, spans, pos = "", [], 0
        while pos < len(template):
            brace = template.find("{", pos)
            if brace < 0:
                text += template[pos:]
                break
            text += template[pos:brace]
            slot = template[brace + 1]
            v = values[slot]
            if not text:
                v = v[0].upper() + v[1:]  # sentence-initial capital
            start = len(text)
            text += v
            if slot == spec["slot"]:
                spans.append({"start": start, "end": len(text), "label": spec["label"], "text": v})
            pos = brace + 3
        documents.append({"id": f"ner-{kind}-en-{i:02d}", "language": "en", "text": text, "spans": spans})
    return {
        "name": f"medscale-ner-{kind}-synthetic-en",
        "version": "1",
        "provenance": "Synthetic sentences written for MedScale by scripts/evaluation/generate_ner_gold.py; "
                      "no real person or record.",
        "license": "CC0-1.0",
        "labels": [spec["label"]],
        "label_map": spec["map"],
        "documents": documents,
    }


if __name__ == "__main__":
    out_dir = sys.argv[1]
    for kind in SETS:
        path = os.path.join(out_dir, f"ner-{kind}-synthetic-en-v1.json")
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            json.dump(build(kind), f, ensure_ascii=False, indent=1)
            f.write("\n")
        print(f"wrote {path}")
