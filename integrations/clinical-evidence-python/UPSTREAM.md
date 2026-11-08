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
