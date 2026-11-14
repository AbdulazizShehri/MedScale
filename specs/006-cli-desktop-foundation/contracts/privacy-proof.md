# Contract: PRIVACY_PROOF Evidence Artifact (Spec 006)

**Spec**: 006-cli-desktop-foundation  
**Status**: Typed evidence contract (MASTER_BUILD_PLAN §14; GLM F-17)  
**Scope**: Spec 006 claim-scoped privacy evidence; synthetic-only; no system-wide zero-packet claim

## Schema

```text
schema_id: medscale.privacy_proof.v1
```

## Artifact location

```text
evidence/006-cli-desktop-foundation/PRIVACY_PROOF.json
```

Companion `limitations.md` MAY duplicate the limitations array for humans.

## Required sections

| Section | Spec 006 expectation |
|---|---|
| `claim_scope` | MedScale CLI + in-process Core Host attributable boundary |
| `network_observation` | No product runtime egress clients; DEFAULT_DENY by absence |
| `dependency_capability_audit` | Workspace product path deps; note clap/anyhow; **tauri absent** |
| `log_crash_marker_scan` | CLI/Core Host scans for synthetic secret markers → 0 hits |
| `webview_cache_scan` | **`NotApplicable`** — Tauri/WebView not admitted |
| `vault_sync_root_evidence` | References doctor + Spec 005 claim/refuse evidence |
| `limitations` | MUST list items below |

## Mandatory limitations (minimum)

1. Synthetic-only; REAL_PHI NOT_AUTHORIZED  
2. Does not claim zero packets system-wide outside MedScale attributable processes  
3. Tauri v2.11.5 / WebView PHI containment **not proven**; Desktop WebView **deferred**  
4. No Network Broker / partner egress qualification (Spec 013)  
5. No mobile entitlement/manifest proof (Spec 009)  
6. v0 visual Desktop UI may be deferred if `imports/v0/` absent  
7. Packs/runtime privacy not in scope (Spec 008)

## Generation rules

1. Produce after doctor + wedge + secret-scan tests.  
2. `doctor` privacy freshness SHOULD reference this artifact’s id/path when present.  
3. Do not mark `webview_cache_scan` as Pass while Tauri is deferred.  
4. Do not treat PRIVACY_PROOF as legal counsel PDPL/SFDA sign-off.

## Pass / fail for Spec 006 exit

| Check | Required |
|---|---|
| Artifact present with schema_id | yes |
| limitations include Tauri deferred + synthetic-only | yes |
| webview section NotApplicable or Fail (not silent Pass) | yes |
| network section consistent with DEFAULT_DENY | yes |
| Related CI tests archived or linked | yes |

## Non-goals

- Absolute OS-wide packet capture certification  
- Admitting Tauri by writing optimistic Pass text  
- Authorizing REAL_PHI
