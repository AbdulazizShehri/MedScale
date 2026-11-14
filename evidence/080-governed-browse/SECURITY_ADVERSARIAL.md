# Security and Adversarial Qualification — Spec 080

Threats B1-B12 from `specs/080-governed-browse/security.md`, plus the
challenges added during T080-06 qualification (rows C1-C6). Every proving
test is bound to the code-head CI run recorded in `EXACT_HEAD_QUALIFICATION.md`
(see `QUALIFICATION.md` for what each test asserts).

| # | Threat | Result |
|---|---|---|
| B1 | SSRF to loopback/private/metadata | pass (network `validate_url_...`, `forbidden_addresses_...`; Core `policy_denials_...`, `transport_outcomes_...`) |
| B2 | DNS rebinding | structural: `PublicOnlyResolver` filters the answer that is used to connect; filter unit-tested |
| B3 | Redirect abuse | pass (`every_redirect_hop_is_re_evaluated`) |
| B4 | Scheme/port/userinfo/numeric-host tricks | pass |
| B5 | Unapproved destination | pass, zero transport calls |
| B6 | Sensitive data in requests / Project context | pass (`sensitive_request_text_and_project_context_are_gated_by_the_privacy_gate`) |
| B7 | Prompt injection via page | pass: flagged, excerpt inert; no code path reads evidence text as instructions |
| B8 | Credential leakage | pass: no credential, cookie, proxy or header support exists; login wall -> takeover |
| B9 | Malicious downloads | pass: disallowed types denied; PDF quarantined |
| B10 | Oversized/slow responses | pass |
| B11 | Tampered stored content | pass (storage digest and tamper tests) |
| B12 | Cross-scope reads | pass |

## Challenges added during qualification (2026-09-23)

| # | Challenge | Finding | Result |
|---|---|---|---|
| C1 | Allowlist bypass by path normalization (`/docs/../admin`, `%2e%2e`, `%252e%252e`, `..;/`, `..\`, `%2f`, `%5c`) | **Defect**: the prefix check ran on the raw path, and the server would normalize it outside the prefix. Fixed in `5daec97`: `validate_url` refuses ambiguous paths (first hop and every redirect hop); allowlist prefixes may not contain them | pass (contracts `url_paths_with_normalization_ambiguity_are_refused`, network `validate_url_...`, Core `every_redirect_hop_is_re_evaluated` with zero transport calls for refused paths) |
| C2 | Prefix confusion (`/docs` covering `/docs-private`) | **Defect**: plain `starts_with`. Fixed: whole-segment matching | pass (contracts `allowlist_entries_validate_and_match`) |
| C3 | Alternate address forms: octal (`0177.0.0.1`), percent-encoded host, backslash authority, `host:443@evil`, empty/zero-padded port, non-ASCII host, `https:/`, uppercase scheme, `localhost.` | none; all refused before any request | pass (network `validate_url_...`) |
| C4 | IPv6 embeddings of forbidden IPv4: 6to4 (`2002:7f00:1::`, `2002:a9fe:a9fe::`), Teredo, local-use NAT64, site-local, mapped metadata/CGNAT | **Gap**: 6to4, Teredo and `64:ff9b:1::/48` were not refused. Fixed | pass (network `forbidden_addresses_...`) |
| C5 | Honest outcome for a name that does not resolve | **Defect**: NXDOMAIN was reported as `private_network_target`. Fixed: only an answer made entirely of forbidden addresses is a private target; NXDOMAIN is `transport_failed` | structural (live resolver path; not exercised in CI) |
| C6 | Restore re-introducing rows a normal write rejects | **Gap**: restore accepted allowlist entries/sessions naming a missing Project or another scope, and receipts whose route/request digest disagreed with their session. Fixed in `verify_browse_consistency` | pass (storage `restore_rejects_hand_edited_080_snapshots`, 8 tamper cases) |

Also checked with no defect: proxy inheritance (`proxy(None)`, so environment
proxies are ignored); ambient cookies (no `cookies` feature, and a fresh
agent per request); custom headers (only a fixed User-Agent); auto-follow
(`max_redirects(0)`); credential use on 401/403 (takeover, one request);
body exhaustion (`limit + 1` read, global timeout); a single live
transport construction site (`facade.rs`), with CLI and Desktop reaching it
only through `CliSession`.

Honest limits: the live public-internet path is not exercised in CI; URL
path segments are not scanned for sensitive spans; instruction-like content
detection is a fixed phrase list and flags only (it is not a safety
classifier, and nothing downstream consumes evidence as instructions).
