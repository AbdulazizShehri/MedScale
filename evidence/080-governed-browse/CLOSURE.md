# CLOSURE — Spec 080 Governed Browse

## Terminal truth

```text
SPEC_080_CLOSED_CANONICAL=true
MERGE_SHA=a8e32beb42a0687a1a7fafec83b775f26f0ed194 (PR #139)
FINAL_HEAD=fe0a72eec406aabe764c48a93bab314d46d53c1c
EXACT_HEAD_CI=35867289972 (6/6)
CODE_HEAD_CI=35861492981 (6/6 on 5daec97)
POST_MERGE_MAIN_CI=35873449163 (6/6 on a8e32be)
REVIEW_POLICY=FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22 (no external reviewer;
  deterministic scope record in EXACT_RANGE_REVIEW.md)
CONTRACTS=PASS (8)  NETWORK_SSRF=PASS (4, socket-free)
STORAGE_MIGRATION_RECOVERY=PASS (10, v8->v9 additive; 8 restore tamper cases)
CORE_AUTHORITY=PASS (7 + 3 unit)  CLI=PASS (1)  DESKTOP_VIEW_MODEL=PASS (1)
SECURITY_ADVERSARIAL=PASS (B1-B12 and qualification challenges C1-C6)
LIVE_PUBLIC_INTERNET_PATH=NOT_EXERCISED (compiled on 3 platforms; recorded residual)
REAL_PHI_AUTHORIZED=false
RELEASE_READY=false
PRIVATE_DATA_READY=false
SPEC_081_IMPLEMENTATION_AUTHORIZED=false until its own promotion
```

## What this closure establishes

MedScale has one Core-owned, read-only HTTPS retrieval path. A request goes
out only to an operator-allowlisted host and whole-segment path prefix of
the Project, over `https` on port 443 to a DNS name. The path must be free
of normalization tricks. Every resolved address is re-checked at connect
time against loopback, private, link-local/metadata, CGNAT, reserved and
IPv6 embedding ranges. Redirects are never auto-followed: each hop is
re-evaluated. Sensitive request text and Project context are gated by the
Spec 079 Privacy Gate. Fetched content is digest-bound, inert evidence;
downloads stay quarantined; login walls go to human takeover and never use
credentials. Every outcome is persisted with a receipt, and survives reopen
and backup/restore. Restore refuses rows that a normal write would reject.

## Defects found and fixed during qualification

The T080-06 challenge found and fixed five real problems before merge
(`SECURITY_ADVERSARIAL.md` C1, C2, C4-C6):
- an allowlist bypass through path normalization (`/docs/../admin`,
  percent-encoded forms);
- prefix segment confusion;
- missing 6to4, Teredo and local-use NAT64 ranges;
- NXDOMAIN mislabelled as a private-network target;
- restore accepting rows that named a missing or out-of-scope Project, or
  receipts that disagreed with their session.

## Honest residuals (non-blocking, recorded)

- The live `ureq` path to the real public internet is not exercised in CI;
  hermetic tests use a scripted transport. The NXDOMAIN/private-answer
  distinction is proven structurally only.
- URL path segments are not scanned for sensitive spans (research sites use
  identifier-shaped article ids); query strings and search queries are.
- Instruction-like content detection is a fixed phrase list that flags only;
  nothing downstream consumes evidence as instructions.
- The `search`, `deterministic_browser` and `agentic_browser` routes report
  `unavailable` (no admitted search provider; browser execution needs the
  Spec 085 bounded worker).
- No rendered Desktop screenshot (no CI rendering step), as in Specs 075-079.
- The local WSL build environment failed with disk I/O errors during this
  spec. All authoritative qualification is GitHub Actions CI.
