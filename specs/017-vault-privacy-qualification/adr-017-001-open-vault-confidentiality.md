# ADR-017-001: Open-vault metadata confidentiality approach

**Status**: Accepted for Spec 017  
**Date**: 2026-09-09

## Decision

1. **Retain** Spec 005 D4 AES-GCM sealed metadata (`meta.sealed`) + sealed blobs as the EncryptedVault at-rest backend.
2. **Do not enable SQLCipher** in this unit. Admission `005-sqlcipher-rusqlite.md` remains preferred re-entry only after SyntheticVault isolation / dedicated sqlcipher crate so `bundled` and `bundled-sqlcipher` do not collide.
3. **Strengthen lifecycle**: on close, seal then securely remove `meta.work.sqlite3` and SQLite sidecars; on open, detect crash leftovers.
4. **`PRIVATE_DATA_READY` remains FALSE** until OS key custody, crash/snapshot artifact proof, and measured platform evidence land (may span Spec 017 closeout + follow-on).

## Consequences

- Honest privacy claims; reduced leftover plaintext after close.
- Open-while-unlocked work file still exists during session — residual risk documented.
- SQLCipher path preserved without fake enablement.
