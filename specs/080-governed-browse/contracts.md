# Contracts — Spec 080 Governed Browse

Module: `crates/medscale-contracts/src/browse.rs`. Closed vocabularies,
`deny_unknown_fields`, and no raw credential or cookie field anywhere.

```text
BrowseIntentKind   = fetch_url | search
BrowseRoute        = http_fetch | search | deterministic_browser | agentic_browser
                     (only http_fetch is available in this build)
BrowseRouteStatus  { route, available, reason }
BrowseAllowlistEntry { header, revision, project_id, host (DNS name),
                       path_prefix (starts with /), enabled }
BrowseRequest      { project_id, intent, url?, query?, context_artifact_id? }
BrowseDenyReason   = empty_allowlist | host_not_allowlisted | scheme_not_https |
  port_not_allowed | ip_literal_host | private_network_target | malformed_url |
  sensitive_span_in_request | privacy_gate_denied | route_unavailable |
  redirect_limit | redirect_target_denied | response_too_large |
  content_type_not_allowed | timeout | transport_failed
BrowsePolicyDecision { outcome: allow | deny, reason?, privacy_decision_id? }
BrowseSessionState = completed | denied | failed | awaiting_human_takeover | cancelled
                     (only awaiting_human_takeover -> cancelled is a transition)
BrowseNavigationStep { seq, url, http_status?, redirect_to?, decision }
HumanTakeoverRequest { url, reason: login_required | access_forbidden }
CredentialHandleRef  { handle_id, origin }   // opaque; never resolved here
BrowseSession      { header, revision, project_id, request, route, state,
                     decision, steps, takeover? }
BrowseEvidenceItem { header, session_id, final_url, content_type, byte_length,
                     content_digest, excerpt (<= 4000 chars, inert),
                     instruction_like_content_flagged }
BrowseDownloadCandidate { header, session_id, final_url, content_type,
                     byte_length, content_digest, status: quarantined }
BrowseReceipt      { header, session_id, project_id, request_digest, route,
                     final_state, step_count, evidence_ids, download_ids,
                     limitations: content_unverified | no_script_execution }
BrowseSessionView  { session, receipt, evidence, downloads }
```

Bounds: `MAX_REDIRECTS = 5`, `MAX_RESPONSE_BYTES = 2_097_152`,
`EXCERPT_MAX_CHARS = 4_000`, `REQUEST_TIMEOUT_MS = 15_000`,
`QUERY_MAX_CHARS = 512`, `URL_MAX_CHARS = 2_048`.
Evidence content types: `text/html`, `text/plain`, `application/json`.
Download content types: `application/pdf`, `text/csv`.

Network seam (`crates/medscale-network/src/browse.rs`): `validate_url`,
`resolve_redirect`, `is_forbidden_ip`, `BrowseTransport` (single GET of a
validated URL, no redirect following), `UreqBrowseTransport` with
`PublicOnlyResolver`, and the socket-free `ScriptedBrowseTransport`.
