# Security challenge — Spec 093 Research OS Desktop parity (read-only slice)

| # | Attack | Control | Evidence |
|---|---|---|---|
| S01 | Desktop bypassing Core to read storage, keys or transports | The view-model imports only `medscale_contracts` types and `CliSession`; every plane is read through `CoreFacade::dispatch` with its capability | code structure (`research_os_workspace.rs`); desktop tests |
| S02 | An unreadable plane shown as "nothing here" | Errors become an explicit `unavailable` row with a cause; only an `Ok` empty list becomes `empty` | desktop `every_plane_is_listed_and_empty_is_not_unavailable`, `errors_map_to_explicit_states` |
| S03 | Cross-project leakage in the view | Each plane is queried with the active project id; Core filters by project, realm and scope | Core per-plane project isolation tests (085-091); `adapter_list` filter |
| S04 | Desktop triggering effects | The route has one action, `research-os-refresh`, which only reads | code structure (`main.rs` handler) |
| S05 | Secrets on screen | Rows show ids, states and bounded details; federation identity secrets and publisher keys are never read by these calls | code structure |

Not controlled / recorded: rendered-UI and assistive-technology checks stay
under `FINAL_V0_UI_ACCESSIBILITY_QUALIFICATION`.
