# SPDX-License-Identifier: Apache-2.0
# Modified from maziyarpanahi/openmed@5b79cae5 (openmed/core/schemas/__init__.py): renamed package/symbols to medscale_clinical / MedScale.
"""Versioned MedScale schema records and JSON Schema loaders."""

from .span import (
    CURRENT_SCHEMA_VERSION,
    SCHEMA_NAMES,
    MedScaleSpan,
    SchemaDriftResult,
    build_schema_snapshot,
    compare_all_schema_drift,
    compare_schema_drift,
    current_schema_version,
    hmac_text_hash,
    load_all_schemas,
    load_schema,
    load_schema_bundle,
    load_schema_snapshot,
    schema_fingerprint,
)

__all__ = [
    "CURRENT_SCHEMA_VERSION",
    "SCHEMA_NAMES",
    "OpenMedSpan",
    "SchemaDriftResult",
    "build_schema_snapshot",
    "compare_all_schema_drift",
    "compare_schema_drift",
    "current_schema_version",
    "hmac_text_hash",
    "load_all_schemas",
    "load_schema",
    "load_schema_bundle",
    "load_schema_snapshot",
    "schema_fingerprint",
]
