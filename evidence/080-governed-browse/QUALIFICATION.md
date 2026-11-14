# Qualification — Spec 080 Governed Browse

Code head `5daec97` (hardening after the qualification challenge; see
`SECURITY_ADVERSARIAL.md` C1-C6). The authoritative run for this head is
recorded in `EXACT_HEAD_QUALIFICATION.md`; the previous code head `8691277`
passed run `35828932513` and is historical. This file covers CONTRACT,
NETWORK_SSRF, STORAGE_MIGRATION_RECOVERY, CORE_AUTHORITY, CLI and DESKTOP
qualification in one place.

## Contracts (`crates/medscale-contracts/src/browse.rs`, 8 tests)

`vocabularies_round_trip_and_are_closed`, `only_http_fetch_is_available`,
`dns_host_names_exclude_ip_literals_and_junk`,
`allowlist_entries_validate_and_match` (now also whole-segment matching and
refused dot-segment/query/backslash prefixes),
`url_paths_with_normalization_ambiguity_are_refused` (new),
`requests_validate_by_intent`,
`decisions_and_sessions_hold_their_invariants`,
`media_type_strips_parameters`, plus envelope coverage through every Core,
CLI and Desktop test.

## Network / SSRF (`crates/medscale-network/src/browse.rs`, 4 tests; no sockets)

| Requirement | Test |
|---|---|
| https + 443 + DNS host; refuse http/ftp/file/scheme-relative, other ports (incl. empty and `0443`), IPv4/IPv6 literals, metadata IP, decimal/hex/octal/short numeric hosts, percent-encoded and non-ASCII hosts, backslash authorities, userinfo (incl. `host:443@evil`), `localhost`/`localhost.`, spaces, and dot-segment/encoded-slash paths | `validate_url_accepts_only_https_dns_hosts_on_443` |
| Forbidden addresses: loopback, RFC 1918, link-local incl. `169.254.169.254`, CGNAT, 0/8, multicast, broadcast, documentation, benchmarking, reserved, `192.0.0.0/24`, IPv6 loopback/unspecified/ULA/link-local/site-local/multicast/documentation, IPv4-mapped (incl. metadata and CGNAT), IPv4-compatible, NAT64 and local-use NAT64, 6to4 embedding a forbidden IPv4, Teredo; public addresses (incl. public 6to4) allowed | `forbidden_addresses_cover_private_and_special_ranges` |
| Redirect resolution: absolute paths and absolute https only; scheme-relative and downgrade targets fail validation | `redirects_resolve_only_absolute_https_and_paths` |
| Scripted transport answers only scripted URLs and truncates to limit + 1 | `scripted_transport_answers_only_scripted_urls` |

`PublicOnlyResolver` filters the default resolver's answer with
`is_forbidden_ip` and returns `HostNotFound` when nothing public remains; the
same answer is used to connect. It records whether the name resolved to
forbidden addresses only, so Core reports `private_network_target` for that
case and `transport_failed` for a name that does not resolve. The live `UreqBrowseTransport` compiles in CI
on all three platforms but is not exercised against the real internet
(recorded residual).

## Storage v9 (`crates/medscale-storage/tests/browse_080.rs`, 10 tests)

`restore_rejects_hand_edited_080_snapshots` now covers 8 tamper cases:
changed evidence bytes, duplicate session, missing receipt, IP-literal
allowlist host, dot-segment allowlist prefix, allowlist naming a missing
Project, session moved to another scope, and swapped receipt request digest.


`migration_v8_to_v9_is_additive`,
`crash_mid_v9_migration_fails_closed_and_backup_recovers`,
`session_commits_are_atomic_and_content_checked`,
`stored_content_that_no_longer_matches_its_digest_fails_closed`,
`allowlist_is_unique_and_disable_is_revision_safe`,
`only_takeover_sessions_can_be_cancelled`,
`consistency_check_detects_invariant_breaks`,
`backup_restore_roundtrips_every_080_row_exactly`,
`restore_rejects_hand_edited_080_snapshots`,
`pre_080_v8_backup_restores_with_empty_browse_tables`.
All Spec 074-079 storage suites pass with `CURRENT_META_SCHEMA_VERSION = 9`.

## Core (`crates/medscale-core/tests/browse_080.rs`, 7 tests; unit tests 3)

| Behavior | Test |
|---|---|
| Empty allowlist, http, other port, IP literals, metadata IP, userinfo, wrong host, wrong path, disabled entry: all denied with zero transport calls; 9 sessions persisted | `policy_denials_happen_before_any_request_and_are_persisted` |
| Allowed fetch: completed, digest-bound evidence, inert excerpt (script removed), instruction-like text flagged, receipt; identical after reopen; other scope refused | `an_allowed_fetch_yields_inert_evidence_and_a_receipt_that_survive_reopen` |
| Same-host redirect followed after re-check; redirects to another host, to http, to the metadata IP, and to an encoded dot-segment path denied without requesting the target; dot-segment first-hop URLs denied with zero transport calls; loop stops at 5 redirects (6 steps) | `every_redirect_hop_is_re_evaluated` |
| Private DNS answer, timeout, transport failure, oversize, disallowed type, 404: explicit states; PDF quarantined | `transport_outcomes_and_response_limits_end_in_explicit_states` |
| 401 -> awaiting human takeover, one request only; cancel; second cancel conflicts | `login_walls_request_human_takeover_and_never_use_credentials` |
| Patient name in URL query denied; search over an unclassified artifact denied by the Privacy Gate; after `public` classification, denied only as route unavailable; privacy decision ids recorded; routes report only `http_fetch` available | `sensitive_request_text_and_project_context_are_gated_by_the_privacy_gate` |
| Malformed request and IP-literal allowlist entry refused with no session | `malformed_requests_are_refused_without_a_session` |
| Unit: inert excerpts, encoded sensitive text, percent decoding | `authority::browse::tests::*` |

## CLI (`crates/medscale-cli/src/browse.rs`)

`browse_commands_run_through_core_across_fresh_sessions`: denied fetch
(empty allowlist), private-IP allowlist refused, allow add, fixture fetch,
search reported unavailable, read commands in human and JSON, session order
and states verified, show, cancel of a completed session refused.

## Desktop (`crates/medscale-desktop/src/browse_workspace.rs`)

`browse_view_models_flow_through_a_real_core_session`: denied fetch, private
allowlist entry refused, allow, fixture fetch with flagged excerpt, overview
newest-first with route availability, disable, fetch denied again, stale
disable reported as a conflict. The Slint route compiled on all three CI
platforms. No rendered screenshot (same residual as Specs 075-079).

## Local supporting runs (not authoritative)

Windows 11 workstation, WSL Ubuntu, rustc/cargo 1.97.1, final code tree:
`cargo fmt --all -- --check` pass; `check-dependency-direction.ps1` pass;
`cargo clippy --workspace --all-targets --locked -- -D warnings` pass;
contracts `browse` 8/8 and network `browse` 4/4 pass. The full local
workspace test run is **NOT_RUN / invalid**: the WSL virtual disk returned
I/O errors mid-run and the distro then failed to start. GitHub Actions is the
qualification path.
