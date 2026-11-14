# v0 UI Import Boundary

The founder uses v0 as the visual/UI generation source.

Place or sync v0-produced UI changes through an isolated branch/import flow. Before integration, record:

- v0 project/chat/version identifier if available;
- Git commit/branch or exported artifact identity;
- files introduced/changed;
- framework/package versions;
- synthetic fixture/data assumptions;
- any generated server/API/database/network code.

Generated server/API/database/network code is **not automatically admitted**. Cursor extracts/adapts the visual/component behavior to the MedScale client boundary and wires it to typed Rust-owned contracts.

Do not put PHI into v0. Do not give v0 direct canonical DB, keys, provider credentials, unrestricted local filesystem, or MedScale authority.

If the visual artifact is not yet available, Cursor continues all nonvisual and integration-contract work and records `FINAL_V0_UI_ARTIFACT` as an external gate only for the final visual release.

## Fixture UI contracts (pre-v0)

Typed shell adapters live in medscale_contracts::fixture_ui (Doctor/Timeline/Brief/Coverage). Final visual artifacts remain FINAL_V0_UI_ARTIFACT.
