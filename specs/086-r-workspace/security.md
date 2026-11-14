# Security — Spec 086 R Workspace

| Threat | Control | Test |
|---|---|---|
| Arbitrary R executed by MedScale | No code path starts R; run requests are refused and recorded | core `managed_runs_are_recorded_and_refused_never_executed` (the configured "Rscript" would write files if run; none appear) |
| Vault exposure to the workspace | Staging directory must be outside the canonical vault; only rendered copies are written; no vault path or secret in any file | core `staging_writes_exact_read_only_copies_outside_the_vault`, `staging_refusals_create_nothing` |
| Path traversal via request | No request carries a path; labels, script and output names are plain names | contract `names_are_plain`; core publication cases |
| Symlink / junction escape | `lstat` before open; Unix device + inode match and post-read re-check; a linked `outputs/` is not followed | core `publication_refuses_unsafe_or_inexact_outputs`, `a_linked_outputs_directory_is_not_followed` (Unix) |
| Environment secret leakage to the IDE | `env_clear` plus allowlist; `MEDSCALE*` always removed; receipt stores names only | contract `launch_environment_is_allowlisted`; core launch test inspects what the child saw |
| Shell injection / `PATH` hijack | Absolute program path only, `Command` without a shell, one argument | core launch test (relative program is `ide_not_found`) |
| Giant outputs | 16 MiB publish bound checked before and while reading | core `big.csv` case |
| Inexact import | Non-UTF-8, header-only, duplicate headers and ragged rows refused | core publication cases |
| Tampered workspace / stale snapshot | Digest check of every generated file; input currency | core tamper and stale tests |
| Tampered storage / backups | Column-body checks, receipt-to-workspace binding, table-to-receipt match, consistency after restore | storage tamper tests |
| Cross-project inputs | Snapshot must be the Project's | core refusals test |

Not controlled (recorded residuals): the external IDE and anything it runs
execute as the OS user with that user's filesystem and network rights;
MedScale does not sandbox them.
