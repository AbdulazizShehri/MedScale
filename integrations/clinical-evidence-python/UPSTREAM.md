# OpenMed-derived clinical evidence modules

- Upstream: https://github.com/maziyarpanahi/openmed
- Exact source commit: `3e2323288ec551157c0b943b331f399ef1434649` (branch `master` at acquisition)
- Original source license: Apache-2.0 (see third_party/openmed/LICENSE).
- Original upstream NOTICE preserved in third_party/openmed/NOTICE.
- Changes: modules copied into the `medscale_clinical` namespace,
  product identifiers replaced, selected imports adjusted, and minimal
  package initializers supplied. All eight corresponding upstream test
  modules were ported to the renamed namespace. No weights, datasets,
  cloud adapters, or background network services were copied.
- Integration: an isolated, optional Python source package. Importing it
  does not automatically activate any clinical authority in MedScale's Rust
  desktop or FHIR flows. Clinical outputs require qualification and review.
- The copy script fetches from GitHub when explicitly run by a developer;
  importing and executing the copied library does not fetch source code.

## Tranche 2 — additional source-derived clinical safety modules

- Exact upstream commit unchanged: `3e2323288ec551157c0b943b331f399ef1434649`.
- 26 additional modules and their original test files were copied and renamed
  with the reproducible `scripts/vendor_openmed_clinical_v2.py` importer.
- Some source tests exercise broader upstream modules that are deliberately
  **not** shipped: SDOH extractor/sections, NLI inference, and review storage
  transitions. The three complete upstream reference tests are preserved in
  `tests/upstream_reference/*.py.txt` and the independent pure tests are
  run. These are documented as exclusions, not as passing tests.
- The original Apache-2.0 license/NOTICE are preserved under `third_party/`;
  each adapted Python implementation has an explicit derivative header.
- For an inventory and validation evidence, see `FEATURE_PARITY_V2.md`.
- Neither tranche grants real-PHI processing authorization or release readiness.
