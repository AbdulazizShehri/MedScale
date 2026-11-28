#!/usr/bin/env python3
"""Deterministic, AST/token-safe source transplant from pinned OpenMed v4.0.0.

Preserves the 44 hand-verified files in `integrations/clinical-evidence-python/`
while automatically computing the clean transitive pure-stdlib closure from
curated OpenMed v4.0.0 roots, copying permissive non-Python resources (pre-compiling
tiny `.yaml` resources into `.yaml.json` + `_yaml_compat` for zero-dep runtime),
and building `@medscale/sdk` in `integrations/medscale-sdk-ts/`.
"""

from __future__ import annotations

import argparse
import ast
import hashlib
import io
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tokenize
from collections import deque

PINNED_OPENMED_COMMIT = "5b79cae5df6bc22b0fcdc799be480ee4ffc23c57"
PINNED_AGENT_COMMIT = "ded6e57b5b93cb6dc849c8639662b381179697c7"
PORT_DATE = "2026-11-28"

PRESERVE_EXISTING_REL = frozenset(
    {
        "__init__.py",
        "core/__init__.py",
        "core/audit.py",
        "core/iso_temporal.py",
        "clinical/__init__.py",
        "clinical/citation_boundaries.py",
        "clinical/citation_minimality.py",
        "clinical/citation_ordering.py",
        "clinical/claim_conflicts.py",
        "clinical/claim_integrity.py",
        "clinical/decision_support.py",
        "clinical/evidence_coverage.py",
        "clinical/evidence_recency.py",
        "clinical/evidence_span_audit.py",
        "clinical/evidence_table.py",
        "clinical/lab_reference_ranges.py",
        "clinical/lexicons/clinical_norm.py",
        "clinical/nli_batch_plan.py",
        "clinical/nli_gate.py",
        "clinical/nli_hypothesis_guard.py",
        "clinical/nli_labels.py",
        "clinical/nli_numeric_precheck.py",
        "clinical/nli_premise_window.py",
        "clinical/output_use_policy.py",
        "clinical/refusal_taxonomy.py",
        "clinical/review_queue_sla.py",
        "clinical/review_state_machine.py",
        "clinical/sdoh_completeness.py",
        "clinical/sdoh_sensitive_use.py",
        "clinical/summary_citations.py",
        "clinical/summary_demographic_minimizer.py",
        "clinical/summary_empty_evidence.py",
        "clinical/summary_length_budget.py",
        "clinical/summary_omission_budget.py",
        "clinical/summary_output_normalizer.py",
        "clinical/summary_profiles.py",
        "clinical/summary_retry_control.py",
        "clinical/summary_section_plan.py",
        "clinical/uncertainty_disclosure.py",
        "clinical/uncertainty_sources.py",
        "clinical/units/__init__.py",
        "clinical/units/convert.py",
        "clinical/units/ucum.py",
    }
)

MEDSPACY_ATTRIBUTION_HEADER = (
    "# Third-party attribution: Assertion, negation, temporality, and experiencer rules\n"
    "# include patterns adapted from medspaCy ConText (https://github.com/medspacy/medspacy,\n"
    f"# Apache-2.0), retrieved via OpenMed@5b79cae5 and ported on {PORT_DATE}.\n"
    "# Local modifications: namespace renamed to medscale_clinical and offline stdlib loader binding.\n"
)

MEDSPACY_TARGETS = frozenset(
    {
        "openmed/clinical/context.py",
        "openmed/clinical/negation_scope.py",
        "openmed/clinical/experiencer.py",
        "openmed/clinical/lexicons/context_cues.py",
        "openmed/clinical/data/context_rules.yaml",
    }
)

# Subtrees explicitly forbidden from entering the zero-dep reference package
EXCLUDED_MODULE_PREFIXES = (
    "openmed.core.anonymizer.engine",
    "openmed.core.anonymizer.locales",
    "openmed.core.anonymizer.person_surrogates",
    "openmed.core.anonymizer.providers.script_names",
    "openmed.core.anonymizer.registry",
    "openmed.core.hf_hub",
    "openmed.core.hf_publish",
    "openmed.core.export_provenance",
    "openmed.core.language_pack_coherence",
    "openmed.core.models",
    "openmed.core.backends",
    "openmed.eval.synth_corpus",
    "openmed.eval.suites",
    "openmed.eval.comparators",
    "openmed.eval.datasets.biomedical_ner",
    "openmed.eval.target_leakage",
    "openmed.training",
    "openmed.service",
    "openmed.integrations",
    "openmed.interop.bridges",
    "openmed.interop.fhir_server",
    "openmed.processing.kafka_connector",
    "openmed.processing.pulsar_connector",
    "openmed.processing.spark_executor",
    "openmed.processing.ray_executor",
    "openmed.mlx",
    "openmed.torch",
    "openmed.onnx",
    "openmed.coreml",
    "openmed.gguf",
    "openmed.ner",
    "openmed.clinical.grounding.embeddings",
    "openmed.clinical.grounding.index",
    "openmed.mcp.server",
)

