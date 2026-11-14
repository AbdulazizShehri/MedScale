# DESKTOP_QUALIFICATION — Spec 076

## Surface

Native Slint `Collaboration` route (`crates/medscale-desktop/ui/app.slint`):
Rooms list + create, Threads list (with live `ReferenceResolution` label) +
open-by-artifact-id, Messages list + post, Tasks list + create + mark-done.
Reuses the existing `desktop-projects` `CliSession` (Rooms are Project-scoped,
so no second vault/session). Light/dark through the shared `Theme` tokens;
keyboard/focus through shared components (`HonestyPanel`, `AdaptiveLineEdit`,
`ToolbarAction`, `QuickAction`, `StatusPill`); no color-only truth states
(`StatusPill`/resolution labels carry text).

Scope note: Notes and Approvals have no Desktop surface in this closure --
CLI-only for those two, the same honest-gap discipline as Spec 075's
saved-views closure (only the Grid view was built; other view kinds were
recorded as a gap, not silently claimed).

## Authority

All state arrives through `collaboration_workspace.rs` view-models backed
exclusively by `CliSession` (Core); no direct storage mutation from Slint/UI.
Covered by an in-module session test mirroring Spec 075's own precedent:
`collab_workspace_flows_through_real_core_session` exercises every function
in the module (`create_room`, `refresh_rooms`, `open_thread`,
`refresh_threads`, `post_message`, `refresh_messages`, `create_task`,
`refresh_tasks`, `complete_task`) against a real `CliSession`/`CoreFacade`,
including the honest fail-closed resolution path (an unregistered artifact
id resolves `missing`, never a fabricated `current`).

### Real bug found and fixed by this test (not hidden)

The first run of this test (CI run `35629013553`, before the fix below)
**failed**: `create room: Unauthorized`. Root cause: `ensure_self_participant`
registered the operator's `ParticipantIdentity` under the hardcoded string
`"desktop-operator"`, but every Core authority check resolves the caller via
`Collab::caller_participant`, which looks up the participant by the
*session's actual bound holder id* (`"cli-holder"` for
`CliSession::connect("desktop-projects")`, the real session the production
Desktop code shares). The registered participant and the resolved caller
were different identities, so `caller_participant()` returned `Unauthorized`
on every subsequent room/thread/message/task action -- meaning the real
"Create Room" button, if clicked in a running Desktop build, would also have
failed this way. `ebec119`'s exact-head CI never caught this because it only
compiled/type-checked/clippy'd the panel; no test exercised the runtime path
until this one did.

Fix: added `CliSession::holder_id() -> OpaqueId` (a public accessor for the
session's real bound actor id) and changed `ensure_self_participant` to
register under `session.holder_id()` instead of the hardcoded string
(`crates/medscale-core/src/cli_session.rs`,
`crates/medscale-desktop/src/collaboration_workspace.rs`). Re-run required
before this qualification's `RESULT` line below is trusted.

## Compile proof

Unlike Spec 075's Desktop change, 076 modified `app.slint` itself (the
highest-risk commit of this spec, `ebec119`, by its own commit message,
since this workstation has zero local Slint compiler feedback -- only a
manual brace-balance check via a Python script was possible before pushing).
CI run `35624477017` on exact head `ebec119` finished green on all 6
required jobs, **including `rust (windows-latest)`** -- the native Slint
compile target -- with no follow-up fix needed. This is stronger evidence
than Spec 075 had available for its own (non-Slint-touching) Desktop change.

## Renders

No rendered PNG evidence exists for this spec: local rendering is
unavailable on this workstation (no display/link toolchain), and
`.github/workflows/ci.yml` has no rendering/screenshot step either, so no
path used to qualify this spec could have produced one. This is the same
honest, recorded residual Spec 075 documented for its own Desktop change,
not a fabricated PASS -- and here it is offset by strictly stronger evidence
than 075 had: an exact-head CI compile of the actual modified `.slint` file
across all three OS targets (Spec 075 touched no `.slint` file at all, so
its own residual note covers a smaller risk surface than this one does).

```text
RESULT = PASS: fix commit bf9907c re-ran the full workspace test suite
(including the in-module collab_workspace_flows_through_real_core_session
test that had caught the holder-id bug) on CI run 35630430836, green 6/6 on
every required job, including rust (windows-latest). Slint compile proof for
the panel itself stands independently: exact-head CI run 35624477017 (head
ebec119) green 6/6 including rust (windows-latest).
RESIDUAL = no rendered PNG evidence exists for this spec (no local or CI
rendering path); Notes and Approvals have no Desktop surface (CLI-only,
documented gap, not silently claimed). Tracked for whichever future spec
adds Desktop UI-logic/rendering test infrastructure.
```
