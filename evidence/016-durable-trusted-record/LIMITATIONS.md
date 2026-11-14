# Spec 016 LIMITATIONS

- `PRIVATE_DATA_READY = FALSE` after this unit
- SyntheticVault open metadata is plaintext while unlocked
- EncryptedVault seal-at-close covers the working DB when closed; open-vault WAL/temp not Q03-qualified here
- Writer lock is OS-process exclusive ownership for store integrity (Q02), not authenticated multi-client sessions (Q04)
- Performance budgets from Trusted V1 plan are **targets**, not measured PASS
- MESC Spec 012 remains externally blocked