# Curated roots for Waves 1, 2, 3 (stdlib-pure reference parity layer)
CURATED_ROOT_MODULES = [
    "openmed.__about__",
    # Wave 1 Model Ecosystem
    "openmed.core.manifest_schema",
    "openmed.core.manifest",
    "openmed.core.model_search",
    "openmed.core.manifest_diff",
    "openmed.core.model_card",
    "openmed.core.model_integrity",
    "openmed.core.aead_envelope",
    "openmed.core.portable_bundle",
    "openmed.core.portable_bundle_calibration",
    "openmed.core.device_capabilities",
    "openmed.core.model_cache_policy",
    "openmed.core.registry_slots",
    "openmed.core.registry_compatibility",
    "openmed.core.registry_errors",
    # Wave 2 Privacy, I18N, Calendars, Risk & Compliance
    "openmed.core.pii_entity_merger",
    "openmed.core.clinical_identifiers",
    "openmed.core.clinical_label_map",
    "openmed.core.clinical_policy",
    "openmed.core.clinical_privacy",
    "openmed.core.clinical_protect",
    "openmed.core.anonymizer.providers.clinical_ids",
    "openmed.core.anonymizer.format_preserve",
    "openmed.core.date_shift",
    "openmed.core.rtl_render",
    "openmed.core.calendar_tables",
    "openmed.core.calendars",
    "openmed.core.safety_sweep",
    "openmed.processing.legacy_encoding",
    "openmed.processing.zh_normalize",
    "openmed.processing.zh_pinyin",
    "openmed.processing.morphology",
    "openmed.processing.transliteration",
    "openmed.processing.sentences",
    "openmed.core.calendar_dates",
    "openmed.core.calendar_month_dates",
    "openmed.core.anonymizer.digit_systems",
    "openmed.core.anonymizer.providers.registry_ids",
    "openmed.core.redaction_strength",
    "openmed.core.explain",
    "openmed.core.results",
    "openmed.clinical.summary_card",
    "openmed.clinical.timeline",
    "openmed.clinical.review_packet",
    "openmed.clinical.nli_assertion_pairs",
    "openmed.clinical.nli_experiencer_pairs",
    "openmed.clinical.nli_temporal_pairs",
    "openmed.clinical.summarize",
    "openmed.clinical.extractive_selection",
    "openmed.clinical.summary_claim_segments",
    "openmed.clinical.summary_input",
    "openmed.clinical.summary_temporal_order",
    "openmed.clinical.grounding.calibration",
    "openmed.eval.attacks.reid",
    "openmed.eval.citation_support_metrics",
    "openmed.eval.error_analysis",
    "openmed.eval.section_recall",
    "openmed.eval.summary_coverage",
    "openmed.eval.summary_gate",
    "openmed.eval.summary_unsupported_claims",
    "openmed.eval.surrogate_quality",
    "openmed.interop.fhir_screening",
    "openmed.multimodal.metadata_scrub",
    "openmed.agent.permissions.access_events",
    "openmed.core.arbitration",
    "openmed.core.review_workflow",
    "openmed.risk.kanon",
    "openmed.risk.k_anonymity",
    "openmed.risk.l_diversity",
    "openmed.risk.reid",
    "openmed.risk.qi_profiler",
    "openmed.risk.budget",
    "openmed.risk.differential_privacy",
    "openmed.risk.minimum_necessary",
    "openmed.risk.redaction_diff",
    "openmed.compliance.part11",
    "openmed.compliance.iso27701",
    "openmed.compliance.dsar",
    "openmed.compliance.audit_chain",
    "openmed.compliance.safe_harbor",
    "openmed.compliance.waiver_ledger",
    "openmed.compliance.expert_attestation",
    # Wave 2 Clinical Units, Context, NLP & Specialty Profiles
    "openmed.clinical.units.display_audit",
    "openmed.clinical.lexicons.context_cues",
    "openmed.clinical.lexicons.section_headers",
    "openmed.clinical.context",
    "openmed.clinical.negation_scope",
    "openmed.clinical.experiencer",
    "openmed.clinical.sections.detect",
    "openmed.clinical.sections.doctype",
    "openmed.clinical.sections.provenance",
    "openmed.clinical.sig_parser",
    "openmed.clinical.medication_sig",
    "openmed.clinical.dosing_check",
    "openmed.clinical.med_reconciliation",
    "openmed.clinical.medication_reconciliation",
    "openmed.clinical.assertion_graph",
    "openmed.clinical.drug_interactions",
    "openmed.clinical.cancer_staging",
    "openmed.clinical.genomics",
    "openmed.clinical.radiology_profile",
    "openmed.clinical.pathology_profile",
    "openmed.clinical.discharge_profile",
    "openmed.clinical.sdoh",
    "openmed.clinical.sdoh_experiencer",
    "openmed.clinical.coreference",
    "openmed.clinical.timeline_graph",
    # Wave 3 Multimodal Pure, Interop, Agent Governance, MCP Security & Eval Licenses
    "openmed.multimodal.ooxml_residual",
    "openmed.multimodal.documents_html",
    "openmed.multimodal.documents_markdown",
    "openmed.multimodal.rtf",
    "openmed.multimodal.epub",
    "openmed.multimodal.tabular_csv",
    "openmed.multimodal.chatlog_jsonl",
    "openmed.interop.hl7v2",
    "openmed.interop.hl7v2_narrative",
    "openmed.interop.cda",
    "openmed.interop.x12_837",
    "openmed.interop.fhir_operations",
    "openmed.interop.smart_scope_grammar",
    "openmed.interop.smart_scope_audit",
    "openmed.agent.action_graph",
    "openmed.agent.action_phases",
    "openmed.agent.reviewer_handoff",
    "openmed.agent.approvals.tokens",
    "openmed.agent.security.injection_guard",
    "openmed.agent.workflows.dispatch",
    "openmed.agent.workflows.prior_auth_completeness",
    "openmed.agent.workflows.trial_eligibility_review",
    "openmed.mcp.consent_receipts",
    "openmed.mcp.upstream_endpoint_policy",
    "openmed.mcp.tool_registry",
    "openmed.eval.data_license_gate",
    "openmed.eval.datasets.licenses",
    "openmed.eval.datasets._dua",
]

