# Spec 013 conformance notes (REFERENCE_ONLY)

| Tool / kit | Role | Status |
|---|---|---|
| Inferno FHIR Validator / test kit | External evidence oracle | **REFERENCE_ONLY** — not executed in Spec 013 CI |
| SMART App Launch test kit | External evidence oracle | **REFERENCE_ONLY** — live partner not authorized |

MedScale does **not** treat validator or SMART-kit output as clinical authority. When attached via Network Broker profile/conformance purposes, results persist only as `EvaluationRecord` with `evidence_only=true`.

Live partner TLS and production SMART authorize remain behind `EXTERNAL_GATES` (`PARTNER_EHR_NPHIES_ENDPOINT` / SMART live partner). Spec 013 proves fail-closed `ExternalGateRequired` without opening sockets.
