# Field mapping: InMemoryAuthorityStore → durable schema

Baseline types: `crates/medscale-core/src/authority/store.rs`, contract objects under `crates/medscale-contracts/src/objects/`.

| Memory path | SQLite / blob | Uniqueness / FK | Transaction boundary |
|---|---|---|---|
| `objects[id] = Source(SourceRecord)` | `authority_objects` class=`source`; `sources` row; blob `digest` | PK object_id; UNIQUE(scope, digest) on sources | Ingest accept / CreateSource when vault open |
| `SourceRecord.bytes` | blob store only on disk | digest verify | with metadata commit |
| `SourceRecord.header.*` | columns + body_json | scope check on read | same |
| `objects[id] = Derived(...)` | `authority_objects` + blob | PK; content_digest_hex NOT NULL | CreateDerived / OCR-ASR when vault open |
| `objects[id] = Proposal` | `authority_objects` body_json | PK | CreateProposal |
| `objects[id] = Assertion` | `authority_objects` | PK; body refs proposal id logically | PromoteProposal (+ audit) |
| `objects[id] = Evaluation` | `authority_objects` | PK | AttachValidatorEvidence / RetrieveLexical / broker receipts |
| `objects[id] = Identity` | `authority_objects` | PK | CreateIdentityAssertion / ingest |
| `objects[id] = Merge` | `authority_objects` | PK | DecideIdentityMerge |
| `objects[id] = Projection` | `authority_objects` | PK | RebuildProjection |
| `objects[id] = Audit` (incl. intents) | `authority_objects` | PK; effect_state updates same id | AppendAudit / CreateExternalActionIntent / TransitionEffect / promote |
| `next_seq` | `store_state['next_seq']` | single row | every alloc+insert |
| Outbox list | derived FROM audit class rows | — | read-only |
| Leases / allowlist / packs | not persisted in 016 | — | process lifetime |

## Facade capability persistence matrix

| Capability | Persist on success when vault open |
|---|---|
| CreateSourceRecord | yes (envelope + blob) |
| CreateDerivedArtifact | yes |
| IngestFhirSynthetic accept | yes (already partial; extend objects) |
| DocumentIntake admit | yes |
| OcrStub / AsrStub | yes |
| CreateProposal | yes |
| PromoteProposal | yes (assertion+audit atomic) |
| CreateIdentityAssertion / DecideIdentityMerge | yes |
| AttachValidatorEvidence / RetrieveLexical | yes (evaluation) |
| AppendAudit / CreateExternalActionIntent / TransitionEffect | yes |
| RebuildProjection | yes |
| NetworkBrokerInvoke audits/evals | yes |
| PacksInstallLocal audit | yes |
| Open/Close vault | load/save store; take/release writer lock |
| Backup/Restore | include authority_objects + store_state |

When **no vault is open**, memory-only behavior may remain for unit tests that never opened a vault; restart qualification **requires** an open vault root.
