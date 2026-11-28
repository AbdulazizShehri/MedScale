"""Conformance & regression tests for Wave 2 OpenMed v4.0.0 privacy, UCUM, and clinical NLP modules."""

from __future__ import annotations

import json
import pathlib
from datetime import date

import pytest

from medscale_clinical.clinical import (
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
from medscale_clinical.clinical.units import convert, ucum
from medscale_clinical.core import (
    calendars,
    clinical_protect,
    date_shift,
    pii_i18n,
    policy,
    rtl_render,
    safety_sweep,
)
from medscale_clinical.risk import k_anonymity, reid

REPO_ROOT = pathlib.Path(__file__).resolve().parents[3]


def test_ucum_vectors_match_generated_contract() -> None:
    vec = json.loads((REPO_ROOT / "crates" / "medscale-fhir" / "tests" / "vectors" / "ucum_v4.json").read_text(encoding="utf-8"))
    for case in vec["parsed"]:
        parsed = ucum.parse_unit(case["input"])
        assert parsed["status"] == case["status"]
        assert parsed.get("canonical_unit") == case["canonical_unit"]
    for case in vec["conversions"]:
        res = convert.normalize_to(case["value"], case["from_unit"], case["to_unit"])
        assert res["status"] == case["status"]
        if res["status"] == "ok":
            assert res["magnitude"] == pytest.approx(case["magnitude"], rel=1e-9, abs=1e-9)


def test_privacy_risk_vectors_match_generated_contract() -> None:
    vec = json.loads((REPO_ROOT / "crates" / "medscale-clinical" / "tests" / "vectors" / "wave2_privacy_risk_v4.json").read_text(encoding="utf-8"))
    for case in vec["calendars"]:
        cal = calendars.get_calendar(case["calendar"])
        y, m, d = cal.from_day_number(case["ordinal"])
        assert [y, m, d] == case["native"]
        assert cal.to_day_number(y, m, d) == case["roundtrip_ordinal"]
    for ds in vec["date_shifts"]:
        if ds["kind"] == "hmac":
            assert date_shift.stable_offset_for(ds["patient_key"], max_days=ds["max_days"], secret=ds["secret"]) == ds["offset"]
        else:
            assert date_shift.stable_offset_from_seed(ds["seed"], max_days=ds["max_days"]) == ds["offset"]
    for rc in vec["rtl"]:
        res = rtl_render.render_redacted(rc["text"], rc["spans"])
        assert res.text == rc["rendered_text"]
        assert res.base_direction == rc["base_direction"]
    assert pii_i18n.normalize_arabic_indic_digits("\u0661\u0662\u0663") == "123"
    assert len(vec["policy_files"]) >= 20
    assert safety_sweep.safety_sweep(vec["safety_sweep"]["text"], [])


def test_clinical_nlp_vectors_match_generated_contract() -> None:
    vec = json.loads((REPO_ROOT / "crates" / "medscale-clinical" / "tests" / "vectors" / "wave2_clinical_nlp_v4.json").read_text(encoding="utf-8"))
    assert len(sections.detect_sections(vec["sections"]["note"])) == len(vec["sections"]["detected"])
    for expected in vec["sigs"]:
        parsed = sig_parser.parse_sig(expected["raw"])
        assert parsed == expected
    assert len(negation_scope.detect_negation_scopes(vec["negation_scope"]["text"], [{"start": 3, "end": 13, "text": "chest pain"}])) == 1
    exp_t = vec["experiencer"]["text"]
    exp_s = exp_t.index("type 2 diabetes")
    assert experiencer.refine_experiencer([{"start": exp_s, "end": exp_s + len("type 2 diabetes"), "text": "type 2 diabetes"}], text=exp_t)[0].assignment.experiencer == "family"
    assert len(sdoh.extract_sdoh(vec["sdoh"]["text"], [])) == len(vec["sdoh"]["findings"])
    rad = radiology_report.parse_radiology_report("EXAM: Chest X-Ray\nINDICATION: Cough.\nFINDINGS: Lungs are clear without consolidation.\nIMPRESSION: Normal chest radiograph.")
    assert rad["impression_text"] == vec["radiology"]["impression_text"]
    path = pathology_profile.extract_pathology_profile("SPECIMEN: Colon polyp\nDIAGNOSIS: Tubular adenoma, negative for dysplasia. Margins negative.")
    assert path["diagnosis"][0]["value"] == vec["pathology"]["diagnosis_text"]
    disch = discharge_summary.extract_discharge_summary("DISCHARGE DIAGNOSIS:\nCommunity-acquired pneumonia.\nDISCHARGE MEDICATIONS:\nAmoxicillin 500 mg PO TID x7 days.")
    assert len(disch.discharge_diagnoses) == vec["discharge"]["diagnosis_count"]
    assert dosing_check.check_dose_ranges([{"drug": "metformin", "route": "oral", "dose": "2500 mg"}], {"metformin": {"oral": {"low": 500, "high": 2000, "unit": "mg"}}})[0].suggestion["status"] == "above_range"
    assert nli_gate.evaluate_nli({"entailment": 0.91, "contradiction": 0.04, "neutral": 0.05, "calibrated": True, "calibration_id": "medscale_clinical-clinical-nli-v1"}, {"claim_id": "c1", "source_id": "s1", "relation": "nli"}).outcome == vec["nli"]["outcome"]
    kan = k_anonymity.kanon_report([{"a": "1", "b": "x"}, {"a": "1", "b": "y"}], ["a"], ["b"])
    assert kan["k"] == 2
    assert context.assert_context_axes({"text": "asthma"}).negation == "affirmed"
    assert reid.risk_report([{"text": "clean", "entities": [], "age_band": "40-49"}], quasi_identifier_fields=["age_band"])["k_min"] == 1
    assert clinical_protect.load_bundled_terms()
    assert policy.list_policies()
    assert med_reconciliation.reconcile_medications([{"medication": "aspirin", "dose": "81 mg", "status": "started"}])[0].current_status == "started"
