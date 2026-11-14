#!/usr/bin/env python3
"""Spec 069 deterministic shape specialization for the pinned HF NER qualification model."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from pathlib import Path

import onnx

PINNED_ONNX_VERSION = "1.19.0"
SOURCE_SHA256 = "b324e829f1fad3b897f926d1a1d1372803c6d04546831a3a2ed103652b916adf"
DERIVED_SHA256 = "183057ca3123d7bd09fda062738390b326b1298c58d0eee39f0003f5a8920f14"
TOKENIZER_SHA256 = "343989712a36cd8b253efeaf8baf6a08b9d2583f78e395e83832e8ee9f8d8ee1"
REVISION = "9faa2f4a2d59b396888b318f596ff719cc893f1e"
REPOSITORY = "onnx-community/bert-base-NER-ONNX"
SEQUENCE_LENGTH = 128
EXPECTED_REPLACEMENTS = {"batch_size": 637, "sequence_length": 680}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def specialize_graph(graph: onnx.GraphProto, counts: dict[str, int]) -> None:
    for value in [*graph.input, *graph.output, *graph.value_info]:
        tensor = value.type.tensor_type
        if not tensor.HasField("shape"):
            continue
        for dim in tensor.shape.dim:
            if dim.dim_param == "batch_size":
                dim.ClearField("dim_param")
                dim.dim_value = 1
                counts["batch_size"] += 1
            elif dim.dim_param == "sequence_length":
                dim.ClearField("dim_param")
                dim.dim_value = SEQUENCE_LENGTH
                counts["sequence_length"] += 1
    for node in graph.node:
        for attr in node.attribute:
            if attr.type == onnx.AttributeProto.GRAPH:
                specialize_graph(attr.g, counts)
            elif attr.type == onnx.AttributeProto.GRAPHS:
                for subgraph in attr.graphs:
                    specialize_graph(subgraph, counts)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-model", type=Path, required=True)
    parser.add_argument("--source-tokenizer", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()

    if onnx.__version__ != PINNED_ONNX_VERSION:
        raise SystemExit(f"onnx=={PINNED_ONNX_VERSION} required; found {onnx.__version__}")
    if sha256(args.source_model) != SOURCE_SHA256:
        raise SystemExit("source ONNX SHA-256 does not match the pinned Hugging Face artifact")
    if sha256(args.source_tokenizer) != TOKENIZER_SHA256:
        raise SystemExit("tokenizer SHA-256 does not match the pinned Hugging Face artifact")

    output = args.output_dir
    output.mkdir(parents=True, exist_ok=True)
    model = onnx.load(str(args.source_model))
    counts = {"batch_size": 0, "sequence_length": 0}
    specialize_graph(model.graph, counts)
    if counts != EXPECTED_REPLACEMENTS:
        raise SystemExit(f"unexpected symbolic-dimension replacement counts: {counts}")
    onnx.save(model, str(output / "model.onnx"))
    if sha256(output / "model.onnx") != DERIVED_SHA256:
        raise SystemExit("derived ONNX SHA-256 is not reproducible")

    shutil.copyfile(args.source_tokenizer, output / "tokenizer.json")
    labels = ["O", "B-MISC", "I-MISC", "B-PER", "I-PER", "B-ORG", "I-ORG", "B-LOC", "I-LOC"]
    (output / "labels.json").write_text(json.dumps(labels, indent=2) + "\n", encoding="utf-8")
    metadata = {
        "export_format": "onnx",
        "fixed_sequence_length": SEQUENCE_LENGTH,
        "license_id": "MIT",
        "original_file": "onnx/model_int8.onnx",
        "repository": REPOSITORY,
        "revision": REVISION,
        "runtime_family": "tract_onnx_token_classification_v1",
        "source_kind": "hugging_face",
        "source_sha256": SOURCE_SHA256,
        "task": "token-classification",
        "transform": "onnx_graph_wide_shape_specialization(batch=1,sequence=128)",
        "upstream_rights_uri": f"https://huggingface.co/{REPOSITORY}/blob/{REVISION}/README.md",
    }
    (output / "model.meta.json").write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"source_sha256={SOURCE_SHA256}")
    print(f"derived_sha256={DERIVED_SHA256}")
    print(f"replacements={counts}")


if __name__ == "__main__":
    main()