# Parent packages whose `__init__.py` must NOT be pulled wholesale (to avoid pulling heavy optional subpackages)
SAFE_STUB_INIT_PACKAGES = frozenset(
    {
        "openmed",
        "openmed.core",
        "openmed.core.anonymizer",
        "openmed.core.anonymizer.providers",
        "openmed.clinical",
        "openmed.risk",
        "openmed.compliance",
        "openmed.multimodal",
        "openmed.interop",
        "openmed.interop.fhir",
        "openmed.agent",
        "openmed.mcp",
        "openmed.eval",
        "openmed.processing",
        "openmed.service",
        "openmed.utils",
        "openmed.clinical.exporters",
        "openmed.clinical.relations",
        "openmed.clinical.events",
        "openmed.clinical.grounding",
        "openmed.clinical.normalization",
        "openmed.clinical.data",
    }
)

DATA_DIRS_TO_COPY = [
    "openmed/core/policies",
    "openmed/core/data",
    "openmed/core/schemas",
    "openmed/core/decoding",
    "openmed/core/language_packs",
    "openmed/clinical/data",
    "openmed/clinical/exporters/fhir/definitions",
    "openmed/clinical/relations/data",
    "openmed/compliance/schemas",
    "openmed/interop/fhir",
    "openmed/processing/resources",
    "openmed/risk/data",
]

SINGLE_DATA_ASSETS = [
    "openmed/core/thresholds.json",
    "openmed/core/africa_context_terms.json",
    "openmed/core/config.schema.json",
]

