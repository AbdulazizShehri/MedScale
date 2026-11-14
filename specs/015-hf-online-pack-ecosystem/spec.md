# Feature Specification: HF + Online Pack Ecosystem (READY_BASE)

**Branch**: `spec/015-hf-online-pack-ecosystem`  
**Status**: QUALIFIED for READY_BASE (008+013 CLOSED; 009 pack-format constraints present; HF credentials/terms EXTERNAL_GATE)  
**Input**: Online pack acquisition contracts that require Network Broker; fail-closed when `HF_ONLINE_PACK_DISTRIBUTION` not granted; TUF/offline-root policy stubs; mobile pack-format constraints from Spec 009. No HF SDK, no uncontrolled provider network, no runtime HF dependency.

## User Stories

### US1 Online acquire refuses without HF gate (P1)
`OnlinePackAcquire` returns `ExternalGateRequired(HF_ONLINE_PACK_DISTRIBUTION)` and never opens sockets.

### US2 Broker is mandatory for any future online path (P1)
Contracts declare `broker_required=true`; doctor reports online path unauthorized and broker_required.

### US3 Mobile format constraints wired (P1)
Acquire request carries chunked/per-arch/16KB/TUF constraints from Spec 009 evidence.

### US4 Offline Pack v0 remains usable (P1)
Local `PacksInstallLocal` unchanged; online path does not weaken offline-only default.

## Anti-scope
Accepting HF terms; production credentials; direct HF HTTP client; Sigstore live publish; real model weights download.
