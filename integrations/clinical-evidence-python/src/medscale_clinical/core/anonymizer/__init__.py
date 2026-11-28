# SPDX-License-Identifier: Apache-2.0
"""Minimal boundary stub: synthetic Faker generation is excluded from zero-dep runtime."""

class Anonymizer:
    def __init__(self, *args, **kwargs):
        raise NotImplementedError("Synthetic Faker anonymizer is excluded from zero-dep medscale_clinical")