YAML_COMPAT_SOURCE = '''# SPDX-License-Identifier: Apache-2.0
"""Minimal stdlib YAML-subset loader for offline vendored clinical data files."""

from __future__ import annotations

import json
import pathlib
from typing import Any


def _parse_scalar(raw: str) -> Any:
    s = raw.strip()
    if not s or s in {"null", "Null", "NULL", "~"}:
        return None
    if s in {"true", "True", "TRUE"}:
        return True
    if s in {"false", "False", "FALSE"}:
        return False
    if (s.startswith('"') and s.endswith('"')) or (s.startswith("'") and s.endswith("'")):
        return s[1:-1]
    if s.startswith("[") and s.endswith("]"):
        try:
            return json.loads(s)
        except Exception:
            inner = s[1:-1].strip()
            return [_parse_scalar(x) for x in inner.split(",")] if inner else []
    try:
        return int(s)
    except ValueError:
        pass
    try:
        return float(s)
    except ValueError:
        return s


def safe_load(stream: Any) -> Any:
    text = stream.read() if hasattr(stream, "read") else str(stream)
    stripped = text.lstrip()
    if stripped.startswith("{") or stripped.startswith("["):
        return json.loads(text)
    lines = []
    for raw in text.splitlines():
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        indent = len(raw) - len(raw.lstrip(" "))
        lines.append((indent, raw.strip()))
    if not lines:
        return None

    def parse_block(idx: int, min_indent: int) -> tuple[Any, int]:
        is_list = lines[idx][1].startswith("- ") or lines[idx][1] == "-"
        if is_list:
            out: list[Any] = []
            cur_indent = lines[idx][0]
            while idx < len(lines) and lines[idx][0] == cur_indent and (
                lines[idx][1].startswith("- ") or lines[idx][1] == "-"
            ):
                item_str = lines[idx][1][2:].strip() if lines[idx][1].startswith("- ") else ""
                idx += 1
                if not item_str:
                    val, idx = parse_block(idx, cur_indent + 1)
                    out.append(val)
                elif ":" in item_str and not (item_str.startswith('"') or item_str.startswith("'")):
                    k, v = item_str.split(":", 1)
                    d = {k.strip(): _parse_scalar(v) if v.strip() else None}
                    if not v.strip() and idx < len(lines) and lines[idx][0] > cur_indent + 1:
                        d[k.strip()], idx = parse_block(idx, cur_indent + 2)
                    while idx < len(lines) and lines[idx][0] >= cur_indent + 2 and not lines[idx][1].startswith("- "):
                        sub_k, sub_v = lines[idx][1].split(":", 1)
                        sub_indent = lines[idx][0]
                        idx += 1
                        if sub_v.strip():
                            d[sub_k.strip()] = _parse_scalar(sub_v)
                        elif idx < len(lines) and lines[idx][0] > sub_indent:
                            d[sub_k.strip()], idx = parse_block(idx, sub_indent + 1)
                        else:
                            d[sub_k.strip()] = {}
                    out.append(d)
                else:
                    out.append(_parse_scalar(item_str))
            return out, idx
        mapping: dict[str, Any] = {}
        cur_indent = lines[idx][0]
        while idx < len(lines) and lines[idx][0] == cur_indent and not lines[idx][1].startswith("- "):
            k, v = lines[idx][1].split(":", 1)
            key = k.strip().strip('"').strip("'")
            idx += 1
            if v.strip():
                mapping[key] = _parse_scalar(v)
            elif idx < len(lines) and lines[idx][0] > cur_indent:
                mapping[key], idx = parse_block(idx, cur_indent + 1)
            else:
                mapping[key] = {}
        return mapping, idx

    result, _ = parse_block(0, 0)
    return result
'''

_HF_PRESERVE_RE = re.compile(
    r"(https?://\S+|OpenMed/[^\s\"'`,\)]+|huggingface\.co/OpenMed[^\s\"'`,\)]*|"
    r"openmed-calendar-table|"
    r"maziyarpanahi/openmed|github\.com/maziyarpanahi/openmed|"
    r"openmed\.life|medspacy|hijridate|py-nepali|arx-deidentifier|sdcTools|"
    r"Copyright\s*\([cC]\)|SPDX-License-Identifier)",
    re.IGNORECASE,
)


ALLOW_PURE_SERVICE = frozenset(
    {
        "openmed.service",
        "openmed.service.brief",
        "openmed.service.journey_resources",
        "openmed.service.journey_workflows",
    }
)


def is_excluded_mod(mod: str) -> bool:
    if mod in ALLOW_PURE_SERVICE:
        return False
    return any(mod == p or mod.startswith(p + ".") for p in EXCLUDED_MODULE_PREFIXES)


def file_to_mod(mirror: pathlib.Path, p: pathlib.Path) -> str:
    rel = p.relative_to(mirror).with_suffix("")
    parts = list(rel.parts)
    if parts[-1] == "__init__":
        parts = parts[:-1]
    return ".".join(parts)


