# Security challenge — Spec 091 Federation

| # | Attack | Control | Evidence |
|---|---|---|---|
| S01 | Raw PHI federating | Peer ceilings can never be `local_phi`; unclassified data is `local_phi`; policy checked at export and again at import | core policy test; contract `raw_phi_never_federates`; backup tamper case |
| S02 | Forged or edited bundle | Strict ed25519 over a domain-separated digest of the canonical body; non-canonical bodies refused | core forgery test (edited sequence) |
| S03 | Replay | Strictly increasing sequence per sender; the peer's last sequence advances in the import transaction | core exchange test (replay refused) |
| S04 | Misdelivery | Bundles name their recipient; others refuse them | core forgery test (wrong recipient) |
| S05 | Trust by connectivity | Peers are trusted explicitly with a key; unknown peers get nothing and are refused on import | core policy and forgery tests |
| S06 | Compromised peer | Revocation is terminal: no imports from, no exports to | core forgery test (revoked) |
| S07 | Overwrite of local data | Imports are new rows; duplicates by digest are skipped; nothing local is modified | core exchange test |
| S08 | Withdrawn data lingering | Tombstones erase bytes and keep the record | core exchange test; storage tombstone case |
| S09 | Identity key theft through backups | The identity secret is never exported; restored vaults cannot sign | storage `backups_never_carry_the_identity_secret`; `whole_platform_092` restore case |
| S10 | Partial import | Items verified before any write; one transaction for sequence, items and receipt | storage `imports_are_atomic_and_identities_are_single` |
| S11 | Hidden central authority | No network code; bundles are files moved by people | code structure |

Not controlled / recorded: out-of-band key verification between
institutions is a human process; key rotation is revoke and re-trust.
