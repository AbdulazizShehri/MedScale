#!/usr/bin/env python3
"""Generate deterministic synthetic Wave 2 conformance vectors from medscale_clinical v0.4.0.

Outputs:
  - crates/medscale-fhir/tests/vectors/ucum_v4.json
  - crates/medscale-clinical/tests/vectors/wave2_privacy_risk_v4.json
  - crates/medscale-clinical/tests/vectors/wave2_clinical_nlp_v4.json
"""

from __future__ import annotations

import json
import pathlib
import sys
from datetime import date
from types import SimpleNamespace
from typing import Any

REPO_ROOT = pathlib.Path(__file__).resolve().parents[3]
SRC_ROOT = REPO_ROOT / "integrations" / "clinical-evidence-python" / "src"
if str(SRC_ROOT) not in sys.path:
    sys.path.insert(0, str(SRC_ROOT))

from medscale_clinical.clinical import (  # noqa: E402
    claim_conflicts,
    context,
    discharge_summary,
    dosing_check,
    experiencer,
    med_reconciliation,
    negation_scope,
    nli_gate,
    pathology_profile,
    radiology_report,
    sdoh,
    sections,
    sig_parser,
)
from medscale_clinical.clinical.units import convert, ucum  # noqa: E402
from medscale_clinical.core import (  # noqa: E402
    audit,
    calendars,
    clinical_protect,
    date_shift,
    pii_i18n,
    policy,
    rtl_render,
    safety_sweep,
)
from medscale_clinical.core.anonymizer.providers import clinical_ids  # noqa: E402
from medscale_clinical.risk import k_anonymity, l_diversity, reid  # noqa: E402


def _write_json(path: pathlib.Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=True) + "\n", encoding="utf-8")


def build_ucum_vectors() -> dict[str, Any]:
    units_to_parse = [
        "kg",
        "mg/dL",
        "mmol/L",
        "Cel",
        "[degF]",
        "mL/min",
        "mm[Hg]",
        "/min",
        "{beats}/min",
        "g/L",
        "mcg",
        "furlong",
    ]
    parsed = []
    for raw in units_to_parse:
        p = ucum.parse_unit(raw)
        parsed.append(
            {
                "input": raw,
                "status": p["status"],
                "canonical_unit": p.get("canonical_unit"),
                "dimension": p.get("dimension", {}),
                "scale": p.get("scale"),
                "offset": p.get("offset", 0.0),
                "is_affine": p.get("is_affine", False),
            }
        )

    conversions_spec = [
        (5.0, "kg", "g"),
        (453.59237, "g", "lb"),
        (98.6, "[degF]", "Cel"),
        (37.0, "Cel", "[degF]"),
        (120.0, "mg/dL", "g/L"),
        (1.5, "L", "mL"),
        (2.4, "L/min", "mL/s"),
        (180.0, "mg/dL", "mmol/L"),
        (5.0, "kg", "Cel"),
        (5.0, "kg", "furlong"),
    ]
    conversions = []
    for val, src, dst in conversions_spec:
        res = convert.normalize_to(val, src, dst)
        conversions.append(
            {
                "value": val,
                "from_unit": src,
                "to_unit": dst,
                "status": res["status"],
                "magnitude": res.get("magnitude"),
                "canonical_magnitude": res.get("canonical_magnitude"),
                "canonical_unit": res.get("canonical_unit"),
                "dimension": res.get("dimension", {}),
            }
        )

    return {"schema_version": 1, "source": "medscale_clinical.clinical.units", "parsed": parsed, "conversions": conversions}