def compute_closure(mirror: pathlib.Path) -> list[pathlib.Path]:
    pkg_root = mirror / "openmed"
    stdlib = set(getattr(sys, "stdlib_module_names", set())) | {"__future__", "typing_extensions"}
    mod_to_file = {
        file_to_mod(mirror, p): p
        for p in sorted(pkg_root.rglob("*.py"))
        if "__pycache__" not in p.parts
    }

    def top_internal_deps(mod: str) -> list[str]:
        p = mod_to_file[mod]
        tree = ast.parse(p.read_text(encoding="utf-8", errors="replace"))
        pkg_parts = mod.split(".") if p.name == "__init__.py" else mod.split(".")[:-1]

        def resolve_rel(node_mod: str | None, level: int) -> str:
            if level == 0:
                return node_mod or ""
            base = pkg_parts[: len(pkg_parts) - (level - 1)] if level - 1 <= len(pkg_parts) else []
            return ".".join(base + ([node_mod] if node_mod else []))

        deps: set[str] = set()
        for node in tree.body:
            if isinstance(node, ast.Import):
                for a in node.names:
                    if a.name in mod_to_file and not is_excluded_mod(a.name):
                        deps.add(a.name)
            elif isinstance(node, ast.ImportFrom):
                target = resolve_rel(node.module, node.level)
                if target.split(".")[0] == "openmed":
                    for a in node.names:
                        cand = f"{target}.{a.name}"
                        if cand in mod_to_file and not is_excluded_mod(cand):
                            deps.add(cand)
                    if target in mod_to_file and not is_excluded_mod(target) and target not in SAFE_STUB_INIT_PACKAGES:
                        deps.add(target)
        return sorted(deps)

    seen: set[str] = set()
    q = deque(r for r in CURATED_ROOT_MODULES if r in mod_to_file and not is_excluded_mod(r))
    while q:
        m = q.popleft()
        if m in seen:
            continue
        seen.add(m)
        for d in top_internal_deps(m):
            if d not in seen:
                q.append(d)

    return sorted(mod_to_file[m] for m in seen if mod_to_file[m].name != "__init__.py" or m not in SAFE_STUB_INIT_PACKAGES)


def rewrite_string_literal(lit: str) -> str:
    protected: list[str] = []

    def _stash(m: re.Match[str]) -> str:
        idx = len(protected)
        protected.append(m.group(0))
        return f"__MEDSCALE_KEEP_{idx}__"

    safe = _HF_PRESERVE_RE.sub(_stash, lit)
    safe = safe.replace('"OpenMed/"', '"__KEEP_HF_PREFIX__/"').replace("'OpenMed/'", "'__KEEP_HF_PREFIX__/'")
    safe = re.sub(
        r"\bopenmed\.(?=(?:core|clinical|risk|compliance|multimodal|interop|agent|mcp|eval|processing|utils|__about__|ner|zero_shot|structured|guard|traces|models)\b)",
        "medscale_clinical.",
        safe,
    )
    if safe in ('"openmed"', "'openmed'"):
        safe = safe.replace("openmed", "medscale_clinical")
    safe = re.sub(r"\bopenmed\.", "medscale.", safe)
    safe = re.sub(r"\bOpenMed\b", "MedScale", safe)
    safe = re.sub(r"\bOPENMED_", "MEDSCALE_", safe)
    safe = safe.replace("openmed://", "medscale://")
    safe = safe.replace("openmed-", "medscale-")
    safe = safe.replace("openmed_", "medscale_")
    safe = safe.replace('"__KEEP_HF_PREFIX__/"', '"OpenMed/"').replace("'__KEEP_HF_PREFIX__/'", "'OpenMed/'")
    for idx, orig in enumerate(protected):
        safe = safe.replace(f"__MEDSCALE_KEEP_{idx}__", orig)
    return safe


