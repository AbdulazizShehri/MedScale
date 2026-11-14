# Spec 091 — Federation (bounded bundle exchange)

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promotion date:** 2026-09-27
**Canonical base:** `ee7e1d5a77a39580b4314067e0557716cca93bfa` (Spec 090 closure merge)
**Target branch:** `spec/091-federation`

## Authority

Dependency proof: `RESEARCH_OS_V2_SPEC_IMPLEMENTATION_CONTRACTS.md`
numbers Federation **091**, hard dependency **090** (`CLOSED_CANONICAL`).
The authority lists research/implementation gates: institution identity
and trust anchors, signed export bundles, explicit sharing policy,
provenance across institutions, revocation/tombstones, federated analysis
only after threat/privacy validation, and no automatic raw-PHI
federation. This spec admits no dependency.

## Scope

Closure gate: "a bounded multi-institution synthetic scenario proves
signed exchange, provenance, revocation, policy denial, and no hidden
central authority". This slice delivers:

- an institution identity per vault (ed25519 via the admitted
  `medscale-keys`; the secret never leaves the vault and is not backed up);
- peers trusted explicitly with a data-class ceiling (never `local_phi`);
  terminal revocation; connectivity never implies trust;
- signed, sequenced bundles (domain-separated signature over the
  canonical body; strictly increasing sequence per sender, so replays are
  refused), carrying items with provenance and tombstones, moved out of
  band as files;
- policy denial at export and import (`local_phi`, over-ceiling), refusal
  of wrong-recipient, unknown or revoked peers, forged or edited bundles;
- imports as new local rows that never overwrite; duplicates skipped;
  tombstones erase bytes and keep the record;
- storage v20; CLI `medscale federation act|show`; a two-institution
  synthetic scenario test.

## Explicitly not authorized

- Federated or partial analysis across institutions; any network
  transport; a central MedScale service.
- Raw PHI federation (never); real institutions; real PHI.

Recorded residuals: the identity secret is not recoverable from a backup
(a restored vault needs a new identity that peers must trust again); key
rotation is by revoking and re-trusting; no schema negotiation beyond the
single bundle format version; no Desktop surface.

## Completion rule

`CLOSED_CANONICAL` only after merge on a green exact head and recorded
post-main verification.
