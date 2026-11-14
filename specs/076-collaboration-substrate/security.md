# Security and Threat Delta — Spec 076

**Status:** implementation contract

Spec 076 adds local-first collaboration state (rooms, threads, messages, tasks, notes, approvals, participant identities, activity). It adds no new product runtime network path, no remote worker, no model execution authority, no Hub sync execution, and no real-PHI authorization.

## 1. Trust boundary

```text
CLI/Desktop
   -> typed Core command/query
      -> actor/session/scope checks (Core Capability, unchanged mechanism)
         -> RoomMembership visibility check (collaboration-scope, separate from Capability)
            -> input + revision validation
               -> anchor shape validation (live resolution is a read-time recompute, never a write-time cache)
                  -> encrypted storage transaction
                     -> ActivityRecord append (same transaction)
```

No CLI/Desktop component may access 076 tables directly. No collaboration mutation may call any other subsystem's mutation path (effects, actions/outbox, proposal promotion) directly or transitively.

## 2. Threats and required controls

### T1 — Approval-triggers-effect escalation

Attack: an `ApprovalDecision` (or a room "approval") is wired, now or by a future careless change, to automatically call `PromoteProposal`, `TransitionEffect`, `CreateExternalActionIntent`, or any clinical/external effect, so that a collaboration approval silently performs a canonical or external action.

Controls:

