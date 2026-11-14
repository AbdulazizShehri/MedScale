# macOS App Sandbox entitlements ReadyBaseMeasured (Spec 041)

## Measure
- Entitlements fixture validates `com.apple.security.app-sandbox = true`
- Detection probe: `APP_SANDBOX_CONTAINER_ID` presence (`app_sandbox_container_active`)
- Unsigned CI/CLI: probe reports inactive (Seatbelt ≠ App Sandbox)

## Non-claims
- Runtime entitlement **enforcement** not measured (`enforcement_measured=false`)
- Codesign / notarization require SIGNING_ACTION (EXTERNAL_GATES / Spec 039 prep)
- Not multi-OS PLATFORM_QUALIFIED
- Seatbelt Spec 031 remains separate

## Binding

| Field | Value |
|---|---|
| OS | macOS (CI macos-latest) + cross-OS artifact validation |
| TEST/PROBE | `OsSandboxPlan::macos_app_sandbox_entitlements_ready_base` / `os_sandbox_041` |
| EXPECTED | artifact valid; apply Ok on macOS; NotReady off-macOS; enforcement_measured=false |
| EXIT_STATUS | *(CI)* |
