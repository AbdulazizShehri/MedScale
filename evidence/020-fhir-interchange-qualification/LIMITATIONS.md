# Spec 020 limitations

- `RELEASE_READY = FALSE`
- `PRIVATE_DATA_READY = FALSE`
- Synthetic-only; REAL_PHI unauthorized
- Full FHIR R4 conformance is **not** claimed from the closed extractor subset
- Profile validation not qualified
- Reference resolution not qualified
- Clinical interpretation never produced by the interchange layer
- Terminology limited to Observation UCUM subset pin — not licensed terminology packs
- External validator output is evidence only, never MedScale authority
- Loss-aware export is a READY_BASE stub (top-level field inventory), not Composition/document export (V1.5)
- SMART / live partner FHIR / NPHIES remain separately gated
- Minimum lovable workflow journey deferred to Spec 021 (Q07)
- MESC untouched
