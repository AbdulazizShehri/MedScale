# Spec 021 limitations

- `RELEASE_READY = FALSE`
- `PRIVATE_DATA_READY = FALSE`
- `MULTI_CLIENT_RELEASE_READY = FALSE`
- Synthetic-only; REAL_PHI unauthorized
- No full FHIR R4 conformance claim (Spec 020 matrix remains authoritative for interchange honesty)
- Final visual Desktop / v0 UI not delivered by this unit
- Accessibility certification not claimed
- Encrypted-vault backup/restore not exercised by the READY_BASE journey (synthetic BackupVault/RestoreVault only)
- Host SessionRegistry OpenSession is optional; journey uses lease-only CoreFacade path
- Spec 012 MESC remains blocked externally; untouched
- Completing the journey does not authorize live SMART/NPHIES, product network egress, or private-data release