def build_privacy_risk_vectors() -> dict[str, Any]:
    sample_dates = [(2026, 11, 28), (2024, 2, 29), (2000, 1, 1), (1995, 7, 14)]
    calendar_cases = []
    for code in calendars.calendar_codes():
        cal = calendars.get_calendar(code)
        for y, m, d in sample_dates:
            ordinal = date(y, m, d).toordinal()
            ny, nm, nd = cal.from_day_number(ordinal)
            roundtrip_ord = cal.to_day_number(ny, nm, nd)
            sy, sm, sd = cal.shift(ny, nm, nd, 14)
            shift_ord = cal.to_day_number(sy, sm, sd)
            calendar_cases.append(
                {
                    "calendar": code,
                    "gregorian": [y, m, d],
                    "ordinal": ordinal,
                    "native": [ny, nm, nd],
                    "roundtrip_ordinal": roundtrip_ord,
                    "shifted_plus14_native": [sy, sm, sd],
                    "shifted_plus14_ordinal": shift_ord,
                }
            )

    date_shifts = [
        {
            "kind": "hmac",
            "patient_key": "synthetic-patient-001",
            "secret": "medscale-synthetic-secret-k1",
            "max_days": 365,
            "offset": date_shift.stable_offset_for("synthetic-patient-001", max_days=365, secret="medscale-synthetic-secret-k1"),
        },
        {
            "kind": "hmac",
            "patient_key": "synthetic-patient-002",
            "secret": "medscale-synthetic-secret-k1",
            "max_days": 180,
            "offset": date_shift.stable_offset_for("synthetic-patient-002", max_days=180, secret="medscale-synthetic-secret-k1"),
        },
        {
            "kind": "seed",
            "seed": 42,
            "max_days": 90,
            "offset": date_shift.stable_offset_from_seed(42, max_days=90),
        },
    ]

    rtl_ar_in = "\u0627\u0644\u0645\u0631\u064a\u0636 [NAME] \u0645\u0635\u0627\u0628 \u0628\u0627\u0644\u0633\u0643\u0631\u064a"
    rtl_start = rtl_ar_in.index("[NAME]")
    rtl_end = rtl_start + len("[NAME]")
    rtl_ar_res = rtl_render.render_redacted(rtl_ar_in, [{"start": rtl_start, "end": rtl_end}])
    ltr_in = "Patient [NAME] evaluated cleanly."
    ltr_start = ltr_in.index("[NAME]")
    ltr_res = rtl_render.render_redacted(ltr_in, [{"start": ltr_start, "end": ltr_start + len("[NAME]")}])

    rtl_cases = [
        {
            "text": rtl_ar_in,
            "spans": [{"start": rtl_start, "end": rtl_end}],
            "rendered_text": rtl_ar_res.text,
            "base_direction": rtl_ar_res.base_direction,
            "isolated": rtl_ar_res.isolated,
            "wrapped_count": len(rtl_ar_res.source_offsets),
            "stripped": rtl_render.strip_bidi_controls(rtl_ar_res.text),
        },
        {
            "text": ltr_in,
            "spans": [{"start": ltr_start, "end": ltr_start + len("[NAME]")}],
            "rendered_text": ltr_res.text,
            "base_direction": ltr_res.base_direction,
            "isolated": ltr_res.isolated,
            "wrapped_count": len(ltr_res.source_offsets),
            "stripped": rtl_render.strip_bidi_controls(ltr_res.text),
        },
    ]

    protect_text = "Parkinson disease evaluated for Dr. Jane Roe while taking metformin 500 mg."
    p1 = protect_text.index("Parkinson")
    p2 = protect_text.index("Jane Roe")
    p3 = protect_text.index("metformin")
    protect_res = clinical_protect.filter_protected_spans(
        [
            SimpleNamespace(start=p1, end=p1 + len("Parkinson"), text="Parkinson", label="PERSON"),
            SimpleNamespace(start=p2, end=p2 + len("Jane Roe"), text="Jane Roe", label="PERSON"),
            SimpleNamespace(start=p3, end=p3 + len("metformin"), text="metformin", label="PERSON"),
        ],
        protect_text,
    )

    sweep_text = "Call SSN 123-45-6789 or email ada@example.org for MRN: 987654."
    sweep_hits = [
        {"label": e.label, "text": e.text, "start": e.start, "end": e.end}
        for e in safety_sweep.safety_sweep(sweep_text, [])
    ]

    policy_dir = SRC_ROOT / "medscale_clinical" / "core" / "policies"
    all_policy_names = sorted(p.stem for p in policy_dir.glob("*.json"))
    core_policies = {name: policy.load_policy(name).to_dict() for name in policy.list_policies()}

    records = [
        {"age_band": "40-49", "zip3": "123", "sex": "F", "dx": "asthma"},
        {"age_band": "40-49", "zip3": "123", "sex": "F", "dx": "diabetes"},
        {"age_band": "50-59", "zip3": "456", "sex": "M", "dx": "hypertension"},
        {"age_band": "50-59", "zip3": "456", "sex": "M", "dx": "hypertension"},
    ]
    kanon = k_anonymity.kanon_report(records, ["age_band", "zip3", "sex"], ["dx"])
    reid_rep = reid.risk_report(
        [{"text": "Patient [NAME] seen.", "entities": [], "age_band": "40-49", "zip3": "123"}],
        quasi_identifier_fields=["age_band", "zip3"],
    )

    return {
        "schema_version": 1,
        "arabic_digit_norm": {
            "eastern": pii_i18n.normalize_arabic_indic_digits("\u0661\u0660\u0669\u0668\u0667\u0666\u0665\u0664\u0663\u0669"),
            "extended": pii_i18n.normalize_arabic_indic_digits("\u06f2\u06f0\u06f3\u06f4\u06f5\u06f6\u06f7\u06f8\u06f9\u06f7"),
        },
        "calendars": calendar_cases,
        "date_shifts": date_shifts,
        "rtl": rtl_cases,
        "clinical_protect": {
            "text": protect_text,
            "retained": [{"text": s.text, "start": s.start, "end": s.end, "label": s.label} for s in protect_res.spans],
            "suppressed_count": protect_res.suppressed_count,
            "checked_count": protect_res.checked_count,
        },
        "safety_sweep": {"text": sweep_text, "hits": sweep_hits},
        "policy_files": all_policy_names,
        "core_policies": {k: {"posture": v["posture"], "default_action": v["default_action"], "strict_no_leak": v["strict_no_leak"], "safety_sweep_mandatory": v["safety_sweep_mandatory"]} for k, v in core_policies.items()},
        "kanon_summary": {
            "record_count": kanon["record_count"],
            "k": kanon["k"],
            "l": kanon["l"],
            "t_closeness": kanon["t_closeness"],
        },
        "reid_summary": {
            "k_min": reid_rep["k_min"],
            "leakage_rate": reid_rep["leakage_rate"],
            "reid_rate": reid_rep["reid_rate"],
            "singleton_count": len(reid_rep["singleton_records"]),
        },
        "validator_smoke": {
            "luhn_valid": clinical_ids.validate_luhn("79927398713"),
            "luhn_invalid": clinical_ids.validate_luhn("79927398714"),
            "npi_valid": clinical_ids.validate_npi("1234567893"),
            "ssn_valid": clinical_ids.validate_ssn("123-45-6789"),
            "ssn_invalid_666": clinical_ids.validate_ssn("666-12-3456"),
        },
    }


