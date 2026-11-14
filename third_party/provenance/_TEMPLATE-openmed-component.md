# Provenance template — OpenMed component (Spec 007+)

Fill **before** the first donor-derived line enters MedScale (`COPY_BOUNDED` / `PORT_TO_RUST` / etc.).

| Field | Value |
|---|---|
| Component id | |
| Upstream URL | `https://github.com/maziyarpanahi/openmed` |
| Revision / tag | `v2.2.0` / `59d9cb0a2e0ccbba8fa3d891a66d83ffaf45e837` |
| Original path(s) | |
| Content identity (hash) | |
| License / NOTICE / permission | |
| Local target path | |
| Modifications | |
| Transitive dependencies | |
| Security placement (P0–P3 / N/A) | |
| Owning MedScale spec | |
| Operation | `COPY_BOUNDED` \| `PORT_TO_RUST` \| `REFERENCE_ONLY` \| … |
| Tests | |
| Update strategy | |
| Exit strategy | |

## Explicit non-permissions

- Code permission does **not** imply model weights, datasets, or restricted terminology.
- Never wholesale-copy the OpenMed repository.
- Never import OpenMed Python as MedScale trusted runtime.

## Anti-scope reminders

| Forbidden | Status |
|---|---|
| OpenMed product runtime dependency in Cargo.toml | Must remain absent |
| REAL_PHI corpora | NOT_AUTHORIZED |
| MESC mutation | NO |
