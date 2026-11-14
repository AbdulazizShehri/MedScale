# Security — Spec 080 Governed Browse

| # | Threat | Control | Proof |
|---|---|---|---|
| B1 | SSRF to loopback/private/metadata | IP-literal and numeric hosts refused; public-only resolver at connect time (IPv4, IPv6, mapped, compatible, NAT64) | network unit tests; Core `transport_outcomes_and_response_limits_end_in_explicit_states` |
| B2 | DNS rebinding | the resolver answer that is filtered is the one used to connect | `PublicOnlyResolver`; unit test of the filter |
| B3 | Redirect abuse | the transport never follows redirects; each hop re-validated; limit 5 | Core `every_redirect_hop_is_re_evaluated` |
| B4 | Scheme/port/userinfo tricks | https and 443 only; `@` refused | network `validate_url_accepts_only_https_dns_hosts_on_443`; Core `policy_denials_happen_before_any_request_and_are_persisted` |
| B5 | Unapproved destination | per-Project allowlist; empty allowlist denies | Core `policy_denials_...` (zero transport calls) |
| B6 | Sensitive data in requests | query scan with Spec 079 recognizers; Privacy Gate for Project context | Core `sensitive_request_text_and_project_context_are_gated_by_the_privacy_gate` |
| B7 | Prompt injection via page | excerpt is inert text; flagged; no code path acts on content | Core `an_allowed_fetch_yields_inert_evidence_and_a_receipt_that_survive_reopen`; Core unit `excerpts_are_inert_and_flag_instruction_like_text` |
| B8 | Credential leakage | no credential, cookie, proxy or ambient header; login walls go to human takeover | Core `login_walls_request_human_takeover_and_never_use_credentials` |
| B9 | Malicious downloads | only allowed types; PDF/CSV quarantined as candidates | Core `transport_outcomes_...` |
| B10 | Oversized/slow responses | size cap and timeout with explicit states | Core `transport_outcomes_...` |
| B11 | Tampered stored content | digest re-checked on every read and restore | storage `browse_080` tests |
| B12 | Cross-scope reads | every read scope-checked | Core `an_allowed_fetch_...` |

Non-capabilities: no POST/forms/purchases, no browser engine, no search
provider, no credential store, no private or institutional destinations.
Recorded limitation: URL path segments are not scanned for sensitive spans
(research sites use identifier-shaped article ids); URL query strings and
search queries are.
