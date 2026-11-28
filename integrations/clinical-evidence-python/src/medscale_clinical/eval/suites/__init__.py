# SPDX-License-Identifier: Apache-2.0
"""Boundary stub: external/DUA benchmark suites are excluded from zero-dep reference runtime."""
GOLDEN = "golden"

def validate_suite_name(name: str) -> str:
    if name != GOLDEN:
        raise NotImplementedError("External/DUA benchmark suites are excluded from zero-dep medscale_clinical")
    return name

def load_suite_fixtures(*args, **kwargs):
    raise NotImplementedError("External/DUA benchmark fixtures are excluded from zero-dep medscale_clinical")
