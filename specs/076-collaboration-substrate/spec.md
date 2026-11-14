# Spec 076 — Collaboration Substrate

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promoted:** 2026-09-21
**Base SHA:** `6021ff9aad397a8488087cae01e56528370e8211`
**Target branch:** `spec/076-collaboration-substrate`
**Dependency:** canonical MedScale through Spec 075 (`CLOSED_CANONICAL`)
**Promotion authority:** `docs/planning/SPEC_076_PROMOTION.md`

## 1. Problem

MedScale can organize research work into Projects (Spec 074) and govern data acquisition into immutable snapshots (Spec 075), but every future consumer that needs people to discuss, review, approve, or track work against those objects — clinical review, research teams, systematic-review screening, dataset annotation, artifact review, MedAgent participation, later Hub sync — would otherwise invent its own comments/tasks/approvals shape. That produces duplicated identity concepts, inconsistent conflict handling, and, worst case, a second authority plane that competes with Core for what is "true."

## 2. Goal

Add a local-first **Collaboration Substrate**: one MedScale-owned model for rooms, participant identities (human/service/agent), threads anchored to exact artifact revisions, append-only messages, tasks, conflict-safe collaborative notes, review/approval requests and decisions, ephemeral presence, and a tamper-evident local activity trail — all built by reusing Spec 074's Project/artifact/revision primitives rather than inventing new ones.

At closure, a user can open a Project, create a Room, invite participants (including an agent participant), open a thread anchored to a specific artifact revision, post and edit messages, create and update a task, write a note and produce a genuine offline conflict copy, request and record an approval decision, watch the tamper-evident activity trail, close/reopen MedScale, and see every fact resolve identically — with zero network dependency and zero implicit effect on any canonical clinical/research object.

## 3. User and operational scenarios

1. A researcher creates a Room scoped to a Project and adds two teammates plus one agent participant; the agent's messages/decisions are visibly distinguishable from human ones everywhere they appear.
2. A reviewer opens a thread anchored to an exact `DataSnapshot` revision (Spec 075) and leaves a comment; the artifact is later refreshed to a new snapshot; the thread now reports `Stale` on next read, without any write having touched the thread.
3. A participant posts a message, edits it, then deletes it; the original body, the edit, and the deletion are all preserved in the append log; the UI shows only the current (edited/deleted) state by default.
4. A participant creates a task anchored to a document artifact and assigns a teammate; the teammate updates its status; a concurrent stale update from a second device (or a second local process) is rejected with `Conflict` and no data is lost.
5. Two participants edit the same collaborative note while one was offline; on reconnect, the second participant's edit becomes a conflict copy that preserves their content in full rather than silently overwriting or auto-merging.
6. A room owner opens an `ApprovalRequest{kind: Review}` with two assignees (dual independent review) anchored to one artifact; each assignee records one `ApprovalDecision` without seeing the other's rationale until the request closes (`blind_until_closed`).
7. Recording an approval decision never promotes a proposal, transitions an effect, or creates an external action intent — those remain separate, explicitly authorized Core operations.
8. A participant is removed from a Room; their prior messages/decisions remain attributed to them, but they can no longer read or write in that Room, effective immediately.
9. The application restarts; every room/thread/message/task/note/approval/activity record resolves identically, and the activity hash chain verifies from the beginning.
10. A vault created before Spec 076 opens and migrates (v4 -> v5) without changing old object IDs or breaking old CLI/Desktop workflows.
11. With network egress disabled, every 076 workflow remains fully useful; there is no code path that requires network for any local-first collaboration operation.
12. An interrupted/crashed mutation never leaves a collaboration row visible without its `ActivityRecord` entry, or vice versa.

## 4. Required outcomes

1. Bounded contracts for `ParticipantIdentity`/`ParticipantKind`/`AgentParticipantIdentity`, `Room`/`RoomMembership`, `AnchorTarget`/`AnchorDetail`, `ThreadRef`, `Message`/`MessageEdit`, `Task`, `NoteDocument`/`NoteRevision`, `ApprovalRequest`/`ApprovalDecision`, `PresenceEvent` (ephemeral only), `ActivityRecord`, and the `CollabEventEnvelope`/`CollabEventKind` normalization concept.
2. Reuse of existing `OpaqueId`, `ObjectHeader`, `DigestSha256`, realm/scope, `ProjectRevision`/`check_revision`/`initial_revision`, `ArtifactDescriptor`/`ArtifactVersionBinding`/`ReferenceResolution`, and `TextSpan`; no new ID, revision, or provenance foundation.
3. Encrypted-vault persistence for all durable 076 rows inside the existing storage architecture (schema v4 -> v5, additive).
4. Room/participant/membership vertical slice: create Room in a Project, register participants (human/service/agent), manage membership, reopen and see identical state.
5. Thread vertical slice: open a thread anchored to an exact `ArtifactDescriptor`, resolve `ReferenceResolution` live at read time, resolve/reopen status transitions.
6. Message vertical slice: post, edit, delete, all append-only with full history preserved.
7. Task vertical slice: create, assign, update with `expected_revision` conflict semantics.
8. Note vertical slice: create, fast-forward edit, and genuine conflict-copy creation on stale concurrent edit.
9. Approval vertical slice: request (with optional blind mode and multiple assignees), record independent decisions, withdraw — with a structurally enforced guarantee that no decision ever triggers a canonical/external effect.
10. Tamper-evident `ActivityRecord` hash chain serving as both audit trail and local activity feed.
11. CLI inspect/mutation paths through Core with stable machine-readable output, matching Spec 074/075 conventions.
12. Native Slint Desktop surface(s) backed exclusively by Core for at minimum a comments/review panel and a task/decision list, reusing the current design system.
13. Migration/reopen/backup-recovery evidence from representative pre-076 vault fixtures (including populated 074 and 075 vaults).
14. Exact-head closure evidence mapping every acceptance criterion to proof.

