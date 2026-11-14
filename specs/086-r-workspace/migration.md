# Migration — Spec 086 R Workspace

- Storage v14 -> v15 is additive: five `rws_*` tables created inside
  `begin/finish_migration(15)`; no existing table changes.
- Backups written by this build carry `schema_version: 15` and five
  required families; `restore_v15` replays v14 first, then the R
  Workspace rows through plain-insert paths, re-checks table bytes, and
  runs `verify_r_workspace_consistency`.
- v14 (and older) backups restore with empty R Workspace tables.
- Staged directories are not part of a backup. A restored workspace whose
  directory is absent inspects as `missing`; publishing from it is
  refused; staging again creates a new workspace.
- Recovery: nothing is left running. Staging writes a `.partial`
  directory and renames it before the row is committed; a crash leaves at
  most an unregistered `.partial` directory. Publication commits its
  receipt and table in one transaction.
