# Spec 087 — Community Extensions (declarative foundation)

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Base SHA:** see `docs/planning/SPEC_087_PROMOTION.md`
**Target branch:** `spec/087-community-extensions`
**Dependency:** 079 + 084 + 085 (075 for data-source reads; all closed).

## 1. Problem

Researchers want small community tools (a schema viewer, a cohort peek)
without handing third-party code their vault. Loading plugin code into the
trusted process would give it ambient authority; running it elsewhere
needs a qualified sandbox MedScale does not have yet.

## 2. Goal

A signed, declarative extension format and a Core-owned lifecycle:
- a pack is a canonical manifest plus an ed25519 signature by a publisher
  the user explicitly trusts;
- commands name operations of a closed, versioned, read-only Host API
  that Core executes; no extension code exists or runs;
- installs are per Project and start with no grants; each capability is
  granted explicitly with a data-class ceiling;
- upgrades that add capabilities need re-consent; rollback, disable,
  uninstall and revocation (publisher or release) are receipts, and
  revocation quarantines atomically.

## 3. Scenarios

1. A developer creates a key (`medscale extension keygen`), writes a
   manifest and builds a signed pack (`medscale extension pack`).
2. A researcher trusts the publisher, installs the pack in a Project, and
   is denied until they grant `snapshot_schema_read` at `local_phi`.
3. Version 1.1 adds `snapshot_rows_read`: the install is `pending_consent`
   until re-enabled; the new command then returns at most `max_rows` rows.
4. A forged, edited, unknown-capability, native-entrypoint, oversized or
   future-API pack is refused with a receipt.
5. The publisher's key is compromised: revoking it quarantines every
   install in every Project in one transaction.

## 4. Requirements

- FR-01 Closed manifest contract; canonical bytes; fail-closed parsing.
- FR-02 Verification pipeline (size, canonical form, trusted publisher,
  key match, strict signature, API range, revocation).
- FR-03 Lifecycle with compare-and-set revisions and receipts.
- FR-04 Grants per capability and Project with data-class ceilings.
- FR-05 Invocation of Host API v1 with a runtime receipt per call.
- FR-06 Storage v16, backup/restore, consistency.
- FR-07 CLI and SDK helpers over Core.

## 5. Success criteria

The closure gate of `RESEARCH_OS_V2_SPEC_IMPLEMENTATION_CONTRACTS.md` §087
for declarative extensions: a third-party sample extension is built,
packaged, installed, denied a non-granted capability, upgraded with
capability re-consent, revoked, and run without trusted-process or vault
access. Executable extensions remain a recorded gate.