- `ApprovalDecision` recording is a pure collaboration-table write plus `ActivityRecord` append; the Core function that records it has no code path to any effect/action/proposal-promotion function, verified structurally by dependency-direction review, not just by test;
- distinct-and-visible in the type system: `ApprovalDecisionOutcome` has no variant that maps 1:1 to `EffectState`/proposal promotion; any future spec wiring collaboration approvals to a real effect must add an explicit, separately-authorized Core capability, not overload this one;
- the acceptance test suite includes a structural check that `authority::collaboration` (076's Core module) contains no call into `authority::promote`, `authority::amend`, or `contracts::actions`.

Tests: recording every `ApprovalDecisionOutcome` variant produces no side effect outside 076 tables + `ActivityRecord`; a direct unit test asserting no cross-module call exists (via `cargo` doc/dependency inspection or an explicit "effects untouched" fixture comparison before/after).

### T2 — Participant impersonation / actor-kind confusion

Attack: a caller claims `actor_kind: Human` while actually acting as an automated `Agent`, or vice versa, to gain trust or evade a policy that treats agent-authored content differently.

Controls:

- `CollabEventEnvelope.actor_kind` is never caller-supplied truth: Core looks up the live `ParticipantIdentity.kind` for `actor_participant_id` and rejects (`InvalidArgument`) any request whose implied kind does not match;
- `ParticipantKind` is immutable once a `ParticipantIdentity` is created (section 4 of `contracts.md`); there is no update path that flips `Human` to `Agent` in place;
- `holder_id` binding reuses the existing session/lease holder identity; a session cannot act as a participant it does not hold.

Tests: mismatched actor-kind claims are rejected; attempt to mutate `ParticipantIdentity.kind` in place is rejected or has no such API; agent-authored `Message`/`ApprovalDecision` is queryable and visibly distinct from human-authored.

### T3 — Cross-project / cross-room visibility leakage

Attack: a participant with access to Room A reads or infers content from Room B (different Project/authority scope) via list/summary/activity endpoints.

Controls:

- every 076 read (`RoomRead`, `ThreadRead`, `MessageRead`, `TaskRead`, `NoteRead`, `ApprovalRequestRead`, `ActivityRead`) filters by `RoomMembership` before returning any row, including counts/summaries;
- `Room.project_id` scope check reuses the existing Project realm/authority-scope check; a participant without Project-level authorization cannot see a Room in that Project regardless of `RoomMembership` state;
- denied resolution does not reveal room/thread/task existence beyond "not found"/"denied", matching the existing `ReferenceResolution::Denied` discipline.

Tests: mixed visible/denied rooms across two Projects; membership in Room A never leaks Room B counts, titles, or activity; error-string content assertions confirm no leaked identifiers.

### T4 — Stale/deleted participant or removed membership retains access

Attack: a participant removed from a Room (or revoked entirely) continues to read/write collaboration state because a cached session or a stale membership check is not re-verified per request.

Controls:

- `RoomMembership.status == Removed` and `ParticipantIdentity.status == Revoked` are checked on every request, not cached across a session's lifetime;
- revocation is immediate and does not require a new session to take effect, matching the existing `SessionRegistry` per-request validation discipline.

Tests: mutation/read attempted immediately after membership removal and after participant revocation both fail closed; a live session with valid Core capabilities still cannot act in a room it was removed from.

### T5 — Stale-write overwrite on mutable 076 state

Attack: two callers update a `Room`, `RoomMembership`, `ThreadRef`, `Task`, or `ApprovalRequest` from the same old state and the later request silently overwrites the newer result.

Controls:

- explicit `expected_revision` precondition on every mutable 076 row except `NoteDocument`;
- mismatch -> `Conflict`, no write, no last-write-wins;
- `NoteDocument` uses the conflict-copy path instead (`contracts.md` section 10) — the caller's content is preserved, never silently dropped, and never silently merged either.

Tests: stale task update, stale room rename, stale membership role change, stale approval-request update all `Conflict`; stale note update produces a conflict copy with both revisions readable.

### T6 — Task/approval privilege escalation via assignment

Attack: a participant assigns themselves (or an unauthorized participant) as approver/adjudicator to manufacture a favorable decision, or an assignee is added after decisions are already recorded to retroactively appear as a reviewer.

Controls:

- `assignee_participant_ids` on `ApprovalRequest` is set at creation and mutation is revision-guarded and audit-logged (`ActivityRecord`), so any addition/removal is visible, timestamped-by-seq, and attributable;
- an `ApprovalDecision` requires the deciding participant to currently be a `RoomMembership`-active member with `ReadOrDecide`-class capability; a non-assignee can still be blocked at the product-policy layer in a later spec, but 076's job is to make every assignment change and every decision individually auditable, not silently retroactive;
- `blind_until_closed` filtering (section 11 of `contracts.md`) prevents an assignee from copying a co-assignee's already-recorded rationale before deciding.

Tests: assignment-list mutation appends an `ActivityRecord`; decisions recorded before an assignee was added remain attributed to their original decider and are not reassignable; blind mode hides co-assignee decisions until `status != Open`.

### T7 — Anchor confusion / stale-review-shown-as-current

Attack: an artifact changes after a thread/approval was opened against it, and the UI/CLI keeps presenting the thread as reviewing the current artifact state.

Controls:

- `ReferenceResolution` is recomputed at every read against the live artifact, never cached at write time (`contracts.md` section 6);
- `Stale`/`Missing`/`Corrupt`/`UnsupportedKind`/`Denied` are distinct, surfaced states, never collapsed into a generic "ok";
- CLI/Desktop must render the resolution state alongside the thread/approval, not merely the anchor identity.

Tests: mutate the anchored artifact after opening a thread/approval, assert resolution flips from `Current` to `Stale`/`Missing` on next read without any 076 write occurring.

### T8 — Duplicate/replay event handling

Attack: a client retries a request (network hiccup on IPC, crash-recovery replay) and the same mutation is applied twice, double-posting a message or double-recording a decision.

Controls:

- `Message`, `MessageEdit`, `NoteRevision`, `ApprovalDecision`, `ActivityRecord` appends are assigned `seq` by storage inside the commit transaction, never client-supplied, so a naive retry cannot silently duplicate a `seq`;
- create-style requests for `Room`/`Task`/`ApprovalRequest`/`ParticipantIdentity` are idempotent on a caller-supplied idempotency-relevant natural key where one exists (e.g. `(room_id, holder_id)` for participant registration), returning the existing row rather than creating a duplicate.

Tests: repeated identical `ParticipantRegister` for the same `holder_id` in the same room returns the same participant, not two; repeated `MessagePost` with the same client-side request id (if the CLI/Desktop surface exposes one) does not duplicate — or, if no idempotency key is exposed, the test documents that retries at the transport layer are the caller's responsibility, and that a genuine duplicate call is treated as two distinct, equally-legitimate messages (never silently dropped in a way that hides a real second message).

### T9 — Malformed/hostile metadata injection

Attack: oversized/control-character/markup/SQL-like room names, message bodies, task titles, or note bodies cause storage/query/UI issues.

Controls:

- bounded UTF-8 validation with explicit constants on every free-text field (`Room.name`, `Message.body`, `Task.title`/`description`, `NoteRevision.body`, `ApprovalDecision.rationale`);
- parameterized storage APIs (no string-built SQL);
- UI/CLI renders text as text, never as executable markup;
- logging safely escapes/bounds user metadata; message/note bodies never appear unbounded in operational logs.

Tests: empty/whitespace, max boundary, over-max, unusual Unicode, control characters, SQL-like strings per frozen policy.

### T10 — Audit tampering / activity-log rewrite

Attack: an `ActivityRecord` is edited or deleted out of band (direct DB tampering, restore of a hand-edited backup) to hide what happened.

Controls:

- `checkpoint_digest` hash-chains every `ActivityRecord` to its predecessor (`contracts.md` section 14); verifying the chain from `seq = 1` detects any edited, deleted, or reordered row;
- `ActivityRecord` rows have no update API; storage exposes append and read only;
- migration/backup/restore evidence includes a hash-chain verification pass (`migration.md` section 14).

Tests: tamper with a stored `ActivityRecord` row directly in a test fixture and prove chain verification detects it; restore-from-backup re-verifies the chain and fails closed on mismatch.

### T11 — Content leakage into logs or cross-scope caches

Attack: message/note/task/approval body text (which may carry sensitive research/clinical discussion) leaks into operational logs, crash dumps, or a shared cache visible across authorization scopes.

Controls:

- logging conventions already required elsewhere apply identically: free-text bodies are never logged in full at info/debug level by default;
- 076 introduces no new shared cache; reads are scope-filtered per request (T3), not memoized across authorization contexts.

Tests: debug-log capture scans across every 076 mutation path assert no raw message/note/task/approval body appears unbounded in log output.

### T12 — Half-committed state after crash

Attack: a mutation's primary row commits but its `ActivityRecord` append does not (or the reverse), leaving an untraceable mutation or an orphaned audit entry.

Controls:

- every mutation transaction includes its `ActivityRecord` append in the same commit (`migration.md` section 5); there is no 076 write path that commits one without the other;
- migration journal fail-closed semantics from the existing framework are preserved for v4 -> v5.

Tests: crash-point matrix from `migration.md` section 6, including "before ActivityRecord append commits."

### T13 — Dependency and supply-chain smuggling

Attack: 076 pulls in a new dependency (e.g. for text diffing, markup rendering) that introduces copyleft/transitive risk or unreviewed behavior.

Controls:

- 076's foundation requires no new dependency: bodies are plain bounded strings, digests reuse `sha2` (already a dependency), and the append-log pattern reuses existing storage primitives;
- if a later slice genuinely needs a new dependency, exact version/license/security/exit review is recorded in the evidence packet before admission, and `cargo-deny`/supply-chain gates must pass on the exact head.

Tests: dependency-direction and supply-chain gates green; if `Cargo.toml` gains any new dependency, a review record exists.

## 3. Explicit non-capabilities

076 introduces no capability for: triggering a clinical/external effect from a collaboration approval, cross-project visibility, agent self-escalation to human trust, silent last-write-wins on any mutable row, Hub network sync execution, or free-form scripted formatting/markup execution on message/note bodies. Any test or review finding suggesting such a capability is a blocking defect, not a scope question.