def transform_python_source(src: str, rel_path: str) -> str:
    out_tokens = []
    reader = io.StringIO(src).readline
    for tok in tokenize.generate_tokens(reader):
        tok_type, tok_str, start, end, line = tok
        if tok_type == tokenize.NAME:
            if tok_str == "openmed":
                tok_str = "medscale_clinical"
            elif "OpenMed" in tok_str:
                tok_str = tok_str.replace("OpenMed", "MedScale")
            elif tok_str.startswith("OPENMED_"):
                tok_str = "MEDSCALE_" + tok_str[len("OPENMED_") :]
        elif tok_type == tokenize.STRING:
            tok_str = rewrite_string_literal(tok_str)
        elif tok_type == tokenize.COMMENT:
            tok_str = rewrite_string_literal(tok_str)
        out_tokens.append((tok_type, tok_str, start, end, line))

    transformed = tokenize.untokenize(out_tokens)
    transformed = re.sub(
        r"^import yaml\b.*$",
        "from medscale_clinical import _yaml_compat as yaml",
        transformed,
        flags=re.MULTILINE,
    )
    transformed = re.sub(
        r"^from faker\.providers import BaseProvider\s*$",
        "try:\n    from faker.providers import BaseProvider\nexcept ImportError:\n    class BaseProvider:\n        pass\n",
        transformed,
        flags=re.MULTILINE,
    )
    for rname in (
        "data/context_rules.yaml",
        "data/sdoh_social_cues.yaml",
        "data/sdoh_substance_cues.yaml",
        "data/section_headers.yaml",
        "data/status_vocab.yaml",
    ):
        transformed = transformed.replace(rname, f"{rname}.json")
    transformed = transformed.replace('resources.files("medscale.', 'resources.files("medscale_clinical.')
    transformed = transformed.replace("resources.files('medscale.", "resources.files('medscale_clinical.")
    transformed = transformed.replace('resources.files("medscale")', 'resources.files("medscale_clinical")')
    transformed = transformed.replace('find_spec("medscale.', 'find_spec("medscale_clinical.')
    transformed = transformed.replace(
        'Path(__file__).resolve().parents[2] / "models.jsonl"',
        'next((_p for _p in (Path(__file__).resolve().parents[2] / "models.jsonl", Path(__file__).resolve().parents[5] / "third_party" / "medscale-model-catalog-v3" / "models.jsonl") if _p.is_file()), Path(__file__).resolve().parents[2] / "models.jsonl")',
    )
    transformed = re.sub(
        r"^_MANIFEST_PII_LANGUAGES = manifest_pii_languages\(\)\s*$",
        "try:\n    _MANIFEST_PII_LANGUAGES = manifest_pii_languages()\nexcept Exception:\n    _MANIFEST_PII_LANGUAGES = set(DEFAULT_MODEL_PLACEHOLDER_LANGUAGES)\n",
        transformed,
        flags=re.MULTILINE,
    )
    transformed = re.sub(
        r"^_MANIFEST_ROWS = load_manifest_rows\(\)\s*$",
        "_MANIFEST_ROWS = load_manifest_rows() if MANIFEST_PATH.is_file() else []\n",
        transformed,
        flags=re.MULTILINE,
    )
    if rel_path == "openmed/clinical/status_vocab.py":
        transformed = transformed.replace(
            "def _parse_yaml_subset(text: str) -> dict[str, Any]:\n",
            "def _parse_yaml_subset(text: str) -> dict[str, Any]:\n    import json\n    if text.lstrip().startswith(\"{\"):\n        return json.loads(text)\n",
        )
    if rel_path == "openmed/processing/sentences.py":
        transformed = transformed.replace(
            "        try:\n"
            "            from pysbd import Segmenter  # type: ignore[import-not-found]\n"
            "        except ImportError as exc:  # pragma: no cover - depends on optional dependency\n"
            "            raise ImportError(\n"
            "                \"pySBD is required for sentence detection. \"\n"
            "                \"Install it with `pip install pysbd` or add the `pysbd` dependency.\"\n"
            "            ) from exc\n\n"
            "        segmenter = Segmenter(\n"
            "            language=language,\n"
            "            clean=clean,\n"
            "            char_span=True,\n"
            "        )",
            "        try:\n"
            "            from pysbd import Segmenter  # type: ignore[import-not-found]\n"
            "            segmenter = Segmenter(\n"
            "                language=language,\n"
            "                clean=clean,\n"
            "                char_span=True,\n"
            "            )\n"
            "        except ImportError:\n"
            "            import re as _re\n"
            "            from types import SimpleNamespace as _NS\n"
            "            class _StdlibSegmenter:\n"
            "                def segment(self, src: str):\n"
            "                    return [\n"
            "                        _NS(sent=m.group(0), start=m.start(), end=m.end())\n"
            "                        for m in _re.finditer(r\"[^\\n.!?]+(?:[.!?]+(?=\\s|$)|$)|\\n+\", src)\n"
            "                        if m.group(0).strip()\n"
            "                    ]\n"
            "            segmenter = _StdlibSegmenter()",
        )

    lines = transformed.splitlines(keepends=True)
    while lines and lines[0].lstrip().startswith("# SPDX-"):
        lines.pop(0)
    # Keep `from __future__ import` ahead of any non-comment code; header comments before everything are valid
    header_parts = [
        "# SPDX-License-Identifier: Apache-2.0\n",
        f"# Modified from maziyarpanahi/openmed@{PINNED_OPENMED_COMMIT[:8]} ({rel_path}): "
        "renamed package/symbols to medscale_clinical / MedScale.\n",
    ]
    if rel_path in MEDSPACY_TARGETS:
        header_parts.append(MEDSPACY_ATTRIBUTION_HEADER)
    return "".join(header_parts) + "".join(lines)


def verify_mirror(mirror: pathlib.Path) -> None:
    head = subprocess.check_output(["git", "-C", str(mirror), "rev-parse", "HEAD"], text=True).strip()
    if head != PINNED_OPENMED_COMMIT:
        raise SystemExit(f"Mirror HEAD {head} != pinned {PINNED_OPENMED_COMMIT}")


