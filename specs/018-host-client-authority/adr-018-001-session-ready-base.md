# ADR-018-001: In-process session READY_BASE

**Status**: Accepted  
**Decision**: Ship in-process `SessionRegistry` bound to existing lease holder IDs before OS IPC transport. Spec 016 OS writer lock remains the store integrity primitive. Do not claim multi-client release readiness. Future OS IPC must re-qualify this ADR.