## 5. Explicit non-goals

Spec 076 MUST NOT implement:

- MedScale Hub, any network sync execution, or any product runtime network path beyond what already exists (Spec 084);
- MedAgent Workbench, agent run execution, or tool invocation (Spec 077) — `AgentParticipantIdentity.agent_profile_ref` is an opaque, unresolved forward reference only;
- Privacy Gate expansion, de-identification, or data-class policy beyond reusing existing classification (Spec 079);
- Clinical Graph or Knowledge/Research Canvas (Spec 083);
- Model Fleet + Compare (Spec 078);
- Governed Browse, AudioFlow, Compute, R Workspace, Community Extensions, Research/Evidence Packs, Institutional Adapters, Federation (Specs 080/081/085/086/087/089/090/091);
- CRDT-based merge of canonical clinical/research facts, or of anything beyond the one explicit `NoteDocument` conflict-copy mechanism defined in `contracts.md`;
- any mechanism by which an `ApprovalDecision` automatically performs a clinical, research-authority, or external effect;
- a second database, vault, ID namespace, revision model, or authority plane;
- full systematic-review/dual-screening/adjudication product workflow (076 proves the primitives support it without redesign; it does not build the product surface);
- real PHI fixtures or any real-PHI authorization claim;
- MESC coupling of any kind;
- wholesale adoption of Buzz's Tauri/React UI, relay protocol, or Nostr-as-storage; Buzz patterns may be selectively studied for shape only, never imported as running code or as canonical storage.

## 6. Compatibility and reliance

- Pre-076 CLI/Desktop behavior is preserved; pre-076 vaults migrate without identity rewrite.
- Spec 074 Project/artifact-ref APIs are reused for Room scoping and anchor resolution; 076 adds no parallel project or artifact system.
- Spec 075 `DataSnapshot`/`ArtifactDescriptor` bindings are reusable anchor targets with zero new pinning mechanism.
- Existing `SessionRegistry`/Core `Capability` mechanism governs all authority; `RoomMembership` adds a collaboration-visibility filter on top and never substitutes for it.

## 7. Missing-domain answers (frozen at T076-01)

- **Identity:** a `ParticipantIdentity` has no patient-identity semantics; it is a collaboration actor record bound to an existing session/audit `holder_id`. No cross-participant entity merge is performed or implied.
- **Time:** 076 durable rows use monotonic `seq`/`revision` for ordering, not wall-clock time or `MedicalTime`; `MedicalTime` keeps its clinical/time-precision semantics and is not reused for repository metadata merely because it exists.
- **Rights:** collaboration content inherits the classification/policy of the room's owning Project; 076 introduces no new rights/license model.
- **Privacy:** no new egress is introduced; message/note/task/approval bodies never leave the local vault in 076; logging conventions bound free-text exposure (`security.md` T11).
- **Provenance:** every mutation binds exact actor identity, actor kind, target object, event kind, and commits with its `ActivityRecord` append in one transaction (`migration.md` section 5).
- **Failure:** denied, unavailable, stale, missing, corrupt, unsupported-kind, and conflict states are distinct typed states end to end, reusing `AuthorityError` and `ReferenceResolution` rather than inventing a parallel taxonomy.
- **Recovery:** restart, migration, backup, rollback via pre-migration backup restore, interrupted-operation fail-closed, and activity hash-chain re-verification rules are defined in `migration.md` and tested.

## 8. Acceptance criteria

The frozen acceptance requirements are listed in `docs/planning/SPEC_076_PROMOTION.md` ("Frozen acceptance requirements") and mapped to proof in `plan.md` T076-J and the `evidence/076-collaboration-substrate/` packet. They are not restated here to avoid divergence; the promotion document is authoritative for acceptance wording.
