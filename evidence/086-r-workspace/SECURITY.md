# Security challenge — Spec 086 R Workspace

Deterministic challenge per `FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`
(no external or LLM reviewer). Each row names the attack, the control and
the test that exercises it. `PASS` is recorded only when exact-head CI has
run the test (see `EXACT_HEAD_QUALIFICATION.md`).

| # | Attack | Control | Evidence |
|---|---|---|---|
| S01 | MedScale executes user R (arbitrary code) | No code path starts R or an interpreter; run requests refused with `compute_denied_platform_unqualified` | core `managed_runs_are_recorded_and_refused_never_executed`: the configured "Rscript" is a program that writes files when run; after the request, `outputs/` is empty |
| S02 | Workspace staged inside the vault, exposing DB and blobs | Canonical staging directory must not be the vault or inside it (`PathOutsideClaim`) | core `staging_refusals_create_nothing` |
| S03 | Vault path, DB name or secrets written into the workspace | Only rendered CSV, schema, README, `.Rproj` and descriptor are written | core `staging_writes_exact_read_only_copies_outside_the_vault` scans every staged file |
| S04 | Parent-directory traversal through names | Labels, scripts and outputs are plain names; no request carries a path | contract `names_are_plain`; core publication cases (`../...`, `data/x.csv`, `C:\x.csv`) |
| S05 | Symlink escape from `outputs/` or `scripts/` | `lstat` before open; Unix device and inode must match the opened file and the path is re-checked after reading; a linked `outputs/` directory is not followed | core `publication_refuses_unsafe_or_inexact_outputs`, `a_linked_outputs_directory_is_not_followed`, linked script case (Unix) |
| S06 | Environment secrets inherited by the IDE | `env_clear` and an allowlist; any `MEDSCALE*` name removed; receipts hold names only; storage refuses a receipt with a non-allowlisted name | contract `launch_environment_is_allowlisted`; core launch test reads the names the child saw; storage invariant and tamper tests |
| S07 | Shell injection or `PATH` hijack via the program | Absolute configured path, `Command` without a shell, workspace as the only argument, stdio closed | core launch test: relative and missing programs never start |
| S08 | Giant output | 16 MiB bound checked before reading and while reading | core `big.csv` case |
| S09 | Inexact or lossy import | UTF-8 only; header-only, duplicate headers and ragged rows refused (the Spec 075 parser would drop ragged rows) | core publication cases |
| S10 | Output swapped after the user inspected it | Optional expected digest; mismatch is `output_changed`; the receipt pins the digest of the bytes actually read | core launch test (digest mismatch) |
| S11 | Staged data edited, descriptor forged, workspace removed | Every generated file and the descriptor are digest-checked before launch, run and publish | core `a_changed_or_missing_workspace_blocks_launch_run_and_publish` |
| S12 | Stale or replaced snapshot | Input currency check (Project, content digest, schema fingerprint) before run and publish | core `a_stale_input_blocks_publication` |
| S13 | Cross-project input | A snapshot must belong to the staging Project | core `staging_refusals_create_nothing` |
| S14 | Tampered rows or backups (class weakened, table marked reviewed, receipt moved, content edited, duplicate rows, run claims execution) | Column-body checks, receipt-to-workspace binding, table-to-receipt and table-to-workspace match, consistency after restore | storage `tampered_r_workspace_rows_fail_closed_on_read`, `tampered_r_workspace_backups_are_refused` (14 cases) |
| S15 | Partial publication after a crash | Receipt and table committed in one transaction | storage `r_workspace_rows_hold_their_invariants` (a failing table write leaves no receipt) |
| S16 | Terminal escape sequences in requested names | CLI prints requested script and output names with `escape_debug` | code review of `medscale-cli/src/r_workspace.rs` |

## Not controlled (recorded, honest)

- The external IDE and any R it runs execute as the OS user, with that
  user's filesystem and network rights. MedScale hands it nothing but the
  staged copy, but it can read the vault directory on disk and use the
  network. MedScale does not claim to sandbox it (`platform_qualified=false`).
- Package installation, restoration and network access by R are the
  user's; MedScale neither performs nor prevents them. `renv.lock` is
  recorded as evidence only.
- On Windows the link check compares lengths only (no stable file-index
  API without new code), so a swap between the check and the open is not
  fully excluded; on Unix device and inode are compared.
- A local attacker who can write to the staging directory can race the
  `.partial` directory between creation and rename. The staging directory
  is the user's own choice; this is not defended.
- One inspection may read up to 64 files of 16 MiB each.
