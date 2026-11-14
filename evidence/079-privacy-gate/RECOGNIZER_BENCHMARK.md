# Recognizer Benchmark — Spec 079 (T079-03)

## What this is and is not

A class-wise count over a small **synthetic development corpus** written
alongside the recognizers. It is a regression floor, not a held-out
benchmark and not evidence of real-world recall. No real PHI is used.

```text
CORPUS   = crates/medscale-core/tests/fixtures/privacy_079/corpus.json
           6 documents: English-language notes with en-US, en-GB, es-ES,
           fr-FR and de-DE value formats, plus one FHIR R4 Patient (JSON)
TEST     = medscale-core lib test
           authority::privacy_recognizers::tests::synthetic_corpus_class_wise_counts
RECOGNIZERS = medscale.privacy.pattern v1 (deterministic)
              medscale.privacy.fhir v1 (structured FHIR)
              (the local model recognizer is exercised separately; the
              fixture Pack is not a qualified PHI recognizer)
MATCH RULE = an expected value counts as found only when a detected span has
             the same kind and exactly the same text
```

## Result

Observed locally on 2026-09-23 (`cargo test -p medscale-core --lib
privacy_recognizers::tests::synthetic_corpus -- --nocapture`, WSL Ubuntu,
rustc 1.97.1), and asserted in CI by the same test (missed must be 0):

| Kind | Expected | Found | Missed | Extra |
|---|---:|---:|---:|---:|
| date | 9 | 9 | 0 | 0 |
| email | 3 | 3 | 0 | 0 |
| identifier | 7 | 7 | 0 | 0 |
| ip_address | 1 | 1 | 0 | 0 |
| person_name | 12 | 12 | 0 | 0 |
| phone | 6 | 6 | 0 | 1 |
| postal_address | 7 | 7 | 0 | 0 |
| postal_code | 5 | 5 | 0 | 0 |
| url | 2 | 2 | 0 | 0 |

The one extra: in `note-en-gb`, the NHS number `943 476 5919` is detected as
a **phone** number. It is still transformed (redacted under the standard
profile), so it does not leak, but it is misclassified. Recorded, not hidden.

## Known limitations

- The corpus was written with the recognizers; 100% found on it says nothing
  about unseen text. Real recall is unmeasured.
- Names are found only after an honorific or a label (`Patient:`, `Dr.`,
  `Frau`, ...). Bare names in running text are missed by the pattern
  recognizer.
- Narrative text inside FHIR resources is scanned by pattern rules only.
- Non-English narrative text and non-Latin scripts were not evaluated
  (founder English-only directive; see `SPEC_079_PROMOTION.md`).
- Every receipt carries `automated_recognition_is_incomplete`.
