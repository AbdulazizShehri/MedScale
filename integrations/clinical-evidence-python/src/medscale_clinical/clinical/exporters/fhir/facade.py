# SPDX-License-Identifier: Apache-2.0
# Modified from maziyarpanahi/openmed@5b79cae5 (openmed/clinical/exporters/fhir/facade.py): renamed package/symbols to medscale_clinical / MedScale.
"""Canonical public facade for deterministic grounded-span FHIR export.

The implementation remains in :mod:`.grounded` so the established import path
and the top-level facade are the same callable rather than competing export
pipelines.
"""

from __future__ import annotations

from .grounded import (
    COREFERENCE_EVIDENCE_EXTENSION_URL,
    FHIR_RESOURCE_TYPES,
    FHIRBundle,
    FHIRExportSummary,
    to_fhir,
)

__all__ = [
    "COREFERENCE_EVIDENCE_EXTENSION_URL",
    "FHIRBundle",
    "FHIRExportSummary",
    "FHIR_RESOURCE_TYPES",
    "to_fhir",
]
