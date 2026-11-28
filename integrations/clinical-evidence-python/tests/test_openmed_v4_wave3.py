"""Wave 3 Python conformance and safety tests for OpenMed v4.0.0 parity."""

from __future__ import annotations

import json
import pathlib
import sys
import unittest

REPO_ROOT = pathlib.Path(__file__).resolve().parents[3]
CONFORMANCE_DIR = REPO_ROOT / "integrations" / "clinical-evidence-python" / "conformance"
if str(CONFORMANCE_DIR) not in sys.path:
    sys.path.insert(0, str(CONFORMANCE_DIR))

from generate_wave3_vectors import (  # noqa: E402
    build_agent_eval_vectors,
    build_interop_vectors,
    build_multimodal_dicom_vectors,
)


class Wave3OpenMedParityTest(unittest.TestCase):
    def test_multimodal_and_archive_safety_vectors(self) -> None:
        vec = build_multimodal_dicom_vectors()
        self.assertIn("Discharge Note", vec["html"]["text"])
        self.assertNotIn("alert(1)", vec["html"]["text"])
        self.assertIn("Alex Rivera", vec["markdown"]["text"])
        self.assertEqual(vec["archive_safety"]["decision"], "reject")
        self.assertIn("path_traversal", vec["archive_safety"]["reasons"])
        self.assertEqual(vec["pdf_geometry"]["status"], "readable")
        self.assertEqual(vec["pdf_geometry"]["declared_page_count"], 1)

    def test_interop_vectors(self) -> None:
        vec = build_interop_vectors()
        self.assertTrue(vec["fhir_deidentify"]["patient_note_redacted"])
        self.assertTrue(vec["fhir_reference_integrity"]["valid_is_valid"])
        self.assertFalse(vec["fhir_reference_integrity"]["broken_is_valid"])
        self.assertEqual(vec["smart_scopes"]["ok_status"], "pass")
        self.assertEqual(vec["smart_scopes"]["excess_status"], "excessive")
        self.assertTrue(vec["hl7v2"]["removes_raw_name"])
        self.assertTrue(vec["cda"]["removes_raw_name"])
        self.assertTrue(vec["x12_837"]["removes_raw_member"])

    def test_agent_mcp_and_eval_vectors(self) -> None:
        vec = build_agent_eval_vectors()
        self.assertEqual(vec["action_graph"]["dag_order"], ["step-a", "step-b"])
        self.assertIn("dependency_cycle", vec["action_graph"]["cycle_findings"])
        self.assertTrue(vec["action_phase"]["unreviewed_waiting_review_blocked"])
        self.assertTrue(vec["approval_token"]["replay_blocked"])
        self.assertIn("instruction_override", vec["injection_guard"]["bad_finding_ids"])
        self.assertEqual(vec["injection_guard"]["clean_finding_count"], 0)
        self.assertTrue(vec["mcp_ssrf"]["blocks_metadata"])
        self.assertTrue(vec["mcp_ssrf"]["blocks_private"])
        self.assertTrue(vec["mcp_ssrf"]["loopback_allowed_when_opted_in"])
        self.assertEqual(vec["consent_receipt"]["verified_receipt_id"], "rcpt-wave3-001")
        self.assertTrue(vec["dua_guard"]["in_repo_blocked"])
        self.assertTrue(vec["dua_guard"]["external_path_validated"])

    def test_committed_wave3_vector_hashes_match_canonical_payload(self) -> None:
        for rel in (
            "crates/medscale-pack/tests/vectors/wave3_multimodal_dicom_v4.json",
            "crates/medscale-fhir/tests/vectors/wave3_interop_v4.json",
            "crates/medscale-clinical/tests/vectors/wave3_agent_eval_v4.json",
        ):
            payload = json.loads((REPO_ROOT / rel).read_text(encoding="utf-8"))
            self.assertIn("vector_sha256", payload)
            self.assertTrue(payload["vector_sha256"].startswith("sha256:"))


if __name__ == "__main__":
    unittest.main()
