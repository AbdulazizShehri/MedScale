# Feature Specification: Synthetic Lexical 10k Corpus (Q10/Q05 residual)

**Feature Branch**: `spec/045-lexical-10k-corpus`  
**Promotion**: `EXISTING_Q10_Q05_RESIDUAL_ELIGIBLE_FOR_PROMOTION`  
**Does not**: claim clinical quality, RELEASE_READY, or budgets_claimed_met.

## Requirements

- **FR-001**: Procedural scale corpus generator for 10_000 synthetic docs.
- **FR-002**: Distinct corpus id `synthetic-lexical-scale`; default builtin unchanged.
- **FR-003**: Perf harness uses scale corpus under delivery-plan / LEXICAL_SCALE env.
- **FR-004**: Doctor `scale_corpus_generator_present=true`; clinical_quality/release_ready false.
- **FR-005**: Evidence under `evidence/045-lexical-10k-corpus/`; advanced deferred **046+**.
