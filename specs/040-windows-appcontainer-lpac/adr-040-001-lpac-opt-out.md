# ADR-040-001 — LPAC via ALL_APPLICATION_PACKAGES_OPT_OUT

## Status

Accepted

## Context

Specs 033/038 measure regular AppContainer FS/network deny. EXTERNAL_GATES and Spec 038
left LPAC as scaffold. Workers need Less Privileged AppContainer evidence without claiming
multi-OS PLATFORM_QUALIFIED.

## Decision

Launch measured children with zero capabilities plus
`PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT`. Prove LPAC identity by absence of
ALL APPLICATION PACKAGES group, plus FS and TCP deny.

## Consequences

- Doctor gains `windows_appcontainer_lpac_measured`
- Scaffold no longer claims LPAC as unmeasured residual
- Gate remains OPEN for composition + macOS App Sandbox entitlements
