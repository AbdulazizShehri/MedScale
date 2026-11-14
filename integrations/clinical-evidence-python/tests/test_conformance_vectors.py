# SPDX-License-Identifier: Apache-2.0
"""The Rust ports' conformance vectors must match this reference package."""

import importlib.util
from pathlib import Path

GENERATOR = Path(__file__).resolve().parents[1] / "conformance" / "generate_vectors.py"


def _generator():
    spec = importlib.util.spec_from_file_location("generate_vectors", GENERATOR)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_checked_in_vectors_match_the_reference():
    generator = _generator()
    for name, builder in generator.VECTORS.items():
        path = generator.VECTOR_DIR / name
        assert path.exists(), f"missing {path}; run conformance/generate_vectors.py"
        assert path.read_text(encoding="utf-8") == generator.render(builder), (
            f"{name} is stale; run conformance/generate_vectors.py"
        )
