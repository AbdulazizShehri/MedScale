# Spec 079 — Privacy Gate

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promoted:** 2026-09-23
**Base SHA:** `cf8731e497273633efe2968ade4cbdd6523e5595`
**Target branch:** `spec/079-privacy-gate`
**Dependency:** canonical MedScale through Spec 078 (`CLOSED_CANONICAL`); hard dependency 074 + 077
**Promotion authority:** `docs/planning/SPEC_079_PROMOTION.md`

## 1. Problem

MedScale has Projects (074), data sources (075), collaboration (076), agents
(077) and fleets (078), but no shared answer to "may this artifact leave
the local boundary, and in what form?". Every later unit (Browse, AudioFlow,
Analytics, Hub, Compute, R, extensions) needs that answer from one Core
authority, not from ad-hoc checks in each feature.

## 2. Goal

One Core-owned Privacy Gate:

- classify artifacts into four data classes, fail closed when unknown;
- detect sensitive spans with local recognizers only;
- transform a source into a new de-identified artifact under an explicit
  profile, with a `DeidReceipt` that records how, and what remains unknown;
- keep reversible pseudonym maps under separate key custody, with audited
  re-identification and revocation;
- decide egress for every boundary through one function that denies unless
  the evidence allows it.

## 3. Scenarios

1. A researcher opens a Project, sees a source note reported as `LocalPhi`
   (basis `DefaultUnclassified`), and declares a public reference document
   `Public`.
2. They create a profile (names pseudonymize, dates generalize, identifiers
   redact, everything else redact) and run a transform on a synthetic
   clinical note with Spanish value formats. A new derived artifact appears; the source is
   unchanged; the receipt lists recognizers, versions, counts and a residual
   scan result.
3. Egress to `Browse` for the unclassified source denies (`DeniedUnclassified`); egress for
   the derived artifact allows (`AllowedDeidentified`).
4. They revoke the receipt; egress for the derived artifact now denies
   (`DeniedReceiptRevoked`).
5. An auditor with the re-identification capability resolves one pseudonym
   with a stated reason; an audit row is written first. After the map is
   revoked, the same request denies and is audited.
6. The model recognizer is unavailable (Pack missing); the residual scan
   says `Unavailable`; egress denies (`DeniedResidualUnavailable`).
7. Everything survives close/reopen and backup/restore with the network
   disabled.

## 4. Non-goals

See `SPEC_079_PROMOTION.md` "Explicitly not authorized".

## 5. Requirements

- FR-01 Classification per artifact per Project, revisioned, default
  `LocalPhi`/`DefaultUnclassified`.
- FR-02 Profiles: complete rule map, bounded name, revocable, revisioned.
- FR-03 Recognizers: deterministic, FHIR-aware, local model; each reports
  identity, version and status.
- FR-04 Transform: new derived artifact; source immutable; receipt binds
  source/output digests, profile revision, recognizers, op counts,
  pseudonym map, residual scan, limitations.
- FR-05 Residual scan: all enabled recognizers re-run on the output; no
  "absent" claim.
- FR-06 Pseudonym maps: keyed pseudonyms; sealed reverse entries; key in
  `KeyStore` only; audited re-identification; revocation destroys the key.
- FR-07 Egress decisions for all boundaries; persisted; fail closed.
- FR-08 CLI (human + JSON) and native Desktop route over Core.
- FR-09 Storage v8 additive migration, backup/restore, consistency checks.
- FR-10 Synthetic multi-locale corpus (English-language notes carrying
  English, Spanish, French and German value formats, plus a FHIR Patient)
  with class-wise detection counts. See the language decision in
  `SPEC_079_PROMOTION.md`.

## 6. Success criteria

The frozen acceptance requirements in `SPEC_079_PROMOTION.md`.
