# Contracts: Spec 021

Authoritative Rust types live in `crates/medscale-contracts/src/workflow/mod.rs`.

## Envelope additions

- `Capability::{RejectProposal, AppendDisclosure, ListDisclosures}`
- Matching `RequestBody` / `ResponseBody` variants in `envelopes`

## Journey CLI

```text
medscale journey run --vault-root <dir> --fixture <fhir.json> \
  --backup-dir <dir> --restore-dir <dir> [--subject ...] [--vault-id ...] [--reject] [--json]
```

Exit codes: `0` success; `1` textual error; `2` JSON `CliJsonError` on stderr when `--json`.

## Non-claims

Completing a journey never sets `RELEASE_READY`, `PRIVATE_DATA_READY`, or full FHIR conformance.
