# Windows AppContainer network ReadyBaseMeasured (Spec 038)

## Measure
- Profile create/derive + `SECURITY_CAPABILITIES` with `CapabilityCount=0`
- Child `appcontainer-net-child` attempts TCP connect to `203.0.113.1:9` (TEST-NET-3)
- PASS only on PermissionDenied / ERROR_ACCESS_DENIED / WSAEACCES

## Non-claims
- Not LPAC measured
- Not multi-OS PLATFORM_QUALIFIED
- EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` remains OPEN
