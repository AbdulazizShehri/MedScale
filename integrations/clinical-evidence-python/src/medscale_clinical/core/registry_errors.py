# SPDX-License-Identifier: Apache-2.0
# Modified from maziyarpanahi/openmed@5b79cae5 (openmed/core/registry_errors.py): renamed package/symbols to medscale_clinical / MedScale.
"""Shared exception types for the family and slot registry APIs."""


class RegistryError(RuntimeError):
    """Base error for offline registry operations."""


class RegistryStateError(RegistryError):
    """Raised when committed registry state is invalid or incoherent."""


class RegistryGateError(RegistryError):
    """Raised when a pointer target lacks matching releasable gate evidence."""