def build_clinical_nlp_vectors() -> dict[str, Any]:
    note = (
        "CHIEF COMPLAINT:\n"
        "Cough and shortness of breath.\n\n"
        "PAST MEDICAL HISTORY:\n"
        "History of asthma.\n\n"
        "MEDICATIONS:\n"
        "Albuterol 2 puffs inhaled q4h PRN wheezing.\n"
    )
    sec_spans = [
        {"label": s["label"], "start": s["start"], "end": s["end"], "loinc_code": s.get("loinc_code"), "header": s.get("header")}
        for s in sections.detect_sections(note)
    ]

    neg_text = "No chest pain or hemoptysis, but has fever."
    neg_spans = negation_scope.detect_negation_scopes(
        neg_text,
        [
            {"start": neg_text.index("chest pain"), "end": neg_text.index("chest pain") + len("chest pain"), "text": "chest pain"},
            {"start": neg_text.index("fever"), "end": neg_text.index("fever") + len("fever"), "text": "fever"},
        ],
    )
    neg_out = [
        {"cue": n.cue, "direction": n.direction, "scope_start": n.scope_start, "scope_end": n.scope_end, "governed": [list(g) for g in n.governed]}
        for n in neg_spans
    ]

    exp_text = "Patient's mother has type 2 diabetes; patient has hypertension."
    exp_out = experiencer.refine_experiencer(
        [
            {"start": exp_text.index("type 2 diabetes"), "end": exp_text.index("type 2 diabetes") + len("type 2 diabetes"), "text": "type 2 diabetes"},
            {"start": exp_text.index("hypertension"), "end": exp_text.index("hypertension") + len("hypertension"), "text": "hypertension"},
        ],
        text=exp_text,
    )

    sigs = [
        sig_parser.parse_sig("1 tab PO BID x7 days"),
        sig_parser.parse_sig("Take 2 puffs inhaled q4h PRN wheezing"),
        sig_parser.parse_sig("500 mg IV q12h x5 days"),
    ]

    medrec = med_reconciliation.reconcile_medications(
        [
            {"medication": "metformin", "dose": "500 mg", "status": "started", "timestamp": "2026-10-01"},
            {"medication": "metformin", "dose": "1000 mg", "status": "changed", "timestamp": "2026-11-01"},
            {"medication": "lisinopril", "dose": "10 mg", "status": "continued", "timestamp": "2026-11-01"},
            {"medication": "aspirin", "dose": "81 mg", "status": "stopped", "timestamp": "2026-11-01"},
        ]
    )
    medrec_out = [
        {
            "ingredient": r.ingredient,
            "current_status": r.current_status,
            "current_dose": r.current_dose,
            "history_len": len(r.history),
        }
        for r in medrec
    ]

    dosing = dosing_check.check_dose_ranges(
        [{"drug": "metformin", "route": "oral", "dose": "2500 mg"}, {"drug": "metformin", "route": "oral", "dose": "1000 mg"}],
        {"metformin": {"oral": {"low": 500, "high": 2000, "unit": "mg"}}},
    )
    dosing_out = [
        {
            "drug": d.suggestion["drug"],
            "status": d.suggestion["status"],
            "requires_clinician_review": d.requires_clinician_review,
            "autonomous_decision": d.autonomous_decision,
        }
        for d in dosing
    ]

    sdoh_text = "Patient lives alone and is a former smoker; currently unemployed."
    sdoh_out = [
        {"category": f.category, "value": f.value, "status": f.status, "temporality": f.temporality}
        for f in sdoh.extract_sdoh(sdoh_text, [])
    ]

    rad = radiology_report.parse_radiology_report(
        "EXAM: Chest X-Ray\nINDICATION: Cough.\nFINDINGS: Lungs are clear without consolidation.\nIMPRESSION: Normal chest radiograph."
    )
    path = pathology_profile.extract_pathology_profile(
        "SPECIMEN: Colon polyp\nDIAGNOSIS: Tubular adenoma, negative for dysplasia. Margins negative."
    )
    disch = discharge_summary.extract_discharge_summary(
        "DISCHARGE DIAGNOSIS:\nCommunity-acquired pneumonia.\nDISCHARGE MEDICATIONS:\nAmoxicillin 500 mg PO TID x7 days."
    )

    nli_res = nli_gate.evaluate_nli(
        {"entailment": 0.91, "contradiction": 0.04, "neutral": 0.05, "calibrated": True, "calibration_id": "medscale_clinical-clinical-nli-v1"},
        {"claim_id": "c1", "source_id": "s1", "relation": "nli"},
    )
    conflicts_res = claim_conflicts.detect_claim_conflicts(
        [
            {"claim_id": "clm-1", "claim_key": "smoking_status", "value": "never", "source_id": "s1", "status": "present"},
            {"claim_id": "clm-2", "claim_key": "smoking_status", "value": "former", "source_id": "s2", "status": "present"},
        ]
    )

    return {
        "schema_version": 1,
        "sections": {"note": note, "detected": sec_spans},
        "negation_scope": {"text": neg_text, "scopes": neg_out},
        "experiencer": {
            "text": exp_text,
            "assignments": [{"text": r.span["text"], "experiencer": r.assignment.experiencer, "cue": r.assignment.cue} for r in exp_out],
        },
        "sigs": sigs,
        "med_reconciliation": medrec_out,
        "dosing_check": dosing_out,
        "sdoh": {"text": sdoh_text, "findings": sdoh_out},
        "radiology": {
            "findings_text": rad["findings_text"],
            "impression_text": rad["impression_text"],
            "recommendation_text": rad["recommendation_text"],
        },
        "pathology": {
            "specimen_count": len(path["specimen"]),
            "diagnosis_text": path["diagnosis"][0]["value"] if path["diagnosis"] else None,
        },
        "discharge": {
            "diagnosis_count": len(disch.discharge_diagnoses),
            "medications": [
                {"medication": m.medication, "dose": m.sig.get("dose"), "unit": m.sig.get("unit"), "route": m.sig.get("route")}
                for m in disch.discharge_medications
            ],
        },
        "nli": {"outcome": nli_res.outcome, "selected_probability": nli_res.selected_probability, "margin": nli_res.margin},
        "claim_conflicts_count": len(getattr(conflicts_res, "conflicts", ())),
    }


def main() -> int:
    _write_json(REPO_ROOT / "crates" / "medscale-fhir" / "tests" / "vectors" / "ucum_v4.json", build_ucum_vectors())
    _write_json(REPO_ROOT / "crates" / "medscale-clinical" / "tests" / "vectors" / "wave2_privacy_risk_v4.json", build_privacy_risk_vectors())
    _write_json(REPO_ROOT / "crates" / "medscale-clinical" / "tests" / "vectors" / "wave2_clinical_nlp_v4.json", build_clinical_nlp_vectors())
    print("Generated Wave 2 conformance vectors successfully.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
