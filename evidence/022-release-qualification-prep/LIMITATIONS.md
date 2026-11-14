# Spec 022 limitations

- `RELEASE_READY = FALSE`
- `PRIVATE_DATA_READY = FALSE`
- `MULTI_CLIENT_RELEASE_READY = FALSE`
- macOS CI / qualification not performed
- Mobile apps not release-qualified (scaffold only)
- Branch protection / required checks not configured by this unit (EXTERNAL_GATES)
- No product signing, notarization, or distribution package SBOM claim
- Public SPDX license choice remains EXTERNAL_GATES (`PUBLIC_SOURCE_LICENSE_CHOICE`)
- Spec 012 MESC remains blocked externally; untouched
- Synthetic-only; REAL_PHI unauthorized
- Prep evidence does not authorize private-data or multi-client release
