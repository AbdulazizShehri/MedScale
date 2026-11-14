# Spec 090 — Institutional Adapters (object storage path)

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promotion date:** 2026-09-27
**Canonical base:** `8df3e45f1ae402f04a9c000c13ad4af466159b3a` (Spec 089 closure merge)
**Target branch:** `spec/090-institutional-adapters`

## Authority

Dependency proof: `RESEARCH_OS_V2_SPEC_IMPLEMENTATION_CONTRACTS.md` and
`RESEARCH_OS_EXECUTION_ROADMAP.md` number Institutional Adapters **090**,
hard dependency **075 + 079 + 084 + 085 + 089** (086 for R institutional
integration; 087 for community-delivered executable adapters, neither in
this slice). All are `CLOSED_CANONICAL`. This spec admits no dependency.

Network truth: product runtime egress is default-deny
(`PRODUCT_RUNTIME_NETWORK_EGRESS = DEFAULT_DENY`); real institutional
endpoints and credentials are external gates (`PRODUCTION_CREDENTIALS`).

## Scope

The closure gate asks for one identity/storage/compute institutional path
proven end to end with outage, revocation and rollback and no authority
split. This slice proves the **storage** path:

- adapters per Project with declared destination, data-class ceiling
  (never `local_phi`), capabilities and a credential *handle* (a name,
  never a secret); suspend, terminal revoke, reconfigure and rollback;
- external writes as durable intents bound to the exact bytes of one Core
  artifact, with the Spec 079 effective class checked at intent and send
  time and an idempotency key over adapter, destination, key and digest;
- effect states `pending -> sent -> confirmed | failed | unknown`, with
  `sent` written before the transport call; crash recovery turns `sent`
  into `unknown`; `unknown` is never retried blindly: reconciliation asks
  the destination and confirms, re-arms or fails;
- a transport interface whose only product implementation is
  **unavailable** (nothing leaves the machine) and an in-process
  institutional store with fault injection for qualification;
- storage v19; CLI `medscale adapter act|show`.

```text
intent (pending) != sent != confirmed external effect
unknown != failed != safe to resend
```

## Explicitly not authorized

- Any network client or real institutional endpoint; real credentials or
  secret material; SSO, LIMS/ELN, EHR, HPC, BI, Posit Workbench adapters
  (later paths).
- Remote deletes or overwrites; automatic retries of `unknown`.
- Real PHI; release claims.

Recorded residuals: the storage path is proven against an in-process
store, not a real service; identity and compute institutional paths are
not in this slice; no Desktop surface.

## Completion rule

`CLOSED_CANONICAL` only after merge on a green exact head and recorded
post-main verification.
