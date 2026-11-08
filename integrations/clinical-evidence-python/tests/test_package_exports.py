# SPDX-License-Identifier: Apache-2.0
"""The package namespace names exactly one object per public name."""

import ast
from pathlib import Path

import pytest

import medscale_clinical.clinical as clinical
from medscale_clinical.clinical import evidence_coverage, review_state_machine
from medscale_clinical.core import audit

INIT = Path(clinical.__file__)


def test_exports_are_explicit_and_unique():
    tree = ast.parse(INIT.read_text(encoding="utf-8"))
    bound = []
    for node in tree.body:
        if isinstance(node, ast.ImportFrom):
            assert all(alias.name != "*" for alias in node.names), "wildcard import"
            bound.extend(alias.asname or alias.name for alias in node.names)
    assert len(bound) == len(set(bound)), "a name is imported twice"
    assert sorted(bound) == sorted(clinical.__all__)
    for name in clinical.__all__:
        assert hasattr(clinical, name), name


def test_review_state_is_the_state_machines():
    assert clinical.ReviewState is review_state_machine.ReviewState
    assert clinical.REVIEW_STATES is review_state_machine.REVIEW_STATES
    assert clinical.CoverageReviewState is evidence_coverage.ReviewState
    assert clinical.COVERAGE_REVIEW_STATES is evidence_coverage.REVIEW_STATES


def test_upstream_features_not_vendored_fail_loudly(tmp_path):
    with pytest.raises(ValueError):
        audit.manifest_hash()
    with pytest.raises(FileNotFoundError):
        audit.manifest_hash(tmp_path / "missing.jsonl")
    manifest = tmp_path / "models.jsonl"
    manifest.write_bytes(b"{}\n")
    assert audit.manifest_hash(manifest) == audit.manifest_hash(manifest)
    with pytest.raises(NotImplementedError):
        audit.AuditReport.attest(object(), "eu", model_artifacts={})
