# Security and Privacy — Spec 079 Privacy Gate

## Threats and required proofs

| # | Threat | Required control | Proof |
|---|---|---|---|
| T1 | Unclassified artifact leaves | Default `LocalPhi`; egress denies | Core test over every boundary |
| T2 | Source overwritten by transform | Output is a new derived artifact; source digest re-verified after transform | Core test compares source bytes/digest before and after |
| T3 | Plaintext leaks into receipts, decisions, audit rows, errors | Receipts hold counts and digests only; fixed error strings | Test scans every persisted 079 row and error for each corpus sensitive value |
| T4 | Pseudonym key in vault metadata or backup | Key stored only in `KeyStore` under `key_account` | Test scans raw SQLite and backup JSON for key bytes (hex and base64) |
| T5 | Re-identification without audit | Audit row written before the value is returned; separate capability | Core test; capability denied without grant |
| T6 | Revoked receipt/profile/map still usable | Revocation checked on every egress/transform/re-identification | Core tests |
| T7 | Output tampered after receipt | Egress recomputes the output digest | Core test mutates stored derived bytes via storage test hook |
| T8 | Recognizer unavailable reported as clean | `Unavailable` residual; egress denies | Core test with missing Pack |
| T9 | Cross-Project access | Every read/write scope-checked through Project | Core test with a foreign Project |
| T10 | Stale revision / replay | `ProjectRevision` checks; transforms are insert-once | Storage + Core tests |
| T11 | Malformed/hostile input | Bounds on input size and span count; invalid UTF-8 refused; malformed FHIR falls back to deterministic scan with status recorded | Core tests |
| T12 | Tampered backup rows | Restore re-validates 079 rows and cross-row invariants | Storage test |
| T13 | Network use | No network code or crate in 079 modules | Deterministic grep in `EXACT_RANGE_REVIEW.md` |
| T14 | False completeness claim | Fixed limitation strings; no "PHI-free" vocabulary | Contract test on limitation vocabulary |

## Explicit non-capabilities

- No compliance, anonymization or "PHI removed" claim anywhere.
- No egress execution. 079 decides; later specs send.
- No real PHI; synthetic fixtures only.
