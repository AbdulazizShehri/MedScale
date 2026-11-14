# Spec 051 — REQUIRED_CHECKS live CI sync

## Overview

Synchronize the Spec 022/037 REQUIRED_CHECKS owner packet with live
`.github/workflows/ci.yml` job names, including
`perf delivery-plan scale (windows)` introduced by Spec 042.

## Requirements

### Requirement: Live job inventory

The REQUIRED_CHECKS evidence MUST list every non-optional primary CI job name
exactly as declared in `ci.yml` `name:` fields for jobs that gate Trusted V1
integrity (rust matrix OS jobs, cargo-deny, supply-chain policy, perf delivery-plan scale).

### Requirement: Owner action honesty

The packet MUST continue to state that configuring branch protection is an
owner GitHub setting and that Cursor cannot mark the gate closed by editing
docs alone.

### Requirement: No false readiness

Updating the packet MUST NOT claim RELEASE_READY, PRIVATE_DATA_READY, or that
branch protection is enabled.

## Success criteria

- REQUIRED_CHECKS.md lists all six live job names (including perf).
- Doctor/evidence note that the packet was refreshed under Spec 051.
- Tests assert packet contains the perf job name string.
