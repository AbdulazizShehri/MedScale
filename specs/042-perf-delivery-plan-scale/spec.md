# Feature Specification: Perf Delivery-Plan Scale (Q05 residual)

**Feature Branch**: `spec/042-perf-delivery-plan-scale`  
**Status**: READY  
**Promotion**: `EXISTING_Q05_RESIDUAL_ELIGIBLE_FOR_PROMOTION`  
**Does not**: claim budgets met; RELEASE_READY; 10k lexical corpus.

## Requirements
- **FR-001**: Harness supports `MEDSCALE_027_DELIVERY_PLAN_SCALE=1` → FHIR 1 MiB + elevated timeline (CI 1000; host may set 10000).
- **FR-002**: Evidence written under `evidence/042-perf-delivery-plan-scale/` with full binding fields.
- **FR-003**: `budgets_claimed_met=false`; CI default scale unchanged.
- **FR-004**: Lexical honesty: builtin corpus not 10k records.
