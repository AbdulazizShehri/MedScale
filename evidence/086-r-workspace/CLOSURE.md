# CLOSURE — Spec 086 R Workspace (foundation slice)

## Terminal truth

```text
SPEC_086_CLOSED_CANONICAL=true
MERGE_SHA=16f2d1fdd600caeb1e5fe29c88b0d06578d0bfae (PR #152)
FINAL_HEAD=c8e855176a8d7e0cdbaa9539351eb4dd74d58c96
EXACT_HEAD_CI=36241268247 (6/6; ubuntu 899 passed / 0 failed / 1 ignored,
  windows 895 / 0 / 1, macOS 897 / 0 / 1)
POST_MERGE_MAIN_CI=36246757563 (6/6 on 16f2d1f)
BASE=61855e4 (PR #151 merge, Spec 085 closure; post-main run 36237543003 6/6)
REVIEW_POLICY=FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22 (no external reviewer;
  deterministic scope record in EXACT_RANGE_REVIEW.md; security challenge in
  SECURITY.md)
STAGING=exact Spec 075 snapshots as read-only canonical CSV copies, outside the vault
EXTERNAL_LAUNCH=RStudio / Positron / folder opener; absolute configured program,
  no shell, stdio closed, allowlisted environment
MANAGED_R_EXECUTION=NOT_ADMITTED (requests recorded and refused)
PUBLICATION=explicit, one regular file, digest-pinned, exact CSV typing,
  derived and unreviewed, class inherited (local_phi in practice)
NETWORK_PATH=none in MedScale; the external IDE is outside MedScale's authority
SANDBOX=platform_qualified=false (unchanged)
DEPENDENCIES_ADDED=none
STORAGE_SCHEMA=v15 (v14->v15 additive; 14 backup tamper cases)
REAL_PHI_AUTHORIZED=false
RELEASE_READY=false
PRIVATE_DATA_READY=false
SPEC_087_IMPLEMENTATION_AUTHORIZED=false until its own promotion
```

## What this closure establishes

- Core stages exact snapshots (digest-verified on load) into a new
  directory under a host-configured staging directory, which must not be
  the vault or inside it; the data copies are read-only and their bytes
  are a pure function of the snapshots.
- The workspace is opened by a configured external program with the
  workspace path as its only argument and an allowlisted environment;
  every launch attempt has a receipt that records variable names only.
- A managed `Rscript` run is validated, bound to the script, `renv.lock`
  and `Rscript` digests, recorded, and refused. MedScale starts no R.
- Outputs return only through explicit publication: link and bound
  checks, an optional expected digest, exact CSV typing by the Spec 075
  parser with no dropped rows, and one transaction for the receipt and the
  derived, unreviewed table.
- A changed or missing workspace and a stale input block launch, run and
  publication. Everything survives restart and backup/restore; tampered
  rows and backups are refused.

## Closure-gate reading

The authority's gate is "a user launches a reproducible R workspace from
exact MedScale snapshots, runs/publishes output explicitly, and no R
process receives ambient vault authority". Launch and explicit
publication are met. **Running R through MedScale is `NOT_ADMITTED`**:
user R is arbitrary code and no platform-qualified sandbox exists. This
is a recorded gate, not a pass.

## Defects found and fixed during qualification

- CI Clippy found `field_reassign_with_default` in the Core test host
  builder (`dc2fd47`).
- The security challenge added CLI escaping of requested names
  (`dc2fd47`).

## Honest residuals (non-blocking, recorded)

- The external IDE and any R it runs execute as the OS user with that
  user's filesystem and network rights; MedScale hands it only the staged
  copy but does not confine it (it could read the vault directory).
- Managed R execution, package restoration and network denial for R need
  a platform-qualified sandbox and a new canonical decision.
- The R version is not probed; `Rscript` is identified by path and file
  digest; `renv.lock` is evidence only; no Arrow/Parquet staging.
- On Windows the pre-open link check compares lengths only.
- A crash during staging can leave an unregistered `.partial` directory.
- Snapshots carry no Spec 079 classification row, so every workspace is
  `local_phi`.
- No Desktop surface; no rendered Desktop evidence.
- No local compile or test run on this workstation; GitHub Actions is the
  compiler of record.
