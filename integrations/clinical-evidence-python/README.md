# MedScale Clinical Evidence (Python)

An optional, **offline-capable** MedScale Python package containing an Apache-2.0
source port of selected OpenMed clinical evidence validators. Its public import
namespace is `medscale_clinical`, not `openmed`.

## Included capabilities

- Evidence span overlap audit (metadata only; no original text in reports).
- Numeric NLI precheck (magnitudes, units, reference intervals).
- Claim conflict and evidence recency review.
- Citation ordering, minimality, and summary citation consistency.
- Claim packet integrity verification.

The 14 source files and eight transplanted upstream test modules are pinned to
one immutable OpenMed commit. See [UPSTREAM.md](UPSTREAM.md) and the preserved
upstream [LICENSE](third_party/openmed/LICENSE) and
[NOTICE](third_party/openmed/NOTICE).

## Additional audit and safety validators (tranche 2)

A second, source-pinned port contributes 26 further modules including guarded
clinical-decision-support envelopes, explicit uncertainty disclosures,
NLI abstention/batching/premise guards, empty-evidence and omission gates for
summaries, SDOH prohibited-use labels, lab reference-range provenance and
human-review state transitions. See [FEATURE_PARITY_V2.md](FEATURE_PARITY_V2.md)
for the exact file inventory, excluded whole-stack tests and implementation
limits. Neither tranche is activated inside MedScale Desktop or Rust Core.

## Installation and tests

From this directory in an isolated Python 3.11+ virtual environment:

```bash
python -m pip install -e ".[test]"
python -m pytest tests -q
```

There is no runtime dependency on the original OpenMed package, a hosted API,
or model weights. The optional test dependencies are development-only. The
vendor-copy script requires `gh` and GitHub access only when deliberately run
by a developer.

## Safety and integration status

These validators are assistive review primitives, **not** autonomous clinical
reasoning, approved diagnosis, or treatment advice. Numeric checks require
already-paired structured values; citation checks require supplied sources and
coordinates. They do not establish semantic entailment or clinical safety.

This optional Python package is **not yet wired into MedScale's Rust authority
plane, native desktop workflow, or FHIR clinical decision pathway**. Use only
synthetic test data until separate clinical, security, and platform gates pass.
