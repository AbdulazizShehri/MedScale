# Evidence LIMITATIONS — Spec 043

- Existence / configuration probes only; **no** measured confidentiality protection for swap, pagefile, hibernate, volume snapshots, or core dumps.
- `PROTECTION_MEASURED` is intentionally unused for these surfaces in Spec 043.
- `PRIVATE_DATA_READY` remains **FALSE**; gate `OS_KEYRING_SWAP_SNAPSHOT_PRIVATE_DATA` stays OPEN.
- Snapshot inventories may be incomplete without elevation / owner policy (VSS, APFS, btrfs).
- Does not promise universal secure deletion of OS residual memory surfaces.
- REAL_PHI unauthorized; RELEASE_READY unchanged; sandbox PLATFORM_QUALIFIED unchanged.
