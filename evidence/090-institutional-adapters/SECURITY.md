# Security challenge — Spec 090 Institutional Adapters

| # | Attack | Control | Evidence |
|---|---|---|---|
| S01 | PHI leaving through an adapter | Ceilings can never be `local_phi`; unclassified data is `local_phi`; class checked at intent and again at send; rows and backups refuse `local_phi` intents | core policy test; contract `local_phi_never_leaves`; storage and backup tamper cases |
| S02 | Secrets in configuration or backups | Only `cred:<name>` handles are accepted; a secret-looking value is refused, including on restore | contract and backup tamper tests |
| S03 | Duplicate external effects | Idempotency key bound to adapter, destination, key and payload digest; confirmed intents never resend; the store never stores different bytes for a key | core happy-path and unknown tests |
| S04 | Blind retry of an uncertain write | `unknown` refuses send and retry; only reconciliation (head by key and digest) moves it | core unknown and crash tests |
| S05 | Crash between send and answer | `sent` is durable before the call and recovers as `unknown`; no resend happens without reconciliation | core `a_crash_after_sent_recovers_as_unknown_never_as_a_resend` |
| S06 | Payload swapped after intent | Send re-reads the artifact and refuses when its digest differs | code path in `adapter_send` (`PayloadMissing` refusal) |
| S07 | Overwriting different remote bytes | Reconciliation that finds different bytes fails the intent; nothing is overwritten | code path (`PresentDifferent -> failed`) |
| S08 | Revoked or suspended adapter still sending | State checked at intent and send; revocation is terminal | core revocation test |
| S09 | Path traversal in object keys | Relative plain keys only | contract key test; core bad-key cases |
| S10 | Hidden network egress | The product transport is unavailable; no network client exists | core `the_product_transport_sends_nothing` |
| S11 | Tampered rows or backups | Column-body checks; intent validation; consistency after restore | storage tamper tests |

Not controlled / recorded: a real institutional endpoint's own behavior
(consistency, retention, access control) is outside MedScale and untested.
