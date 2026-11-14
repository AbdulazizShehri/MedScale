# Security challenge — Spec 089 Research Packs

| # | Attack | Control | Evidence |
|---|---|---|---|
| S01 | A Pack smuggling code or capabilities | Packs are compiled-in declarative manifests; field kinds and migration steps are closed vocabularies; installing grants nothing and installs no Extension | contract vocabularies; Core tests (no capability path) |
| S02 | Forged or unknown Pack version | Installs pin the shipped manifest digest; storage re-checks every install and artifact against the shipped version | storage `rows_are_checked_against_the_shipped_pack`, backup tamper cases |
| S03 | Invalid domain data | Every create/update is validated against the schema (kinds, required fields, choices, dates) | core workflow test; contract field tests |
| S04 | Skipping workflow steps | Only declared transitions; compare-and-set revisions | core workflow test (`draft -> approved` refused; stale revision conflicts) |
| S05 | Manufactured certainty | Evidence axes are explicit with `unknown`; a missing citation cannot support; a retracted one is not support; the verdict is `supported` only when all axes are recorded | contract `evidence_axes_stay_explicit`; core assessment case |
| S06 | Destructive or partial migration | Only additive steps; install and all artifacts upgrade in one transaction | core upgrade test; storage upgrade case |
| S07 | Uninstall destroying data | Uninstall disables; artifacts stay readable and immutable until reinstall | core upgrade/uninstall test |
| S08 | Cross-project access | Installs and artifacts are per Project | core test (other Project sees nothing) |
| S09 | Tampered rows or backups | Column-body checks; consistency after restore | storage tamper tests |

Not claimed: citations are recorded as given, not resolved; no clinical
correctness of any assessment.
