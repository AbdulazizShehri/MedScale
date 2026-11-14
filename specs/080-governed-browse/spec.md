# Spec 080 — Governed Browse

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promoted:** 2026-09-23
**Base SHA:** `561f97feabe6beb7f079e2af679a4119b50368ce`
**Target branch:** `spec/080-governed-browse`
**Dependency:** 077 + 079 + the Spec 013 Network Broker (all closed)
**Promotion authority:** `docs/planning/SPEC_080_PROMOTION.md`

## 1. Problem

Research work needs public web sources (guidelines, papers, registries).
Without a governed path, a user or agent would reach the web outside the
MedScale authority: no allowlist, no private-network protection, no privacy
check on what leaves, and no provenance for what comes back.

## 2. Goal

One Core-owned, read-only Browse path: operator-allowlisted https hosts
only, private networks refused at connect time, every redirect re-checked,
Project content gated by the Privacy Gate, and every fetch recorded as a
session with inert evidence and a receipt.

## 3. Scenarios

1. A researcher allows `pubmed.ncbi.nlm.nih.gov` for a Project and fetches
   an article page; a session, an evidence item (digest, final URL, inert
   excerpt) and a receipt are stored.
2. A fetch to a host that is not allowed, to `http://`, to an IP literal,
   or to a name that resolves to a private address is denied before or at
   connect time, and the denial is stored.
3. A page redirects to a host that is not allowed; the session stops at
   that hop with `redirect_target_denied`.
4. A page contains "ignore previous instructions"; the evidence item is
   flagged and nothing acts on it.
5. A PDF link returns a quarantined download candidate, not an artifact.
6. A login wall yields a human-takeover request; no credential is used.
7. A URL query containing a patient name is denied; a search derived from
   an unclassified artifact is denied by the Privacy Gate.

## 4. Requirements

- FR-01 Allowlist per Project (add, list, disable; DNS host names only).
- FR-02 URL validation: https, port 443, no userinfo, no IP literal.
- FR-03 Public-only resolution at connect time; no automatic redirects.
- FR-04 Per-hop re-evaluation with a redirect limit.
- FR-05 Sensitive-span scan of the URL query and search query; Privacy
  Gate decision for Project context.
- FR-06 Response limits (size, type, timeout) with explicit states.
- FR-07 Inert evidence items; quarantined download candidates.
- FR-08 Human-takeover state for login walls; no credential use.
- FR-09 Sessions, evidence, downloads and receipts persisted atomically;
  storage v9; backup/restore; consistency checks.
- FR-10 CLI and Desktop over Core; route availability reported.

## 5. Success criteria

The frozen acceptance requirements in `SPEC_080_PROMOTION.md`.
