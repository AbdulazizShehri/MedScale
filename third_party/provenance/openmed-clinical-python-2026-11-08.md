# Provenance: OpenMed clinical evidence modules (Python reference package)

| Field | Value |
|---|---|
| Component id | `openmed-clinical-python` |
| Upstream URL | `https://github.com/maziyarpanahi/openmed` |
| Revision / tag | `master` / `3e2323288ec551157c0b943b331f399ef1434649` |
| Original path(s) | 34 modules under `openmed/clinical/`, `openmed/clinical/units/`, `openmed/clinical/lexicons/clinical_norm.py`, `openmed/core/{audit,iso_temporal}.py`, and their tests under `tests/unit/clinical/` |
| Content identity (hash) | Per file: upstream Git blob id and local SHA-256 in `integrations/clinical-evidence-python/UPSTREAM.md` ("File inventory") |
| License / NOTICE / permission | Apache-2.0. Upstream `LICENSE` and `NOTICE` kept verbatim in `integrations/clinical-evidence-python/third_party/openmed/` and included in built wheels (`license-files`) |
| Local target path | `integrations/clinical-evidence-python/` (package `medscale_clinical`) |
| Modifications | Namespace and product-name substitution; three test files reduced to self-contained tests (full originals kept, not run); explicit package exports; two functions that need unvendored upstream files now raise. Each derived file carries a change notice naming its upstream path |
| Transitive dependencies | None at runtime (standard library only). Tests: `pytest`, `pytest-subtests` |
| Security placement (P0–P3 / N/A) | P3: developer reference and conformance oracle only; not loaded by Core, the CLI or the desktop app |
| Owning MedScale spec | Clinical evidence integration (PRs #477, #478) |
| Operation | `REFERENCE_ONLY` for the Python package; `PORT_TO_RUST` for each check Core uses (`crates/medscale-clinical`), held to the reference by conformance vectors |
| Tests | 438 Python tests and 182 subtests; Rust conformance tests in `crates/medscale-clinical/tests/` |
| Update strategy | Re-pin a new upstream commit, re-run the importers on a fresh tree, regenerate the inventory and the conformance vectors |
| Exit strategy | Delete `integrations/clinical-evidence-python/`; the Rust ports and their checked-in vectors stand alone |

## Explicit non-permissions

- Code permission does **not** imply model weights, datasets, or restricted terminology. None were copied; lab reference ranges ship no numeric values.
- Never wholesale-copy the OpenMed repository.
- Never import OpenMed Python as MedScale trusted runtime. This package satisfies that rule only because Core never runs it: Core uses the Rust ports.

## Anti-scope reminders

| Forbidden | Status |
|---|---|
| OpenMed product runtime dependency in Cargo.toml | Absent |
| REAL_PHI corpora | NOT_AUTHORIZED |
| MESC mutation | NO |
