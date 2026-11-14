# Legal flow decision-record capability

**Date:** 2026-08-26  
**Gate:** `LEGAL_COUNSEL_FLOW_MAPPING` = `PENDING_WHEN_REQUIRED`

## Delivered

- `medscale_contracts::legal::FlowDecisionRecord` schema v1
- PendingCounsel default with structural validity rejecting invented conclusions

## Not delivered

- Any PDPL/SFDA/controller/processor lawful-basis conclusion
- Counsel acceptance workflows

Counsel fills `counsel_conclusion` later; MedScale stores records only.