def sha256_file(p: pathlib.Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def ensure_init_packages(pkg_root: pathlib.Path, subfile: pathlib.Path) -> None:
    cur = subfile.parent
    while cur != pkg_root and pkg_root in cur.parents:
        init_f = cur / "__init__.py"
        if not init_f.exists():
            init_f.write_text('# SPDX-License-Identifier: Apache-2.0\n"""Vendored MedScale reference subpackage."""\n', encoding="utf-8")
        cur = cur.parent


def vendor_python(mirror: pathlib.Path, py_root: pathlib.Path) -> list[dict[str, str]]:
    try:
        import yaml as build_yaml
    except ImportError:
        build_yaml = None

    src_pkg = py_root / "src" / "medscale_clinical"
    # Clean any previously vendored generated files while preserving core existing files
    for old_f in list(src_pkg.rglob("*")):
        if old_f.is_file() and "__pycache__" not in old_f.parts:
            rel_old = old_f.relative_to(src_pkg).as_posix()
            if rel_old not in PRESERVE_EXISTING_REL:
                old_f.unlink()

    (src_pkg / "_yaml_compat.py").write_text(YAML_COMPAT_SOURCE, encoding="utf-8")
    anon_dir = src_pkg / "core" / "anonymizer"
    (anon_dir / "providers").mkdir(parents=True, exist_ok=True)
    (anon_dir / "__init__.py").write_text(
        '# SPDX-License-Identifier: Apache-2.0\n"""Minimal boundary stub: synthetic Faker generation is excluded from zero-dep runtime."""\n\nclass Anonymizer:\n    def __init__(self, *args, **kwargs):\n        raise NotImplementedError("Synthetic Faker anonymizer is excluded from zero-dep medscale_clinical")\n',
        encoding="utf-8",
    )
    (anon_dir / "providers" / "__init__.py").write_text('# SPDX-License-Identifier: Apache-2.0\n', encoding="utf-8")
    (anon_dir / "locales.py").write_text(
        '# SPDX-License-Identifier: Apache-2.0\ndef resolve_locale(*args, **kwargs):\n    raise NotImplementedError("Synthetic Faker locale generation is excluded from zero-dep medscale_clinical")\n',
        encoding="utf-8",
    )
    suites_dir = src_pkg / "eval" / "suites"
    suites_dir.mkdir(parents=True, exist_ok=True)
    (suites_dir / "__init__.py").write_text(
        '# SPDX-License-Identifier: Apache-2.0\n"""Boundary stub: external/DUA benchmark suites are excluded from zero-dep reference runtime."""\nGOLDEN = "golden"\n\ndef validate_suite_name(name: str) -> str:\n    if name != GOLDEN:\n        raise NotImplementedError("External/DUA benchmark suites are excluded from zero-dep medscale_clinical")\n    return name\n\ndef load_suite_fixtures(*args, **kwargs):\n    raise NotImplementedError("External/DUA benchmark fixtures are excluded from zero-dep medscale_clinical")\n',
        encoding="utf-8",
    )

    records: list[dict[str, str]] = []
    closure_files = compute_closure(mirror)
    for src_f in closure_files:
        up_rel = src_f.relative_to(mirror).as_posix()
        inner_rel = up_rel.removeprefix("openmed/")
        if inner_rel in PRESERVE_EXISTING_REL:
            continue
        dst_f = src_pkg / inner_rel
        dst_f.parent.mkdir(parents=True, exist_ok=True)
        ensure_init_packages(src_pkg, dst_f)

        raw = src_f.read_text(encoding="utf-8")
        dst_f.write_text(transform_python_source(raw, up_rel), encoding="utf-8")
        records.append(
            {
                "upstream": up_rel,
                "target": dst_f.relative_to(py_root).as_posix(),
                "sha256": sha256_file(dst_f),
            }
        )

    # Copy permissive data asset directories (excluding __pycache__ or .py handled above)
    for drel in DATA_DIRS_TO_COPY:
        sdir = mirror / drel
        if not sdir.exists():
            continue
        for src_f in sorted(sdir.rglob("*")):
            if not src_f.is_file() or "__pycache__" in src_f.parts or src_f.suffix in {".py", ".pyc"}:
                continue
            up_rel = src_f.relative_to(mirror).as_posix()
            dst_f = src_pkg / up_rel.removeprefix("openmed/")
            dst_f.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src_f, dst_f)
            if dst_f.suffix == ".yaml" and build_yaml is not None:
                parsed = build_yaml.safe_load(src_f.read_text(encoding="utf-8"))
                (dst_f.with_suffix(".yaml.json")).write_text(
                    json.dumps(parsed, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
                    encoding="utf-8",
                )
            records.append(
                {
                    "upstream": up_rel,
                    "target": dst_f.relative_to(py_root).as_posix(),
                    "sha256": sha256_file(dst_f),
                }
            )

    for up_rel in SINGLE_DATA_ASSETS:
        src_f = mirror / up_rel
        if src_f.exists():
            dst_f = src_pkg / up_rel.removeprefix("openmed/")
            dst_f.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src_f, dst_f)
            records.append(
                {
                    "upstream": up_rel,
                    "target": dst_f.relative_to(py_root).as_posix(),
                    "sha256": sha256_file(dst_f),
                }
            )

    tp_dir = py_root / "third_party" / "openmed"
    tp_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(mirror / "LICENSE", tp_dir / "LICENSE")
    shutil.copy2(mirror / "NOTICE", tp_dir / "NOTICE")
    return records


