# Security — Spec 087 Community Extensions

| Threat | Control | Test |
|---|---|---|
| Malicious code in an extension | No extension code is loaded or run; entrypoint vocabulary is `declarative` only; `wasm`/`native` fail the manifest parse | contract `unknown_capabilities_and_entrypoints_fail_closed`; core `untrusted_forged_or_incompatible_packs_are_refused` |
| Unknown or invented capability | Closed capability vocabulary; unknown value fails closed | same |
| Forged or tampered pack | Strict ed25519 over a domain-separated digest of the exact canonical manifest bytes; non-canonical JSON refused | core forged / edited / non-canonical cases |
| Untrusted publisher, key substitution | Only user-trusted publisher keys; key must equal the trusted one; no built-in trust root | core `publisher_unknown`, `publisher_key_mismatch` |
| Capability escalation on update | Upgrade computes the capability expansion; any expansion sets `pending_consent`; nothing runs until re-enabled; grants are per capability | core lifecycle test |
| Downgrade attack | Upgrades must be strictly higher versions from the same publisher; rollback only to the recorded previous release, never a revoked one | core lifecycle test |
| Default authority | An install has no grants; every capability needs an explicit grant for one Project with a data-class ceiling | core lifecycle test (`capability_not_granted`, `data_class_above_ceiling`) |
| Cross-project data access | Targets must be the Project's; installs and grants are per Project | core `revocation_quarantines_and_nothing_crosses_projects` |
| Revoked publisher / release keeps running | Revocation quarantines every affected install in one transaction; quarantined installs cannot be enabled or reinstalled | same |
| Oversized pack / result | 256 KiB pack bound; row pages bounded by `max_rows` (<= 1000) | core oversized case; rows case |
| Tampered storage / backups | Column-body checks, canonical release bytes, consistency after restore | storage tamper tests |
| Vault, key, filesystem, network, device access | Host API v1 has three read-only operations executed by Core; there is no path to any other resource | code structure (`HostOperation` is closed) |

Not controlled / recorded: licenses are declared, not verified; the
publisher's identity behind a key is the user's judgment; no registry
review tiers.
