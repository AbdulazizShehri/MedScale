# Spec 028 LIMITATIONS

- `PRIVATE_DATA_READY = FALSE` — OS keyring READY_BASE does **not** complete Q03
- Swap, hibernation, and volume snapshot artifact proofs remain unqualified
- Linux CI without keyutils may report `os_keyring_available=false` (fail-soft); FakeOsKeyStore still proves API boundary
- keyring **4.x** not admitted (rustc ≥ 1.88 required)
- Probe uses synthetic account `medscale.vault.__doctor_probe__` only
- REAL_PHI unauthorized; synthetic-only
- Clearing EXTERNAL_GATES `OS_KEYRING_SWAP_SNAPSHOT_PRIVATE_DATA` requires measured swap/snapshot evidence in a later unit
