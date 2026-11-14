# Spec 043 — Swap / Snapshot / Core-Dump Honesty (Q03 residual)

## Classification

`EXISTING_Q03_RESIDUAL_ELIGIBLE_FOR_PROMOTION` — see `docs/planning/SPEC_043_PROMOTION.md`.

## Delivered

- Extended `privacy_probes` with classified surfaces: pagefile, hibernate, swap, snapshot, core_dump_config.
- Each surface reports `existence_honesty` + `protection_honesty`.
- Protection for these residuals is always `owner_or_os_policy_required` in this unit (never `protection_measured`).
- Doctor `swap_snapshot_honesty_present=true`; `private_data_ready=false`.
- Residual classes open: swap, hibernate, snapshot, pagefile, core_dump.

## Honesty

| Claim | Value |
|---|---|
| PRIVATE_DATA_READY | false |
| PROTECTION_MEASURED (swap/snapshot/…) | false |
| OS_KEYRING_SWAP_SNAPSHOT_PRIVATE_DATA | remains OPEN |

## Platform notes

- Windows: pagefile.sys / hiberfil.sys / swapfile.sys; VSS via System Volume Information + optional `vssadmin list shadows`; Minidump/WER paths.
- Linux: `/proc/swaps`, `/sys/power/disk`, common snapshot dirs + btrfs sysfs hint, `/proc/sys/kernel/core_pattern`.
- macOS: `/private/var/vm` swapfiles + sleepimage; `tmutil listlocalsnapshots`; `/cores`.
