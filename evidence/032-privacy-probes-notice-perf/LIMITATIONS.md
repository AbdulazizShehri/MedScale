# Evidence LIMITATIONS — Spec 032

- Privacy probes report existence / leftover presence only; they do **not** qualify swap, hibernate, volume snapshots, or pagefile confidentiality.
- `PRIVATE_DATA_READY` remains **FALSE**; `OS_KEYRING_SWAP_SNAPSHOT_PRIVATE_DATA` stays OPEN.
- NOTICE inventory is attribution prep; `rights_license_decision=false`; `PUBLIC_SOURCE_LICENSE_CHOICE` stays PENDING.
- Perf binding strengthens reproducibility metadata only; budgets are **not** claimed met; `RELEASE_READY=false`.
- Windows pagefile/hiberfil often PermissionDenied — treated as existence-inferred `Detected` when that is the OS signal; content is never read.
- REAL_PHI unauthorized; MULTI_CLIENT_RELEASE_READY unchanged; WORKER_OS_SANDBOX_PLATFORM_QUALIFIED unchanged.
