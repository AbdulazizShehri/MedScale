# Feature Specification: Audit + Exports + Settings + Integrations

**Feature Branch**: `spec/064-audit-exports-settings-integrations`
**Created**: 2026-09-15
**Status**: CLOSED_CANONICAL
**Promotion**: POST_SPEC_063_PRODUCT_SURFACE_SEQUENCE

## Goal
Complete native Desktop utility surfaces using existing doctor, disclosure, FHIR support, and Network Broker truth without converting configuration/status UI into authority or readiness evidence.

## Requirements
- **FR-001**: Audit Trail MUST present admitted synthetic disclosure records with explicit Core Host canonical-audit boundary.
- **FR-002**: Exports MUST surface the existing FHIR R4 support matrix and preserve loss/conformance limitations; no full-conformance or clinical-interpretation claim.
- **FR-003**: Settings MUST expose doctor truth for locality, PHI authorization, private-data readiness, release readiness, accessibility, and external residuals.
- **FR-004**: Integrations MUST expose Network Broker default-deny posture and live-partner authorization truth.
- **FR-005**: NPHIES, online packs, and MESC status MUST remain honest and external/optional where canonically defined.
- **FR-006**: Desktop MUST NOT add direct storage/network clients or partner credentials.
- **FR-007**: Configuration UI MUST NOT be treated as qualification evidence.
- **FR-008**: Existing patient/Insights/workflow surfaces and package/release honesty MUST remain intact.
- **FR-009**: Synthetic demo data MUST remain deterministic and visibly labeled.
- **FR-010**: Real PHI, production partner integrations, signing/notarization, qualified-hardware performance, WCAG conformance, and release readiness remain out of scope.

## Success Criteria
- Audit Trail, Exports, Settings, and Integrations are real native surfaces.
- Disclosure audit rows remain synthetic/non-release claims.
- FHIR rows visibly separate qualified/partial/unsupported axes.
- Settings display `REAL_PHI`/`PRIVATE_DATA_READY`/`RELEASE_READY`/WCAG truth.
- Network Broker remains default deny and live partner authorization remains false.
- Spec 012/MESC stays optional/deferred and untouched.
