# Contracts — Spec 086 R Workspace

All types live in `medscale-contracts/src/r_workspace.rs`, use
`deny_unknown_fields`, and carry `R_WORKSPACE_SCHEMA_VERSION = 1`.

| Authority name | Type | Notes |
|---|---|---|
| `RWorkspaceManifest` | `RWorkspaceManifest` | Project, label, layout v1, stage root, `csv_copy`, `explicit_publish_only`, class + basis, inputs, generated files (sorted, exactly the layout's files) |
| — | `RWorkspace` | manifest + descriptor digest (the manifest's canonical JSON) |
| `RWorkspaceState` | `WorkspaceIntegrity` | `intact`, `changed`, `missing` (observed, never stored) |
| `RRuntimeIdentity` | `RRuntimeIdentity` | configured `Rscript` path, found, file digest, `version = not_probed` |
| `RLockfileRef` | `RLockfileRef` | `renv.lock` digest and size (evidence only) |
| `RLaunchRequest` / `RLaunchReceipt` | same | IDE kind; state `launched`, `ide_not_configured`, `ide_not_found`, `workspace_missing`, `workspace_changed`, `spawn_failed`; environment variable names only |
| `RRunRequest` / `RRunReceipt` | same | refusal `compute_denied_platform_unqualified`, `workspace_missing`, `workspace_changed`, `input_stale`, `script_invalid`; script digest, runtime, lockfile |
| `RPublishRequest` / `RPublishReceipt` | same | optional expected digest; state `published` / `refused`; refusals `bad_name`, `not_found`, `not_regular_file`, `too_large`, `output_changed`, `output_invalid`, `workspace_missing`, `workspace_changed`, `input_stale` |
| — | `RPublishedTable` | derived, `reviewed=false`, class of the workspace, `derived_from` = inputs in order |

Bounds: 16 inputs; label 64 characters; names 128 characters; publish
16 MiB; evidence files 4 MiB; program digest 256 MiB; staged CSV 512 MiB;
64 `outputs/` entries per inspection.

Launch environment allowlist: `LAUNCH_ENV_ALLOWLIST` (locale, temporary
directory, display, home and Windows system variables); every name
starting with `MEDSCALE` is removed even if listed.