def vendor_ts_sdk(mirror: pathlib.Path, ts_root: pathlib.Path) -> list[dict[str, str]]:
    up_ts = mirror / "clients" / "typescript"
    (ts_root / "src").mkdir(parents=True, exist_ok=True)
    (ts_root / "tests").mkdir(parents=True, exist_ok=True)

    pkg_json = {
        "name": "@medscale/sdk",
        "version": "0.1.0",
        "description": "Zero-dependency TypeScript client for MedScale local REST & workflow services (adapted from OpenMed v4.0.0 Apache-2.0 client).",
        "type": "module",
        "main": "./dist/index.js",
        "types": "./dist/index.d.ts",
        "exports": {
            ".": {
                "types": "./dist/index.d.ts",
                "import": "./dist/index.js",
            }
        },
        "scripts": {
            "test": "node --test tests/sdk.test.ts",
        },
        "license": "Apache-2.0",
        "files": ["dist", "README.md", "LICENSE", "NOTICE", "UPSTREAM.md"],
        "dependencies": {},
    }
    (ts_root / "package.json").write_text(json.dumps(pkg_json, indent=2) + "\n", encoding="utf-8")
    shutil.copy2(up_ts / "tsconfig.json", ts_root / "tsconfig.json")
    shutil.copy2(mirror / "LICENSE", ts_root / "LICENSE")
    shutil.copy2(mirror / "NOTICE", ts_root / "NOTICE")

    def rewrite_ts(txt: str, src_rel: str) -> str:
        header = (
            "// SPDX-License-Identifier: Apache-2.0\n"
            f"// Adapted from maziyarpanahi/openmed@{PINNED_OPENMED_COMMIT[:8]} ({src_rel}); "
            "renamed to @medscale/sdk / MedScale*.\n\n"
        )
        out = txt
        out = out.replace("OpenMedWorkflow", "MedScaleWorkflow")
        out = out.replace("OpenMedClient", "MedScaleClient")
        out = out.replace("OpenMedConfig", "MedScaleConfig")
        out = out.replace("OpenMedError", "MedScaleError")
        out = out.replace("OpenMed", "MedScale")
        out = out.replace("X-OpenMed-Key-Id", "X-MedScale-Key-Id")
        out = out.replace("openmed.agent.", "medscale.agent.")
        out = out.replace("openmed.", "medscale.")
        out = out.replace("openmed-", "medscale-")
        out = out.replace('"openmed"', '"medscale"')
        out = out.replace("'openmed'", "'medscale'")
        return header + out

    out_records = []
    for name in ("index.ts", "workflow-client.ts", "journey-workflows.generated.ts"):
        src = (up_ts / "src" / name).read_text(encoding="utf-8")
        dst = ts_root / "src" / name
        dst.write_text(rewrite_ts(src, f"clients/typescript/src/{name}"), encoding="utf-8")
        out_records.append(
            {
                "upstream": f"clients/typescript/src/{name}",
                "target": dst.as_posix(),
                "sha256": sha256_file(dst),
            }
        )

    (ts_root / "UPSTREAM.md").write_text(
        f"# Upstream provenance: @medscale/sdk\n\n"
        f"- Source repo: `https://github.com/maziyarpanahi/openmed`\n"
        f"- Pinned tag/commit: `v4.0.0` (`{PINNED_OPENMED_COMMIT}`)\n"
        f"- Original directory: `clients/typescript/`\n"
        f"- License: Apache-2.0 (see `LICENSE` and `NOTICE`)\n"
        f"- Modifications: Package and symbol names renamed to `@medscale/sdk`, `MedScaleClient`, `MedScaleWorkflowClient`, and `medscale.*` schema/header prefixes.\n",
        encoding="utf-8",
    )
    return out_records


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mirror", default=r"D:\medscale-upstream-mirrors\openmed-v4.0.0")
    parser.add_argument("--repo", default=r"D:\MedScale")
    args = parser.parse_args(argv)

    mirror = pathlib.Path(args.mirror)
    repo = pathlib.Path(args.repo)
    verify_mirror(mirror)

    py_records = vendor_python(mirror, repo / "integrations" / "clinical-evidence-python")
    ts_records = vendor_ts_sdk(mirror, repo / "integrations" / "medscale-sdk-ts")

    print(f"Vendored {len(py_records)} Python/asset files into medscale_clinical")
    print(f"Vendored {len(ts_records)} TypeScript SDK files into @medscale/sdk")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
