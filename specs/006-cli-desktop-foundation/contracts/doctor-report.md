# Contract: Doctor Report (Spec 006)

**Spec**: 006-cli-desktop-foundation  
**Status**: CLI/operational contract  
**Scope**: `medscale doctor`; synthetic-only; no secrets; no REAL_PHI

## Command

```text
medscale doctor [--json]
```

Default: human-readable lines. `--json`: single `DoctorReport` object (serde).

## Required axes (MUST)

1. Product identity + version  
2. Local-only / network posture (`DEFAULT_DENY`)  
3. Vault location (path or default policy class) + vault status  
4. Sync/remote-root risk  
5. Key-store availability  
6. Privacy evidence freshness (points at PRIVACY_PROOF artifact when present)  
7. Filesystem / OS claim status  
8. Packs/runtime placeholder (`not_implemented` until Spec 008)  
9. Network broker placeholder (`not_implemented` until Spec 013)  
10. Limitations / notes (short)

## Response sketch

```text
DoctorReport {
  product_name, version,
  local_only: true,
  network_posture: DefaultDeny,
  vault: { status, root_path?, root_class?, vault_id?, encryption_profile? },
  sync_remote_risk: { status, detail? },
  key_store: { availability, backend_label? },
  privacy_evidence: { status, artifact_id?, observed_at?, path? },
  filesystem_claim: { status, notes? },
  packs_runtime: { status: NotImplemented },
  network_broker: { status: NotImplemented },
  limitations: [string]
}
```

## Behavioral rules

1. Doctor MUST NOT print passphrases, recovery codes, DEKs, or keyring secret payloads.
2. Doctor MUST NOT claim WebView/Tauri privacy PASS while Tauri is deferred.
3. Missing PRIVACY_PROOF → `privacy_evidence.status = Missing` (not silent Fresh).
4. Doctor is read-mostly; it MUST NOT create a vault as a side effect of reporting.
5. When vault is open in-session, doctor MAY report richer status still without secrets.

## Exit codes

- Success with report: 0  
- Doctor aggregation failure (unexpected): 1  
- Never “success” with empty required axes

## Tests required

- All required axes present in `--json`
- Secret-marker scan on stdout/stderr = 0 hits
- Bootstrap Spec 001 fields superseded (no permanent `vault: not_implemented` when 005 vault APIs exist—use typed NotConfigured/Open/etc.)
