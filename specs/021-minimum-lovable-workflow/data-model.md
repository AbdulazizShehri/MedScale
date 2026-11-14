# Data model: Spec 021 Minimum Lovable Workflow

## New / extended types (contracts)

| Type | Role |
|---|---|
| `WorkflowDoctorStatus` | Doctor honesty: `workflow_ready_base` vs `release_ready=false` |
| `DisclosureRecord` | Local append-only export/share disclosure (synthetic) |
| `ImportPreview` | Pre-promote view: source + proposal + loss-aware export |
| `JourneyStep` / `JourneyStepResult` / `JourneyReport` | Named journey accounting (non-authoritative) |
| `CliJsonError` | Stable CLI JSON error envelope |

## Persistence

- Disclosures persist as durable `ActionAuditRecord` rows with `action=disclosure.append` and structured `detail` = `DisclosureRecord` JSON (Spec 016 authority_objects path).
- Rejects persist as audits with `action=proposal.reject` without creating a `ClinicalAssertion`.
- No new SQLite table or object-class migration in READY_BASE.

## Reused durable graph

SourceRecord, Proposal, ClinicalAssertion, projections, vault blobs/meta, BackupManifest — unchanged from Specs 003–020.
