# Security challenge — Spec 094 Research OS Desktop row actions

| # | Attack | Control | Evidence |
|---|---|---|---|
| S01 | Desktop bypassing Core | `act` calls only `CliSession` methods; Core applies the same capability, project, realm and scope checks as the CLI and writes the receipt | code structure; desktop adapter round-trip test (receipts) |
| S02 | Crafted UI action string running an unoffered action (for example revoke or export) | `act` accepts only the five plane/action pairs; anything else is refused before Core is asked | desktop `actions_go_through_core_and_unknown_actions_change_nothing` |
| S03 | Terminal or data-moving actions one click away | Revoke, send, export, import, publish, install and consent are not Desktop actions in this slice | desktop `only_reversible_actions_are_offered` |
| S04 | Reporting a refused action as done | A Core error or refusal becomes "not done" with the reason; only a recorded resulting state is shown as done | desktop tests; `main.rs` handler |
| S05 | Acting outside the session's authority | Extension actions carry the active project id; Compute and adapter ids are resolved by Core only within the session's realm and scope (`scoped_compute_job`, the adapter realm/scope check), with the same authority as the CLI; Desktop offers them only from the active project's rows | Core isolation tests (085, 087, 090); code structure |

Not controlled / recorded: a `running` Compute job started by another
process can be cancelled only in-process (Spec 085 residual).
